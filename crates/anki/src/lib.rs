//! Anki decks from Steps content: one `.apkg` with a `Steps` deck, a level
//! per course and a subdeck per lecture (`Steps::Hepatology::01 Liver
//! Cirrhosis`). Shared by
//! `steps-tools anki` and the app's «export to Anki» button.
//!
//! An `.apkg` is a zip of an Anki SQLite collection (`collection.anki2`, the
//! schema-11 layout every Anki and AnkiDroid imports) and a `media` index.
//! Model, deck and note ids, field names and GUIDs match the interim
//! `/mnt/project-files/steps/anki/make_apkg.py` (genanki), so decks
//! imported from either one update in place and keep their review history.
//!
//! Note types: Steps Basic (Front, Back, Source, Update, Id) and Steps Cloze
//! (Text, Extra, Source, Update, Id). MCQ, enumerate, essay and order
//! items become Basic notes; a case becomes one note per step and a table one
//! note per blank. Tags: `hb::L01`, `topic::<slug>`, `approved` or
//! `unverified`, `draft` for unmerged lectures, and the item's own tags.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;

use rusqlite::{Connection, MAIN_DB, params};
use serde_json::{Value, json};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use steps_core::content::{Body, Item, Lecture, OutlineEntry, Status};

/// Same CSS as make_apkg.py; Anki's night mode adds `.nightMode`.
const CSS: &str = ".card{font-family:system-ui,sans-serif;font-size:18px;line-height:1.5;text-align:left;color:#1d2420;background:#fff}
.nightMode .card{color:#e6ebe7;background:#1b211e}
.src{margin-top:14px;font-size:13px;color:#6b766f}
.upd{margin-top:10px;padding:6px 8px;border-left:3px solid #c08a1e;font-size:14px}
.why{font-size:15px}.cloze{font-weight:bold;color:#2f6b4f}";

const BACK_TAIL: &str =
    r#"<div class="src">{{Source}}</div>{{#Update}}<div class="upd">Update: {{Update}}</div>{{/Update}}"#;

const LETTERS: &[u8] = b"ABCDEFGH";

/// What decides which items go in, and how they are tagged.
#[derive(Default)]
pub struct Options<'a> {
    /// Item ids Mohamed approved: tagged `approved`, like items whose file
    /// status is `approved`.
    pub approved: Option<&'a BTreeSet<String>>,
    /// Item ids he marked ✗ wrong: left out.
    pub wrong: Option<&'a BTreeSet<String>>,
    /// Items he edited (✎), already applied: used instead of the book version.
    pub edits: Option<&'a BTreeMap<String, Item>>,
    /// Lectures not merged yet: tagged `draft`.
    pub drafts: Option<&'a BTreeSet<String>>,
    /// The MSc's courses: every course gets its own level,
    /// `Steps::Basics of Research::03 …`. Without it, lectures sit straight
    /// under `Steps` with their old names (`Steps::L01 …`).
    pub program: Option<&'a steps_core::program::Program>,
}

fn has(set: Option<&BTreeSet<String>>, id: &str) -> bool {
    set.is_some_and(|s| s.contains(id))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    Basic,
    Cloze,
}

impl Model {
    pub fn id(self) -> i64 {
        match self {
            Model::Basic => stable_id("steps-basic-v1"),
            Model::Cloze => stable_id("steps-cloze-v1"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Note {
    pub model: Model,
    pub deck: i64,
    /// Front/Back/Source/Update/Id, or Text/Extra/Source/Update/Id.
    pub fields: [String; 5],
    pub tags: Vec<String>,
    pub guid: String,
}

impl Note {
    /// Card ordinals: one for Basic, one per distinct `{{cN::…}}` for Cloze.
    fn card_ords(&self) -> Vec<u32> {
        match self.model {
            Model::Basic => vec![0],
            Model::Cloze => {
                let ords: BTreeSet<u32> = steps_core::content::cloze_blanks(&self.fields[0])
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|(n, _)| *n > 0)
                    .map(|(n, _)| n - 1)
                    .collect();
                if ords.is_empty() { vec![0] } else { ords.into_iter().collect() }
            }
        }
    }
}

pub struct Deck {
    /// (deck id, full name, description)
    pub decks: Vec<(i64, String, String)>,
    pub notes: Vec<Note>,
    /// Notes per lecture, in order.
    pub per_lecture: Vec<(String, usize)>,
    /// Items left out: rejected or marked ✗.
    pub skipped: usize,
}

impl Deck {
    pub fn card_count(&self) -> usize {
        self.notes.iter().map(|n| n.card_ords().len()).sum()
    }
}

/// `Steps::Hepatology::03 Portal hypertension`: course, then the lecture's
/// number in it and its real title (Mohamed, 2026-10-29), so decks nest the
/// way the app lists them. Anki keeps cards it already has in their old
/// deck on import (`Steps::L03 …` from before), so the import note says how
/// to move them. Without a program: `Steps::L03 Portal hypertension`.
pub fn deck_name(lec: &Lecture, program: Option<&steps_core::program::Program>) -> String {
    if lec.lecture == steps_core::own::LECTURE {
        return format!("Steps::{}", steps_core::own::TITLE);
    }
    let num = lec.lecture.split_once('.').map(|(_, n)| n).unwrap_or(&lec.lecture);
    match course_deck(lec, program) {
        Some(course) => format!("{course}::{:0>2} {}", num.trim_start_matches('L'), lec.title),
        None => format!("Steps::{num} {}", lec.title),
    }
}

/// `Steps::Hepatology`: the lecture's course level.
fn course_deck(lec: &Lecture, program: Option<&steps_core::program::Program>) -> Option<String> {
    let course = program?.course_for(&lec.lecture)?;
    Some(format!("Steps::{}", course.title))
}

pub fn build(lectures: &[Lecture], opts: &Options<'_>) -> Deck {
    let mut deck = Deck {
        decks: vec![(stable_id("steps-deck-root"), "Steps".into(), ROOT_DESC.into())],
        notes: Vec::new(),
        per_lecture: Vec::new(),
        skipped: 0,
    };
    for lec in lectures {
        let did = stable_id(&format!("steps-deck-{}", lec.lecture));
        let desc = if lec.lecture == steps_core::own::LECTURE {
            "Cards Mohamed wrote himself in Steps.".to_string()
        } else {
            let what =
                if opts.program.is_some_and(|p| p.slides(&lec.lecture)) { "Slides" } else { "Book pp." };
            format!(
                "{} · {what} {}–{}{}",
                esc(&lec.title),
                lec.book.pages[0],
                lec.book.pages[1],
                lec.author.as_deref().map(|a| format!(" · {}", esc(a))).unwrap_or_default()
            )
        };
        if let Some(name) = course_deck(lec, opts.program)
            && !deck.decks.iter().any(|(_, n, _)| *n == name)
        {
            let id =
                stable_id(&format!("steps-deck-course-{}", steps_core::program::course_of(&lec.lecture)));
            deck.decks.push((id, name, String::new()));
        }
        deck.decks.push((did, deck_name(lec, opts.program), desc));
        let before = deck.notes.len();
        let draft = has(opts.drafts, &lec.lecture);
        for item in &lec.items {
            if item.status == Status::Rejected || has(opts.wrong, &item.id) {
                deck.skipped += 1;
                continue;
            }
            // An edit is his own text, not yet checked against the book.
            let (item, approved) = match opts.edits.and_then(|e| e.get(&item.id)) {
                Some(edited) => (edited, false),
                // Approved in the content file, or by his ✓ / approved.txt (#254).
                None => (item, item.status == Status::Approved || has(opts.approved, &item.id)),
            };
            let mut notes = notes_for(item, lec, did, approved);
            if draft {
                notes.iter_mut().for_each(|n| n.tags.push("draft".into()));
            }
            deck.notes.extend(notes);
        }
        deck.per_lecture.push((lec.lecture.clone(), deck.notes.len() - before));
    }
    deck
}

const ROOT_DESC: &str = "Steps: Alexandria Internal Medicine Postgraduate Lectures, Chapter 1 (Hepato-Biliary). \
Re-import a newer steps.apkg to update the cards; your reviews are kept. Cards tagged <b>unverified</b> \
are not checked yet: trust the book page on the back.";

/// The notes for one item, ported from make_apkg.py's `notes_for`.
pub fn notes_for(item: &Item, lec: &Lecture, deck: i64, approved: bool) -> Vec<Note> {
    let own = steps_core::own::is_own(&item.id);
    let src = match (own, item.src.page) {
        (true, 0) => esc(&lec.title),
        _ => format!("{} · Book p. {}", esc(&lec.title), item.src.page),
    };
    let upd =
        item.update.as_ref().map(|u| format!("{} ({})", esc(&u.text), esc(&u.reference))).unwrap_or_default();
    let (chapter, num) = lec.lecture.split_once('.').unwrap_or((&lec.lecture, ""));
    let mut tags = vec![
        format!("{chapter}::{num}"),
        format!("topic::{}", item.topic.as_deref().unwrap_or("misc")),
        if approved { "approved" } else { "unverified" }.to_string(),
    ];
    if own {
        // His own cards carry `own` (from the item) and the lecture they belong with.
        tags.truncate(1);
    }
    tags.extend(item.tags.iter().cloned());
    let tags: Vec<String> = tags.iter().map(|t| t.split_whitespace().collect::<Vec<_>>().join("-")).collect();
    let id = item.id.as_str();

    let basic = |front: String, back: String, suffix: &str| Note {
        model: Model::Basic,
        deck,
        fields: [front, back, src.clone(), upd.clone(), format!("{id}{suffix}")],
        tags: tags.clone(),
        guid: guid_for(&format!("{id}{suffix}")),
    };
    let letter = |i: usize| LETTERS.get(i).map(|&b| b as char).unwrap_or('?');
    // `order[shown] = original index`; see `option_order`.
    let options = |opts: &[String], order: &[usize]| {
        order
            .iter()
            .enumerate()
            .map(|(shown, &i)| format!("{}. {}", letter(shown), esc(&opts[i])))
            .collect::<Vec<_>>()
            .join("<br>")
    };
    let shown_letter = |order: &[usize], i: usize| letter(order.iter().position(|&o| o == i).unwrap_or(i));

    match &item.body {
        Body::Card { front, back } => vec![basic(esc(front), esc(back), "")],
        Body::Cloze { text, extra } => vec![Note {
            model: Model::Cloze,
            deck,
            fields: [
                esc(text),
                esc(extra.as_deref().unwrap_or("")),
                src.clone(),
                upd.clone(),
                id.to_string(),
            ],
            tags: tags.clone(),
            guid: guid_for(id),
        }],
        Body::Mcq { stem, options: opts, answer, why } => {
            let order = option_order(id, opts);
            let why: String = order
                .iter()
                .enumerate()
                .filter_map(|(shown, &i)| Some((shown, why.get(i)?)))
                .map(|(shown, w)| format!(r#"<div class="why"><b>{}</b>: {}</div>"#, letter(shown), esc(w)))
                .collect();
            vec![basic(
                format!("{}<br><br>{}", esc(stem), options(opts, &order)),
                format!("<b>{}</b><br>{why}", shown_letter(&order, *answer)),
                "",
            )]
        }
        Body::Enumerate { prompt, points, pass, hook } => vec![basic(
            format!("{}<br><i>({} points; pass at {pass})</i>", esc(prompt), points.len()),
            match hook {
                Some(h) => format!(r#"{}<div dir="rtl">😄 {}</div>"#, list(points), esc(h)),
                None => list(points),
            },
            "",
        )],
        Body::Essay { prompt, outline } => {
            let mut body = String::from("<ol>");
            for entry in outline {
                match entry {
                    OutlineEntry::Heading(h) => body.push_str(&format!("<li>{}</li>", esc(h))),
                    OutlineEntry::Section(map) => {
                        for (head, points) in map {
                            body.push_str(&format!("<li><b>{}</b>{}</li>", esc(head), list(points)));
                        }
                    }
                }
            }
            body.push_str("</ol>");
            vec![basic(format!("Essay outline: {}", esc(prompt)), body, "")]
        }
        Body::Case { stem, steps } => {
            let mut shown = esc(stem);
            let mut out = Vec::new();
            for (n, step) in steps.iter().enumerate() {
                let n = n + 1;
                if let Some(reveal) = step.reveal.as_deref().filter(|r| !r.is_empty()) {
                    shown.push_str(&format!("<br><br>{}", esc(reveal)));
                }
                let order = option_order(&format!("{id}#s{n}"), &step.options);
                let answer = match step.answer {
                    Some(a) if !step.options.is_empty() => format!("<b>{}</b> ", shown_letter(&order, a)),
                    _ => String::new(),
                };
                out.push(basic(
                    format!(
                        "{shown}<br><br><b>Step {n}:</b> {}<br>{}",
                        esc(&step.question),
                        options(&step.options, &order)
                    ),
                    format!("{answer}{}", esc(&step.why)),
                    &format!("#s{n}"),
                ));
            }
            out
        }
        Body::Table { title, columns, rows, blanks } => blanks
            .iter()
            .filter_map(|&[r, c]| {
                let row = rows.get(r)?;
                Some(basic(
                    format!(
                        "{}: <b>{}</b> → <b>{}</b>?",
                        esc(title),
                        esc(row.first()?),
                        esc(columns.get(c)?)
                    ),
                    esc(row.get(c)?),
                    &format!("#r{r}c{c}"),
                ))
            })
            .collect(),
        Body::Order { prompt, steps } => {
            vec![basic(format!("Put in order: {}", esc(prompt)), list(steps), "")]
        }
    }
}

/// Python's `html.escape`, then newlines as `<br>` (make_apkg.py's `esc`).
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            '\n' => out.push_str("<br>"),
            c => out.push(c),
        }
    }
    out
}

fn list(items: &[String]) -> String {
    let lis: String = items.iter().map(|i| format!("<li>{}</li>", esc(i))).collect();
    format!("<ol>{lis}</ol>")
}

/// make_apkg.py's `mid()`: the first 8 hex digits of the name's SHA-1.
pub fn stable_id(name: &str) -> i64 {
    let digest = Sha1::digest(name.as_bytes());
    i64::from(u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]))
}

/// genanki's `guid_for`: the first 8 bytes of SHA-256, in Anki's base91.
/// The order a note shows its options in (`order[shown] = original index`):
/// the bank's rule (`steps_core::exam::option_order`), seeded by the note id,
/// so the book's key isn't always A and a re-import keeps the same order.
pub fn option_order(note_id: &str, options: &[String]) -> Vec<usize> {
    let digest = Sha256::digest(note_id.as_bytes());
    let seed = u64::from_be_bytes(digest[..8].try_into().unwrap());
    steps_core::exam::option_order(options, &mut steps_core::exam::Rng::new(seed))
}

pub fn guid_for(value: &str) -> String {
    const TABLE: &[u8] =
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&()*+,-./:;<=>?@[]^_`{|}~";
    let digest = Sha256::digest(value.as_bytes());
    let mut n = u64::from_be_bytes(digest[..8].try_into().unwrap());
    let mut out = Vec::new();
    while n > 0 {
        out.push(TABLE[(n % TABLE.len() as u64) as usize]);
        n /= TABLE.len() as u64;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

/// Anki's duplicate check: the first 8 hex digits of the SHA-1 of the
/// first field without HTML.
fn checksum(field: &str) -> i64 {
    let mut text = String::new();
    let mut in_tag = false;
    for ch in field.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    let digest = Sha1::digest(text.trim().as_bytes());
    i64::from(u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]))
}

fn model_json(model: Model, deck: i64, now_s: i64) -> Value {
    let (name, fields, qfmt, afmt, kind) = match model {
        Model::Basic => (
            "Steps Basic",
            ["Front", "Back", "Source", "Update", "Id"],
            "{{Front}}".to_string(),
            format!(r#"{{{{FrontSide}}}}<hr id="answer">{{{{Back}}}}{BACK_TAIL}"#),
            0,
        ),
        Model::Cloze => (
            "Steps Cloze",
            ["Text", "Extra", "Source", "Update", "Id"],
            "{{cloze:Text}}".to_string(),
            format!("{{{{cloze:Text}}}}<br>{{{{Extra}}}}{BACK_TAIL}"),
            1,
        ),
    };
    let flds: Vec<Value> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            json!({"name": f, "ord": i, "font": "Liberation Sans", "media": [], "rtl": false, "size": 20, "sticky": false})
        })
        .collect();
    json!({
        "css": CSS,
        "did": deck,
        "flds": flds,
        "id": model.id().to_string(),
        "latexPost": "\\end{document}",
        "latexPre": "\\documentclass[12pt]{article}\n\\special{papersize=3in,5in}\n\\usepackage[utf8]{inputenc}\n\\usepackage{amssymb,amsmath}\n\\pagestyle{empty}\n\\setlength{\\parindent}{0in}\n\\begin{document}\n",
        "latexsvg": false,
        "mod": now_s,
        "name": name,
        "req": [[0, "all", [0]]],
        "sortf": 0,
        "tags": [],
        "tmpls": [{
            "name": if model == Model::Basic { "Card 1" } else { "Cloze" },
            "ord": 0, "qfmt": qfmt, "afmt": afmt,
            "bafmt": "", "bqfmt": "", "bfont": "", "bsize": 0, "did": null
        }],
        "type": kind,
        "usn": -1,
        "vers": []
    })
}

fn deck_json(id: i64, name: &str, desc: &str, now_s: i64) -> Value {
    json!({
        "collapsed": false, "conf": 1, "desc": desc, "dyn": 0, "extendNew": 0, "extendRev": 50,
        "id": id, "mod": now_s, "name": name, "usn": -1,
        "lrnToday": [0, 0], "newToday": [0, 0], "revToday": [0, 0], "timeToday": [0, 0]
    })
}

const SCHEMA: &str = "
CREATE TABLE col (id integer primary key, crt integer not null, mod integer not null, scm integer not null,
  ver integer not null, dty integer not null, usn integer not null, ls integer not null, conf text not null,
  models text not null, decks text not null, dconf text not null, tags text not null);
CREATE TABLE notes (id integer primary key, guid text not null, mid integer not null, mod integer not null,
  usn integer not null, tags text not null, flds text not null, sfld integer not null, csum integer not null,
  flags integer not null, data text not null);
CREATE TABLE cards (id integer primary key, nid integer not null, did integer not null, ord integer not null,
  mod integer not null, usn integer not null, type integer not null, queue integer not null, due integer not null,
  ivl integer not null, factor integer not null, reps integer not null, lapses integer not null, left integer not null,
  odue integer not null, odid integer not null, flags integer not null, data text not null);
CREATE TABLE revlog (id integer primary key, cid integer not null, usn integer not null, ease integer not null,
  ivl integer not null, lastIvl integer not null, factor integer not null, time integer not null, type integer not null);
CREATE TABLE graves (usn integer not null, oid integer not null, type integer not null);
CREATE INDEX ix_notes_usn on notes (usn);
CREATE INDEX ix_cards_usn on cards (usn);
CREATE INDEX ix_revlog_usn on revlog (usn);
CREATE INDEX ix_cards_nid on cards (nid);
CREATE INDEX ix_cards_sched on cards (did, queue, due);
CREATE INDEX ix_revlog_cid on revlog (cid);
CREATE INDEX ix_notes_csum on notes (csum);
";

/// genanki's collection config and default deck options.
const CONF: &str = r#"{"activeDecks":[1],"addToCur":true,"collapseTime":1200,"curDeck":1,"curModel":null,"dueCounts":true,"estTimes":true,"newBury":true,"newSpread":0,"nextPos":1,"sortBackwards":false,"sortType":"noteFld","timeLim":0}"#;
const DCONF: &str = r#"{"1":{"autoplay":true,"id":1,"lapse":{"delays":[10],"leechAction":0,"leechFails":8,"minInt":1,"mult":0},"maxTaken":60,"mod":0,"name":"Default","new":{"bury":true,"delays":[1,10],"initialFactor":2500,"ints":[1,4,7],"order":1,"perDay":20,"separate":true},"replayq":true,"rev":{"bury":true,"ease4":1.3,"fuzz":0.05,"ivlFct":1,"maxIvl":36500,"minSpace":1,"perDay":100},"timer":0,"usn":0}}"#;

/// The deck as `.apkg` bytes. Note and card ids start at `now_ms`, as Anki's
/// own do. Built in memory, so it needs no temp directory (Android has none
/// that is writable by default).
pub fn to_apkg(deck: &Deck, now_ms: i64) -> Result<Vec<u8>, String> {
    let s = |e: &dyn std::fmt::Display| e.to_string();
    let now_s = now_ms / 1000;
    let collection = {
        let mut db = Connection::open_in_memory().map_err(|e| s(&e))?;
        db.execute_batch(SCHEMA).map_err(|e| s(&e))?;

        let first_deck = deck.decks.last().map(|d| d.0).unwrap_or(1);
        let models: BTreeMap<String, Value> = [Model::Basic, Model::Cloze]
            .into_iter()
            .map(|m| (m.id().to_string(), model_json(m, first_deck, now_s)))
            .collect();
        let mut decks = BTreeMap::new();
        decks.insert("1".to_string(), deck_json(1, "Default", "", now_s));
        for (id, name, desc) in &deck.decks {
            decks.insert(id.to_string(), deck_json(*id, name, desc, now_s));
        }
        db.execute(
            "INSERT INTO col VALUES (1, ?1, ?2, ?2, 11, 0, 0, 0, ?3, ?4, ?5, ?6, '{}')",
            params![
                now_s - now_s % 86_400,
                now_ms,
                CONF,
                serde_json::to_string(&models).unwrap(),
                serde_json::to_string(&decks).unwrap(),
                DCONF
            ],
        )
        .map_err(|e| s(&e))?;

        let tx = db.unchecked_transaction().map_err(|e| s(&e))?;
        let mut next_id = now_ms;
        let mut due = 0;
        for note in &deck.notes {
            let nid = next_id;
            next_id += 1;
            tx.execute(
                "INSERT INTO notes VALUES (?1, ?2, ?3, ?4, -1, ?5, ?6, ?7, ?8, 0, '')",
                params![
                    nid,
                    note.guid,
                    note.model.id(),
                    now_s,
                    format!(" {} ", note.tags.join(" ")),
                    note.fields.join("\x1f"),
                    note.fields[0],
                    checksum(&note.fields[0]),
                ],
            )
            .map_err(|e| s(&e))?;
            for ord in note.card_ords() {
                // New cards come in book order: `due` is their position.
                due += 1;
                tx.execute(
                    "INSERT INTO cards VALUES (?1, ?2, ?3, ?4, ?5, -1, 0, 0, ?6, 0, 0, 0, 0, 0, 0, 0, 0, '')",
                    params![next_id, nid, note.deck, ord, now_s, due],
                )
                .map_err(|e| s(&e))?;
                next_id += 1;
            }
        }
        tx.commit().map_err(|e| s(&e))?;
        db.serialize(MAIN_DB).map_err(|e| s(&e))?.to_vec()
    };

    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let deflated =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("collection.anki2", deflated).map_err(|e| s(&e))?;
    zip.write_all(&collection).map_err(|e| s(&e))?;
    zip.start_file("media", deflated).map_err(|e| s(&e))?;
    zip.write_all(b"{}").map_err(|e| s(&e))?;
    Ok(zip.finish().map_err(|e| s(&e))?.into_inner())
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use super::*;

    fn fixture() -> Lecture {
        serde_yaml_ng::from_str(include_str!("../../../fixtures/content/sample.yaml")).unwrap()
    }

    #[test]
    fn ids_match_the_python_exporter() {
        // Values from genanki.guid_for and make_apkg.py's mid().
        assert_eq!(guid_for("hb.L01.001"), "kO@wJGlHO&");
        assert_eq!(guid_for("hb.L01.006#s2"), "HxUBC_F~bH");
        assert_eq!(Model::Basic.id(), 1_650_575_226);
        assert_eq!(Model::Cloze.id(), 3_011_687_230);
        assert_eq!(stable_id("steps-deck-hb.L01"), 2_720_016_684);
        assert_eq!(deck_name(&fixture(), None), "Steps::L01 Liver Cirrhosis");
        let program: steps_core::program::Program =
            serde_yaml_ng::from_str(include_str!("../../../content/program.yaml")).unwrap();
        let mut cvs = fixture();
        cvs.lecture = "cvs.L01".into();
        cvs.title = "Embryology".into();
        assert_eq!(deck_name(&cvs, Some(&program)), "Steps::Cardiovascular::01 Embryology");
        let mut renal = fixture();
        renal.lecture = "renal.L03".into();
        renal.title = "Physiology".into();
        assert_eq!(deck_name(&renal, Some(&program)), "Steps::Renal::03 Physiology");
        let opts = Options { program: Some(&program), ..Default::default() };
        let deck = build(&[cvs, renal.clone(), renal], &opts);
        let names: Vec<_> = deck.decks.iter().map(|(_, n, _)| n.as_str()).collect();
        assert_eq!(names.iter().filter(|n| **n == "Steps::Renal").count(), 1, "{names:?}");
        assert_eq!(names.iter().filter(|n| **n == "Steps::Cardiovascular").count(), 1, "{names:?}");
        assert_eq!(esc("a<b & \"c\"\nd'"), "a&lt;b &amp; &quot;c&quot;<br>d&#x27;");
    }

    #[test]
    fn options_shuffle_per_note_and_keep_the_book_order_when_they_must() {
        let s = |xs: &[&str]| xs.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let four = s(&["Propranolol", "Carvedilol", "Band ligation", "TIPS"]);
        // Same id, same order: a re-import doesn't move the options.
        assert_eq!(option_order("hb.L04.001", &four), option_order("hb.L04.001", &four));
        // Across notes the key lands on every letter, not always A.
        let keys: BTreeSet<usize> = (1..=40)
            .map(|n| option_order(&format!("hb.L04.{n:03}"), &four).iter().position(|&i| i == 0).unwrap())
            .collect();
        assert_eq!(keys.len(), 4, "{keys:?}");
        // "All of the above" and number ladders keep their sense.
        assert_eq!(option_order("hb.L01.001", &s(&["a", "b", "c", "All of the above"])), [0, 1, 2, 3]);
        assert_eq!(option_order("hb.L01.001", &s(&["HPS", "PoPH", "Both", "Neither"])), [0, 1, 2, 3]);
        assert_eq!(option_order("hb.L01.001", &s(&["Above 30", "Above 10", "Above 20"])), [1, 2, 0]);
    }

    #[test]
    fn approved_status_in_the_file_tags_approved() {
        let mut lec = fixture();
        lec.items[1].status = Status::Approved;
        let deck = build(std::slice::from_ref(&lec), &Options::default());
        let id = lec.items[1].id.clone();
        let note = deck.notes.iter().find(|n| n.fields[4] == id).unwrap();
        assert!(note.tags.contains(&"approved".to_string()), "{:?}", note.tags);
        assert!(deck.notes[0].tags.contains(&"unverified".to_string()));
    }

    #[test]
    fn every_kind_becomes_notes() {
        let lec = fixture();
        let approved = BTreeSet::from(["hb.L01.001".to_string()]);
        let opts = Options { approved: Some(&approved), ..Options::default() };
        let deck = build(std::slice::from_ref(&lec), &opts);
        // 7 one-note items + 2 case steps + 3 table blanks.
        assert_eq!(deck.notes.len(), 11);
        assert_eq!(deck.card_count(), 13, "the cloze has three blanks");
        assert_eq!(deck.notes[0].tags, ["hb::L01", "topic::definition", "approved"]);
        assert_eq!(
            deck.notes[1].tags,
            ["hb::L01", "topic::child-pugh", "unverified", "numbers", "high-yield"]
        );
        // Options are shuffled per note id (#87); the answer and the why lines follow.
        let mcq = &deck.notes[2];
        assert!(
            mcq.fields[0]
                .ends_with("A. Class A<br>B. Cannot be scored without the INR<br>C. Class C<br>D. Class B")
        );
        assert!(mcq.fields[1].starts_with(
            "<b>D</b><br><div class=\"why\"><b>A</b>: Class A needs 5-6 points; he has 8.</div><div class=\"why\"><b>B</b>: The book&#x27;s table"
        ));
        let step2 = deck.notes.iter().find(|n| n.fields[4] == "hb.L01.006#s2").unwrap();
        assert!(step2.fields[0].contains("PaO2 falls by 6 mmHg on standing.<br><br><b>Step 2:</b>"));
        assert!(
            step2.fields[0].ends_with(
                "A. Liver transplantation<br>B. Long-term oxygen alone<br>C. TIPS<br>D. Carvedilol"
            )
        );
        assert_eq!(step2.fields[1], "<b>A</b> LT improves oxygenation at 1 year. TIPS can worsen shunting.");
        let blank = deck.notes.iter().find(|n| n.fields[4] == "hb.L01.007#r2c1").unwrap();
        assert_eq!(blank.fields[0], "Child-Pugh points: <b>Bilirubin (mg/dL)</b> → <b>1 point</b>?");
        assert_eq!(blank.fields[1], "&lt;2");
        let order = deck.notes.last().unwrap();
        assert_eq!(order.fields[2], "Liver Cirrhosis · Book p. 16");
        assert_eq!(
            deck.decks.iter().map(|d| d.1.as_str()).collect::<Vec<_>>(),
            ["Steps", "Steps::L01 Liver Cirrhosis"]
        );
    }

    #[test]
    fn wrong_items_drop_out_and_edits_replace_the_book_text() {
        let lec = fixture();
        let wrong = BTreeSet::from(["hb.L01.003".to_string()]);
        let approved = BTreeSet::from(["hb.L01.001".to_string(), "hb.L01.008".to_string()]);
        let mut edited = lec.items[0].clone();
        if let Body::Card { back, .. } = &mut edited.body {
            *back = "Fibrosis plus nodules.".into();
        }
        let edits = BTreeMap::from([("hb.L01.001".to_string(), edited)]);
        let drafts = BTreeSet::from(["hb.L01".to_string()]);
        let opts = Options {
            approved: Some(&approved),
            wrong: Some(&wrong),
            edits: Some(&edits),
            drafts: Some(&drafts),
            program: None,
        };
        let deck = build(&[lec], &opts);
        assert_eq!(deck.skipped, 1);
        assert_eq!(deck.notes.len(), 10);
        assert_eq!(deck.notes[0].fields[1], "Fibrosis plus nodules.");
        assert!(deck.notes[0].tags.contains(&"unverified".to_string()));
        let order = deck.notes.iter().find(|n| n.fields[4] == "hb.L01.008").unwrap();
        assert_eq!(order.tags, ["hb::L01", "topic::management", "approved", "draft"]);
    }

    #[test]
    fn the_apkg_is_an_anki_collection() {
        let dir = tempfile::tempdir().unwrap();
        let deck = build(&[fixture()], &Options::default());
        let bytes = to_apkg(&deck, 1_800_000_000_000).unwrap();

        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut media = String::new();
        zip.by_name("media").unwrap().read_to_string(&mut media).unwrap();
        assert_eq!(media, "{}");
        let mut collection = Vec::new();
        zip.by_name("collection.anki2").unwrap().read_to_end(&mut collection).unwrap();
        let db_path = dir.path().join("collection.anki2");
        std::fs::write(&db_path, &collection).unwrap();

        // Read it back the way the brief asks: with Python's sqlite3.
        let script = r#"
import json, sqlite3, sys
db = sqlite3.connect(sys.argv[1])
assert db.execute("pragma integrity_check").fetchone()[0] == "ok"
notes = db.execute("select count(*) from notes").fetchone()[0]
cards = db.execute("select count(*) from cards").fetchone()[0]
orphans = db.execute("select count(*) from cards where nid not in (select id from notes)").fetchone()[0]
models, decks = map(json.loads, db.execute("select models, decks from col").fetchone())
fields = db.execute("select flds from notes where guid = 'kO@wJGlHO&'").fetchone()[0].split("\x1f")
names = sorted(d["name"] for d in decks.values())
print(notes, cards, orphans, sorted(m["name"] for m in models.values()), names, fields[4])
"#;
        let out =
            match std::process::Command::new("python3").args(["-I", "-c", script]).arg(&db_path).output() {
                Ok(out) => out,
                Err(e) => {
                    eprintln!("python3 not found ({e}); checking with rusqlite only");
                    let db = Connection::open(&db_path).unwrap();
                    let n: i64 = db.query_row("select count(*) from notes", [], |r| r.get(0)).unwrap();
                    assert_eq!(n, 12);
                    return;
                }
            };
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            "11 13 0 ['Steps Basic', 'Steps Cloze'] ['Default', 'Steps', 'Steps::L01 Liver Cirrhosis'] hb.L01.001"
        );
    }
}
