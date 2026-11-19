//! Questions Mohamed adds himself from a file (Settings › بياناتك › «ضيف
//! أسئلة من ملف», docs/OWN-QUESTIONS.md): rows of a spreadsheet, or the same
//! fields in JSON or YAML, each a single-best-answer MCQ under a category of
//! his naming. Each category is a bank of its own (`exam::Set::Own`), and
//! each question joins the First Aid lecture it names, by page or id, so the
//! bank's lecture and topic filters take it like any other.
//!
//! Pure: the app reads the file, the images and the store; this turns rows
//! into [`OwnQuestion`]s, says what is wrong with which row, and what an
//! import would add or change ([`plan`]). Stored as JSON in the synced
//! `own_questions` table, one row per question; figures go in `own_images`
//! in chunks small enough for one sync op each ([`chunks`]).

use std::collections::{BTreeMap, BTreeSet};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use crate::content::{Body, Index, Item, Lecture, Source, Status};

/// Every imported question carries this tag, and `mine:<category>`.
pub const TAG: &str = "mine";
const SET_PREFIX: &str = "mine:";
/// The letters of the options a row may have.
pub const LETTERS: &str = "ABCDEFGHIJ";

pub const MAX_ID: usize = 40;
pub const MAX_CATEGORY: usize = 30;
pub const MAX_STEM: usize = 4000;
pub const MAX_OPTION: usize = 500;
pub const MAX_WHY: usize = 2000;
pub const MAX_TEXT: usize = 6000;
pub const MAX_TAGS: usize = 10;
/// A figure may be at most 1 MB.
pub const MAX_IMAGE_BYTES: usize = 1024 * 1024;
/// Raw bytes per stored chunk: 45 KB is 60 KB as base64, inside a sync op.
pub const CHUNK_BYTES: usize = 45 * 1024;
/// The figure kinds a question may have.
pub const IMAGE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/webp", "image/gif"];

/// One question, as stored (JSON in `own_questions.value`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnQuestion {
    /// As he wrote it (`P-0001`): unique in its category.
    pub id: String,
    /// The bank it is in (`Pathoma`).
    pub category: String,
    /// The First Aid lecture it joins (`cvs.L03`).
    pub lecture: String,
    /// A topic of that lecture, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    /// The printed First Aid page it names, when it named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    pub stem: String,
    pub options: Vec<String>,
    pub answer: usize,
    /// One per option, or none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub why: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    /// A YouTube video id, and the title he gave it (may be empty).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<crate::sealed::Video>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// A figure, stored apart in chunks under its SHA-256.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {
    pub sha256: String,
    /// `image/png`, `image/jpeg`, `image/webp` or `image/gif`.
    pub kind: String,
    pub bytes: u32,
    pub chunks: u32,
}

impl OwnQuestion {
    /// The row id: `<category>/<id>`.
    pub fn key(&self) -> String {
        key(&self.category, &self.id)
    }

    /// The item id in its lecture: `cvs.L03.mine-<category hash>-p-0001`.
    /// The same file gives the same ids on every device.
    pub fn item_id(&self) -> String {
        format!(
            "{}.mine-{:08x}-{}",
            self.lecture,
            fnv(&self.category.to_lowercase()),
            self.id.to_ascii_lowercase()
        )
    }

    /// The bank's item: an MCQ in its lecture, approved because he wrote it.
    /// A question with no why lines has an empty one per option; the
    /// explanation stands on its own.
    pub fn item(&self) -> Item {
        let mut tags = vec![TAG.to_string(), set_tag(&self.category)];
        tags.extend(self.tags.iter().cloned());
        let why = if self.why.len() == self.options.len() {
            self.why.clone()
        } else {
            vec![String::new(); self.options.len()]
        };
        Item {
            id: self.item_id(),
            body: Body::Mcq {
                stem: self.stem.clone(),
                options: self.options.clone(),
                answer: self.answer,
                why,
            },
            topic: self.topic.clone(),
            src: Source { book: Some(TAG.to_string()), page: self.page.unwrap_or(0), quote: None },
            status: Status::Approved,
            update: None,
            tags,
        }
    }
}

pub fn key(category: &str, id: &str) -> String {
    format!("{category}/{id}")
}

/// `mine:<category>`.
pub fn set_tag(category: &str) -> String {
    format!("{SET_PREFIX}{category}")
}

/// The category an item's tags put it in, if it is an imported question.
pub fn category_of(tags: &[String]) -> Option<&str> {
    tags.iter().find_map(|t| t.strip_prefix(SET_PREFIX))
}

pub fn is_mine(tags: &[String]) -> bool {
    tags.iter().any(|t| t == TAG)
}

/// FNV-1a, 32 bits: a short, stable stand-in for a category in ids.
fn fnv(text: &str) -> u32 {
    text.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193))
}

/// A figure's bytes in stored chunks: `<sha256>.<n>` → base64.
pub fn chunks(sha256: &str, bytes: &[u8]) -> Vec<(String, String)> {
    bytes
        .chunks(CHUNK_BYTES)
        .enumerate()
        .map(|(i, c)| (format!("{sha256}.{i}"), STANDARD.encode(c)))
        .collect()
}

/// The figure again from its chunks, in order; none when one is missing.
pub fn join(image: &Image, chunk: impl Fn(&str) -> Option<String>) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(image.bytes as usize);
    for i in 0..image.chunks {
        out.extend(STANDARD.decode(chunk(&format!("{}.{i}", image.sha256))?).ok()?);
    }
    (out.len() == image.bytes as usize).then_some(out)
}

/// What a figure's first bytes say it is.
pub fn image_kind(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        [0xff, 0xd8, 0xff, ..] => Some("image/jpeg"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("image/webp"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        _ => None,
    }
}

/// A figure the file brings, by the name a row gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileImage {
    pub image: Image,
}

/// One row of the file: its line (2 for the first row under the headings)
/// and its fields by column name, trimmed, empty ones left out.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Row {
    pub line: usize,
    pub fields: BTreeMap<String, String>,
}

/// What is wrong with a row, for the app to say in Arabic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    NoRows,
    UnknownColumn(String),
    Missing(&'static str),
    BadId(String),
    Twice(String),
    TooLong(&'static str, usize),
    FewOptions,
    Gap(char),
    BadAnswer(String),
    WhyCount { options: usize, why: usize },
    NoPlace,
    PageNotInBook(u32),
    BadPage(String),
    UnknownLecture(String),
    UnknownTopic(String),
    BadVideo(String),
    NoImage(String),
    TooManyTags,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowProblem {
    /// 0 for the file as a whole.
    pub line: usize,
    pub problem: Problem,
}

/// The columns a file may have.
pub fn columns() -> Vec<String> {
    let mut out: Vec<String> =
        ["id", "category", "page", "lecture", "topic", "stem"].iter().map(|s| s.to_string()).collect();
    out.extend(LETTERS.chars().map(|c| c.to_string()));
    out.extend(["answer", "explanation"].iter().map(|s| s.to_string()));
    out.extend(LETTERS.chars().map(|c| format!("why_{c}")));
    out.extend(["objective", "image", "video", "video_title", "tags"].iter().map(|s| s.to_string()));
    out
}

/// Where rows can go: the index (pages → lectures) and the lectures (topics).
pub struct Places<'a> {
    pub index: &'a Index,
    pub lectures: &'a BTreeMap<String, Lecture>,
}

impl Places<'_> {
    fn lecture_of_page(&self, page: u32) -> Option<&str> {
        let e = self.index.lectures.iter().find(|e| (e.pages[0]..=e.pages[1]).contains(&page))?;
        self.lectures.contains_key(&e.id).then_some(e.id.as_str())
    }

    /// A topic by id or by title, any case.
    fn topic(&self, lecture: &str, wanted: &str) -> Option<String> {
        let w = wanted.trim().to_lowercase();
        let lec = self.lectures.get(lecture)?;
        lec.topics
            .iter()
            .find(|t| t.id.to_lowercase() == w || t.title.to_lowercase() == w)
            .map(|t| t.id.clone())
    }
}

/// The rows as questions, with what is wrong with any row. A row with a
/// problem is left out; the rest stand.
pub fn read(
    rows: &[Row],
    places: &Places,
    images: &BTreeMap<String, FileImage>,
) -> (Vec<OwnQuestion>, Vec<RowProblem>) {
    let mut problems = Vec::new();
    if rows.is_empty() {
        problems.push(RowProblem { line: 0, problem: Problem::NoRows });
    }
    let known: BTreeSet<String> = columns().into_iter().collect();
    let mut unknown: BTreeSet<&str> = BTreeSet::new();
    for r in rows {
        unknown.extend(r.fields.keys().map(String::as_str).filter(|k| !known.contains(*k)));
    }
    problems.extend(
        unknown.into_iter().map(|k| RowProblem { line: 0, problem: Problem::UnknownColumn(k.into()) }),
    );
    let mut out = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for r in rows {
        match question(r, places, images) {
            Ok(q) => {
                let k = key(&q.category.to_lowercase(), &q.id.to_lowercase());
                if let Some(first) = seen.insert(k, r.line) {
                    problems.push(RowProblem {
                        line: r.line,
                        problem: Problem::Twice(format!("{} ({first})", q.id)),
                    });
                } else {
                    out.push(q);
                }
            }
            Err(p) => problems.extend(p.into_iter().map(|problem| RowProblem { line: r.line, problem })),
        }
    }
    (out, problems)
}

fn question(
    r: &Row,
    places: &Places,
    images: &BTreeMap<String, FileImage>,
) -> Result<OwnQuestion, Vec<Problem>> {
    let f = |k: &str| r.fields.get(k).map(String::as_str).filter(|v| !v.trim().is_empty()).map(str::trim);
    let mut p = Vec::new();
    let mut long = |name: &'static str, text: Option<&str>, max: usize| {
        if text.is_some_and(|t| t.chars().count() > max) {
            p.push(Problem::TooLong(name, max));
        }
    };
    long("id", f("id"), MAX_ID);
    long("category", f("category"), MAX_CATEGORY);
    long("stem", f("stem"), MAX_STEM);
    long("explanation", f("explanation"), MAX_TEXT);
    long("objective", f("objective"), MAX_TEXT);
    for c in LETTERS.chars() {
        long("option", f(&c.to_string()), MAX_OPTION);
        long("why", f(&format!("why_{c}")), MAX_WHY);
    }
    let id = f("id").unwrap_or_default().to_string();
    if id.is_empty() {
        p.push(Problem::Missing("id"));
    } else if !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
        p.push(Problem::BadId(id.clone()));
    }
    let category = f("category").unwrap_or_default().to_string();
    if category.is_empty() {
        p.push(Problem::Missing("category"));
    }
    let stem = f("stem").unwrap_or_default().to_string();
    if stem.is_empty() {
        p.push(Problem::Missing("stem"));
    }
    // Options: A, B, … with no gap.
    let given: Vec<(char, Option<&str>)> = LETTERS.chars().map(|c| (c, f(&c.to_string()))).collect();
    let last = given.iter().rposition(|(_, v)| v.is_some());
    let mut options = Vec::new();
    if let Some(last) = last {
        for (c, v) in &given[..=last] {
            match v {
                Some(v) => options.push(v.to_string()),
                None => p.push(Problem::Gap(*c)),
            }
        }
    }
    if options.len() < 2 {
        p.push(Problem::FewOptions);
    }
    let answer = match f("answer") {
        None => {
            p.push(Problem::Missing("answer"));
            0
        }
        Some(a) => {
            let letter = a.trim().to_ascii_uppercase();
            match LETTERS.find(letter.as_str()).filter(|i| letter.len() == 1 && *i < options.len().max(1)) {
                Some(i) if !options.is_empty() => i,
                _ => {
                    p.push(Problem::BadAnswer(a.to_string()));
                    0
                }
            }
        }
    };
    let why: Vec<Option<&str>> =
        LETTERS.chars().take(options.len()).map(|c| f(&format!("why_{c}"))).collect();
    let written = why.iter().filter(|w| w.is_some()).count();
    let extra = LETTERS.chars().skip(options.len()).filter(|c| f(&format!("why_{c}")).is_some()).count();
    if (written > 0 && written < options.len()) || extra > 0 {
        p.push(Problem::WhyCount { options: options.len(), why: written + extra });
    }
    let why: Vec<String> = if written == options.len() {
        why.into_iter().map(|w| w.unwrap_or("").to_string()).collect()
    } else {
        vec![]
    };
    // Where it goes: a lecture id, or a page.
    let page = match f("page") {
        None => None,
        Some(t) => match t.trim_end_matches(".0").parse::<u32>() {
            Ok(n) if n > 0 => Some(n),
            _ => {
                p.push(Problem::BadPage(t.to_string()));
                None
            }
        },
    };
    let lecture = match (f("lecture"), page) {
        (Some(l), _) => {
            if places.lectures.contains_key(l) {
                Some(l.to_string())
            } else {
                p.push(Problem::UnknownLecture(l.to_string()));
                None
            }
        }
        (None, Some(n)) => {
            let found = places.lecture_of_page(n).map(str::to_string);
            if found.is_none() {
                p.push(Problem::PageNotInBook(n));
            }
            found
        }
        (None, None) => {
            if f("page").is_none() {
                p.push(Problem::NoPlace);
            }
            None
        }
    };
    let topic = match (f("topic"), &lecture) {
        (Some(t), Some(l)) => {
            let found = places.topic(l, t);
            if found.is_none() {
                p.push(Problem::UnknownTopic(t.to_string()));
            }
            found
        }
        _ => None,
    };
    let video = match f("video") {
        None => None,
        Some(v) => match youtube_id(v) {
            Some(id) => Some(crate::sealed::Video {
                youtube: id,
                title: f("video_title")
                    .unwrap_or_default()
                    .chars()
                    .take(crate::sealed::Video::MAX_TITLE)
                    .collect(),
            }),
            None => {
                p.push(Problem::BadVideo(v.to_string()));
                None
            }
        },
    };
    let image = match f("image") {
        None => None,
        Some(name) => match find_image(images, name) {
            Some(i) => Some(i.image.clone()),
            None => {
                p.push(Problem::NoImage(name.to_string()));
                None
            }
        },
    };
    let tags: Vec<String> = f("tags")
        .map(|t| t.split([',', '،']).map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    if tags.len() > MAX_TAGS {
        p.push(Problem::TooManyTags);
    }
    if !p.is_empty() {
        return Err(p);
    }
    Ok(OwnQuestion {
        id,
        category,
        lecture: lecture.expect("checked above"),
        topic,
        page,
        stem,
        options,
        answer,
        why,
        explanation: f("explanation").map(str::to_string),
        objective: f("objective").map(str::to_string),
        image,
        video,
        tags: tags.into_iter().filter(|t| t != TAG && !t.starts_with(SET_PREFIX)).collect(),
    })
}

/// The figure a row names: the exact name first, then any case, then the
/// file name without its folder.
fn find_image<'a>(images: &'a BTreeMap<String, FileImage>, name: &str) -> Option<&'a FileImage> {
    let base = |s: &str| s.rsplit(['/', '\\']).next().unwrap_or(s).to_lowercase();
    images.get(name).or_else(|| images.iter().find(|(k, _)| base(k) == base(name)).map(|(_, v)| v))
}

/// The video id in a YouTube link (`watch?v=`, `youtu.be/`, `/shorts/`,
/// `/embed/`) or a bare id.
pub fn youtube_id(text: &str) -> Option<String> {
    let t = text.trim();
    if crate::sealed::Video::is_id(t) {
        return Some(t.to_string());
    }
    let rest = t.split_once("://").map_or(t, |(_, r)| r);
    let (host, path) = rest.split_once('/')?;
    let host = host.trim_start_matches("www.").trim_start_matches("m.");
    let id = match host {
        "youtu.be" => path.split(['?', '&', '#']).next()?,
        "youtube.com" | "youtube-nocookie.com" => {
            if let Some(q) = path.strip_prefix("watch?").or_else(|| path.split_once("watch?").map(|(_, q)| q))
            {
                q.split('&').find_map(|kv| kv.strip_prefix("v="))?
            } else {
                let p = path.strip_prefix("shorts/").or_else(|| path.strip_prefix("embed/"))?;
                p.split(['?', '&', '#', '/']).next()?
            }
        }
        _ => return None,
    };
    crate::sealed::Video::is_id(id).then(|| id.to_string())
}

/// What an import would do, by row key.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    pub new: Vec<String>,
    pub changed: Vec<String>,
    pub same: Vec<String>,
}

pub fn plan(existing: &[OwnQuestion], incoming: &[OwnQuestion]) -> Plan {
    let by_key: BTreeMap<String, &OwnQuestion> = existing.iter().map(|q| (q.key(), q)).collect();
    let mut out = Plan::default();
    for q in incoming {
        match by_key.get(&q.key()) {
            None => out.new.push(q.key()),
            Some(old) if *old == q => out.same.push(q.key()),
            Some(_) => out.changed.push(q.key()),
        }
    }
    out
}

/// The categories, each with how many questions, in name order.
pub fn categories(questions: &[OwnQuestion]) -> Vec<(String, usize)> {
    let mut out: BTreeMap<&str, usize> = BTreeMap::new();
    for q in questions {
        *out.entry(q.category.as_str()).or_default() += 1;
    }
    out.into_iter().map(|(c, n)| (c.to_string(), n)).collect()
}

#[cfg(test)]
#[path = "ownq_tests.rs"]
mod tests;
