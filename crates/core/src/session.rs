//! The study session: which items to show, in what order, and how a typed
//! answer is checked. Pure logic; the app's `study` screen drives it.
//!
//! Order: due reviews first, interleaved across topics (mixing helps
//! retrieval), then new items blocked by topic in book order (one idea at a
//! time while it is first learned). Everything is capped at a time budget.

use std::collections::{BTreeMap, HashSet};

use crate::content::{Body, Item, Lecture, OutlineEntry, Status};
use crate::review::Grade;

/// A rough time for one item, in seconds, used to fit a session to its budget.
pub fn estimate_secs(body: &Body) -> u32 {
    match body {
        Body::Card { .. } => 20,
        Body::Cloze { text, .. } => {
            let blanks = crate::content::cloze_blanks(text).map_or(1, |b| b.len().max(1));
            15 + 10 * blanks as u32
        }
        Body::Mcq { .. } => 45,
        Body::Enumerate { points, .. } => 30 + 6 * points.len() as u32,
        Body::Essay { outline, .. } => 60 + 20 * outline.len() as u32,
        Body::Case { steps, .. } => 30 + 40 * steps.len() as u32,
        Body::Table { blanks, .. } => 15 + 20 * blanks.len() as u32,
        Body::Order { steps, .. } => 10 + 8 * steps.len() as u32,
        Body::Calc { .. } => 90,
    }
}

/// What the composer gets from the review store and the learner's choices.
#[derive(Debug, Clone)]
pub struct Inputs<'a> {
    /// The lectures to study from, in book order.
    pub lectures: &'a [&'a Lecture],
    /// Only these topics (`hb.L01/definition`); empty means every topic.
    pub topics: &'a HashSet<String>,
    /// Item kinds the player can show (`card`, `cloze`, …).
    pub kinds: &'a [&'a str],
    /// Due item ids, most overdue first.
    pub due: &'a [String],
    /// Ids already studied at least once (not new).
    pub seen: &'a HashSet<String>,
    /// Ids the learner marked wrong (✗): kept out until fixed.
    pub hidden: &'a HashSet<String>,
    /// How many new items may still start today.
    pub max_new: usize,
    /// Time budget in seconds.
    pub budget_secs: u32,
}

/// One planned item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub item_id: String,
    pub lecture: String,
    /// `hb.L01/definition`, or the lecture id when the item has no topic.
    pub topic: String,
    pub est_secs: u32,
    /// First time ever seen.
    pub fresh: bool,
}

/// Builds the ordered session. Always returns at least one item when anything
/// is available, even if that one item is longer than the budget.
pub fn compose(input: &Inputs<'_>) -> Vec<Planned> {
    // Every eligible item, with its book position for stable ordering.
    let mut pool: BTreeMap<&str, (usize, Planned)> = BTreeMap::new();
    let mut book_pos = 0usize;
    for lecture in input.lectures {
        for item in &lecture.items {
            book_pos += 1;
            if !eligible(item, input) {
                continue;
            }
            let topic = topic_key(&lecture.lecture, item);
            if !input.topics.is_empty() && !input.topics.contains(&topic) {
                continue;
            }
            let planned = Planned {
                item_id: item.id.clone(),
                lecture: lecture.lecture.clone(),
                topic,
                est_secs: estimate_secs(&item.body),
                fresh: !input.seen.contains(&item.id),
            };
            pool.insert(item.id.as_str(), (book_pos, planned));
        }
    }

    let mut taken: HashSet<String> = HashSet::new();
    let due: Vec<Planned> = input
        .due
        .iter()
        .filter_map(|id| pool.get(id.as_str()))
        .filter(|(_, p)| taken.insert(p.item_id.clone()))
        .map(|(_, p)| Planned { fresh: false, ..p.clone() })
        .collect();

    let mut fresh: Vec<(usize, Planned)> =
        pool.values().filter(|(_, p)| p.fresh && !taken.contains(&p.item_id)).cloned().collect();
    fresh.sort_by_key(|(pos, _)| *pos);
    let fresh = block_by_topic(fresh.into_iter().map(|(_, p)| p).collect());
    let mut new_left = input.max_new;

    let mut out = Vec::new();
    let mut used = 0u32;
    for p in interleave(due).into_iter().chain(fresh) {
        if p.fresh && new_left == 0 {
            continue;
        }
        if out.is_empty() || used + p.est_secs <= input.budget_secs {
            new_left -= usize::from(p.fresh);
            used += p.est_secs;
            out.push(p);
        }
    }
    out
}

fn eligible(item: &Item, input: &Inputs<'_>) -> bool {
    item.status != Status::Rejected
        && !input.hidden.contains(&item.id)
        && input.kinds.contains(&item.body.kind())
}

fn topic_key(lecture: &str, item: &Item) -> String {
    match &item.topic {
        Some(t) => format!("{lecture}/{t}"),
        None => lecture.to_string(),
    }
}

/// Keeps book order but pulls every item of a topic together, topics in the
/// order they first appear.
fn block_by_topic(items: Vec<Planned>) -> Vec<Planned> {
    let mut order: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, Vec<Planned>> = BTreeMap::new();
    for p in items {
        if !groups.contains_key(&p.topic) {
            order.push(p.topic.clone());
        }
        groups.entry(p.topic.clone()).or_default().push(p);
    }
    order.into_iter().flat_map(|t| groups.remove(&t).unwrap_or_default()).collect()
}

/// Reorders so that two neighbours share a topic only when nothing else is
/// left, while keeping the most overdue items near the front.
fn interleave(mut items: Vec<Planned>) -> Vec<Planned> {
    let mut out: Vec<Planned> = Vec::with_capacity(items.len());
    while !items.is_empty() {
        let last = out.last().map(|p| p.topic.clone());
        let pick = items.iter().position(|p| Some(&p.topic) != last.as_ref()).unwrap_or(0);
        out.push(items.remove(pick));
    }
    out
}

// ---------------------------------------------------------------------------
// A running session

/// How one item went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub item_id: String,
    pub grade: Grade,
    pub fresh: bool,
    /// Graded again later in the same session after an Again.
    pub retry: bool,
}

/// A session in progress. An item graded Again comes back once at the end of
/// the session, so it is retrieved again while it is still fresh.
#[derive(Debug, Clone, Default)]
pub struct Session {
    queue: Vec<Planned>,
    pos: usize,
    retried: HashSet<String>,
    pub outcomes: Vec<Outcome>,
    /// One per grade, newest last, so [`Self::undo`] can put it all back.
    steps: Vec<Step>,
}

/// What one grade changed, to take it back exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Step {
    /// The position before the grade (hidden items it skipped included).
    pos: usize,
    /// Where the grade queued its Again retry, if it did.
    retry_at: Option<usize>,
    /// The review event the grade logged, for the store to take back.
    event: Option<String>,
}

/// A grade [`Session::undo`] took back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Undone {
    pub outcome: Outcome,
    /// The logged review event to retract (`StudyStore::retract`).
    pub event: Option<String>,
}

impl Session {
    pub fn new(plan: Vec<Planned>) -> Self {
        Self { queue: plan, ..Self::default() }
    }

    pub fn current(&self) -> Option<&Planned> {
        self.queue.get(self.pos)
    }

    /// 0-based position and the current queue length (it grows on Again).
    pub fn progress(&self) -> (usize, usize) {
        (self.pos, self.queue.len())
    }

    pub fn is_done(&self) -> bool {
        self.pos >= self.queue.len()
    }

    /// Whether the current item is a same-session retry.
    pub fn is_retry(&self) -> bool {
        self.current().is_some_and(|p| self.queue[..self.pos].iter().any(|q| q.item_id == p.item_id))
    }

    pub fn grade(&mut self, grade: Grade) {
        self.grade_unless(grade, |_| false);
    }

    /// Grades the current item, then skips every item `hidden` says he
    /// marked ✗ (here or by sync) from what's left (#246). A hidden item is
    /// not queued for a retry either.
    pub fn grade_unless(&mut self, grade: Grade, hidden: impl Fn(&str) -> bool) {
        self.grade_logged(grade, None, hidden);
    }

    /// [`Self::grade_unless`], remembering the review event the grade was
    /// logged as, so [`Self::undo`] can hand it back for retraction.
    pub fn grade_logged(&mut self, grade: Grade, event: Option<String>, hidden: impl Fn(&str) -> bool) {
        let Some(p) = self.current().cloned() else { return };
        let retry = self.is_retry();
        let mut step = Step { pos: self.pos, retry_at: None, event };
        self.outcomes.push(Outcome { item_id: p.item_id.clone(), grade, fresh: p.fresh, retry });
        if grade == Grade::Again && !hidden(&p.item_id) && self.retried.insert(p.item_id.clone()) {
            step.retry_at = Some(self.queue.len());
            self.queue.push(Planned { fresh: false, ..p });
        }
        self.steps.push(step);
        self.pos += 1;
        while self.current().is_some_and(|q| hidden(&q.item_id)) {
            self.pos += 1;
        }
    }

    /// Whether there is a grade [`Self::undo`] can take back.
    pub fn can_undo(&self) -> bool {
        !self.steps.is_empty()
    }

    /// Takes back the newest grade: the item is current again, its outcome
    /// and its Again retry are gone, and the event it was logged as comes
    /// back for the store to retract. Grades before it stay; calling it again
    /// takes those back one by one.
    pub fn undo(&mut self) -> Option<Undone> {
        let step = self.steps.pop()?;
        let outcome = self.outcomes.pop()?;
        if let Some(at) = step.retry_at {
            if at < self.queue.len() {
                self.queue.remove(at);
            }
            self.retried.remove(&outcome.item_id);
        }
        self.pos = step.pos;
        Some(Undone { outcome, event: step.event })
    }

    /// Queues more items after what's left, skipping any still to come, so
    /// "study this page" during a session adds to it instead of replacing it.
    /// Returns how many were added.
    pub fn append(&mut self, more: Vec<Planned>) -> usize {
        let mut added = 0;
        for p in more {
            if !self.queue[self.pos..].iter().any(|q| q.item_id == p.item_id) {
                self.queue.push(p);
                added += 1;
            }
        }
        added
    }

    pub fn summary(&self) -> Summary {
        // The first grade of each item is the one that counts for "right".
        let mut first: Vec<&Outcome> = Vec::new();
        for o in &self.outcomes {
            if !first.iter().any(|f| f.item_id == o.item_id) {
                first.push(o);
            }
        }
        let right = first.iter().filter(|o| matches!(o.grade, Grade::Good | Grade::Easy)).count();
        let shaky: Vec<String> = first
            .iter()
            .filter(|o| matches!(o.grade, Grade::Again | Grade::Hard))
            .map(|o| o.item_id.clone())
            .collect();
        Summary { items: first.len(), right, fresh: first.iter().filter(|o| o.fresh).count(), shaky }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// Distinct items answered.
    pub items: usize,
    /// Answered Good or Easy the first time.
    pub right: usize,
    /// Seen for the first time.
    pub fresh: usize,
    /// Graded Again or Hard the first time: these come back sooner.
    pub shaky: Vec<String>,
}

// ---------------------------------------------------------------------------
// Typed answers

/// How close a typed answer is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    Right,
    /// The numbers are right but a sign (≥, <) is missing, or a word has a
    /// small typo. Shown with the full answer.
    Close,
    Wrong,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Op(&'static str),
    Word(String),
}

/// Lower-cases, maps Arabic-Indic digits and look-alike symbols to ASCII, and
/// splits into numbers, comparison signs and words. Spacing, dashes and other
/// punctuation are dropped, so "5 - 6", "5-6" and "5 to 6" read the same.
fn tokens(s: &str) -> Vec<Tok> {
    let mut norm = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '٠'..='٩' => norm.push(char::from(b'0' + (c as u32 - '٠' as u32) as u8)),
            '۰'..='۹' => norm.push(char::from(b'0' + (c as u32 - '۰' as u32) as u8)),
            '٫' => norm.push('.'),
            '≥' | '⩾' => norm.push_str(">="),
            '≤' | '⩽' => norm.push_str("<="),
            '×' => norm.push('x'),
            '−' | '–' | '—' => norm.push('-'),
            'µ' | 'μ' => norm.push('u'),
            _ => norm.extend(c.to_lowercase()),
        }
    }
    let chars: Vec<char> = norm.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    // A dash before a number is a minus sign unless it joins a range ("5-6");
    // after "to" or a comparison sign it is a minus again ("-2.6 to -1.4").
    let mut after_to = false;
    while i < chars.len() {
        let c = chars[i];
        let digit_at = |j: usize| {
            chars.get(j).is_some_and(|d| d.is_ascii_digit())
                || (chars.get(j) == Some(&'.') && chars.get(j + 1).is_some_and(|d| d.is_ascii_digit()))
        };
        if c == '-' && digit_at(i + 1) && (after_to || !matches!(out.last(), Some(Tok::Num(_)))) {
            i += 1;
            let start = i;
            while i < chars.len() {
                let d = chars[i];
                let sep_then_digit =
                    (d == '.' || d == ',') && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit());
                if d.is_ascii_digit() || sep_then_digit {
                    i += 1;
                } else {
                    break;
                }
            }
            let raw: String = chars[start..i].iter().collect();
            out.push(Tok::Num(-parse_number(&raw)));
            after_to = false;
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            after_to = false;
            let start = i;
            while i < chars.len() {
                let d = chars[i];
                let sep_then_digit =
                    (d == '.' || d == ',') && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit());
                if d.is_ascii_digit() || sep_then_digit {
                    i += 1;
                } else {
                    break;
                }
            }
            let raw: String = chars[start..i].iter().collect();
            out.push(Tok::Num(parse_number(&raw)));
        } else if c == '>' || c == '<' {
            let eq = chars.get(i + 1) == Some(&'=');
            out.push(Tok::Op(match (c, eq) {
                ('>', true) => ">=",
                ('>', false) => ">",
                ('<', true) => "<=",
                _ => "<",
            }));
            i += if eq { 2 } else { 1 };
            after_to = true;
        } else if c.is_alphanumeric() {
            let start = i;
            while i < chars.len() && chars[i].is_alphanumeric() && !chars[i].is_ascii_digit() {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if word == "to" {
                after_to = true;
            } else {
                after_to = false;
                out.push(Tok::Word(us_spelling(word)));
            }
        } else {
            i += 1;
        }
    }
    out
}

/// `1,000` and `1,000,000` are thousands, `1,5` is one and a half, `2.5` is
/// two and a half, `1,000.5` is a thousand and a half.
fn parse_number(raw: &str) -> f64 {
    let int_part = raw.split('.').next().unwrap_or(raw);
    let thousands = int_part.contains(',') && int_part.split(',').skip(1).all(|g| g.len() == 3);
    let cleaned = if thousands || raw.contains('.') {
        raw.replace(',', "")
    } else {
        raw.replacen(',', ".", 1).replace(',', "")
    };
    cleaned.parse().unwrap_or(f64::NAN)
}

/// Unit words and their spellings. Typed units are optional, but a different
/// unit is a different answer ("7 weeks" for "7 days").
const UNITS: &[(&str, &[&str])] = &[
    ("sec", &["s", "sec", "secs", "second", "seconds"]),
    ("min", &["min", "mins", "minute", "minutes"]),
    ("hour", &["h", "hr", "hrs", "hour", "hours"]),
    ("day", &["d", "day", "days"]),
    ("week", &["wk", "wks", "week", "weeks"]),
    ("month", &["mo", "mos", "month", "months"]),
    ("year", &["y", "yr", "yrs", "year", "years"]),
    ("kg", &["kg"]),
    ("g", &["g", "gm", "gram", "grams"]),
    ("mg", &["mg"]),
    ("ug", &["ug", "mcg"]),
    ("ng", &["ng"]),
    ("l", &["l", "liter", "litre", "liters", "litres"]),
    ("dl", &["dl"]),
    ("ml", &["ml"]),
    ("mmol", &["mmol"]),
    ("umol", &["umol"]),
    ("meq", &["meq"]),
    ("iu", &["iu"]),
    ("mmhg", &["mmhg"]),
    ("cmh", &["cmh"]),
    ("cm", &["cm"]),
    ("mm", &["mm"]),
    ("copies", &["copies", "copy"]),
];

fn unit_of(word: &str) -> Option<&'static str> {
    UNITS.iter().find(|(_, spellings)| spellings.contains(&word)).map(|(unit, _)| *unit)
}

/// British spellings mapped to American ones, so both read the same
/// (haemolysis = hemolysis, oesophagus = esophagus, anaemia = anemia).
const SPELLINGS: &[(&str, &str)] = &[
    ("haem", "hem"),
    ("aem", "em"),
    ("oesoph", "esoph"),
    ("oedem", "edem"),
    ("oestr", "estr"),
    ("paed", "ped"),
    ("foet", "fet"),
    ("coeli", "celi"),
    ("isation", "ization"),
    ("ise", "ize"),
    ("our", "or"),
];

fn us_spelling(word: String) -> String {
    let mut w = word;
    for (uk, us) in SPELLINGS {
        // "our" and "ise" only as endings (tumour, localise), not "four", "wise".
        if matches!(*uk, "our" | "ise") {
            if w.len() >= uk.len() + 3 && w.ends_with(uk) {
                w.truncate(w.len() - uk.len());
                w.push_str(us);
            }
        } else if w.contains(uk) {
            w = w.replace(uk, us);
        }
    }
    w
}

/// Opposite prefixes: one edit apart, opposite meanings. A typed word that
/// swaps one for another is wrong, not a typo. Each group lists prefixes that
/// stand for each other on the same stem; nothing else counts as a swap.
const CONTRASTS: &[&[&str]] = &[
    &["hypo", "hyper", "normo", "eu"],
    &["intra", "extra", "inter"],
    &["pre", "post"],
    &["ante", "retro"],
    &["brady", "tachy"],
    &["micro", "macro"],
    &["endo", "exo"],
    &["ab", "ad"],
    &["af", "ef"],
    &["in", "ex"],
    &["sub", "supra"],
    &["mono", "poly", "oligo"],
];

/// Whether two words are the same stem behind opposite prefixes
/// (hypokalaemia vs hyperkalaemia, abduct vs adduct). The stems must be
/// identical, so a typo or an unrelated word never counts as a swap.
fn contrasting(a: &str, b: &str) -> bool {
    CONTRASTS.iter().any(|group| {
        group.iter().any(|pa| {
            group.iter().any(|pb| {
                pa != pb
                    && a.strip_prefix(pa)
                        .zip(b.strip_prefix(pb))
                        .is_some_and(|(sa, sb)| sa.len() >= 3 && sa == sb)
            })
        })
    })
}

/// Checks a typed cloze answer against the expected one.
///
/// With numbers: the numbers must match in order; units and spacing don't
/// matter; a missing comparison sign is Close, a different one is Wrong.
/// Without numbers: the words must match, ignoring case and punctuation; a
/// small typo is Close.
pub fn check_typed(expected: &str, typed: &str) -> Match {
    let whole = check_one(expected, typed);
    if whole != Match::Wrong {
        return whole;
    }
    // A list answer ("vitamin A and retinoids", "ALT or AST"): every listed
    // part typed, in any order, is Right; some of an "and" list is Close; any
    // one of an "or" list is as good as that part.
    let (exp_parts, either) = list_parts(expected);
    if exp_parts.len() < 2 || exp_parts.iter().any(|p| p.chars().any(|c| c.is_ascii_digit())) {
        return whole;
    }
    let (got_parts, _) = list_parts(typed);
    let mut covered = vec![Match::Wrong; exp_parts.len()];
    for g in &got_parts {
        let best =
            exp_parts.iter().enumerate().map(|(i, e)| (i, check_one(e, g))).min_by_key(|(_, m)| match m {
                Match::Right => 0,
                Match::Close => 1,
                Match::Wrong => 2,
            });
        match best {
            Some((i, m)) if m != Match::Wrong => {
                if covered[i] != Match::Right {
                    covered[i] = m;
                }
            }
            // Something typed that is not on the list.
            _ => return Match::Wrong,
        }
    }
    let hits: Vec<Match> = covered.into_iter().filter(|m| *m != Match::Wrong).collect();
    if hits.is_empty() {
        Match::Wrong
    } else if (either || hits.len() == exp_parts.len()) && hits.iter().all(|m| *m == Match::Right) {
        Match::Right
    } else {
        Match::Close
    }
}

/// Splits "a, b and c" / "a or b" / "a/b" into its parts, and says whether
/// they are alternatives ("or", "/").
fn list_parts(s: &str) -> (Vec<String>, bool) {
    let lower = s.to_lowercase();
    let either = lower.contains(" or ") || lower.contains('/');
    let parts = lower
        .replace(" and ", ",")
        .replace(" or ", ",")
        .replace(['/', ';', '&'], ",")
        .split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    (parts, either)
}

fn check_one(expected: &str, typed: &str) -> Match {
    let exp = tokens(expected);
    let got = tokens(typed);
    if got.is_empty() {
        return Match::Wrong;
    }
    let nums = |t: &[Tok]| -> Vec<f64> {
        t.iter().filter_map(|x| if let Tok::Num(n) = x { Some(*n) } else { None }).collect()
    };
    let ops = |t: &[Tok]| -> Vec<&'static str> {
        t.iter().filter_map(|x| if let Tok::Op(o) = x { Some(*o) } else { None }).collect()
    };
    let words = |t: &[Tok]| -> Vec<String> {
        t.iter().filter_map(|x| if let Tok::Word(w) = x { Some(w.clone()) } else { None }).collect()
    };

    let exp_nums = nums(&exp);
    if !exp_nums.is_empty() {
        let got_nums = nums(&got);
        let same = got_nums.len() == exp_nums.len()
            && got_nums.iter().zip(&exp_nums).all(|(a, b)| (a - b).abs() <= 1e-9 * b.abs().max(1.0));
        if !same {
            return Match::Wrong;
        }
        let units = |t: &[Tok]| -> Vec<&'static str> {
            t.iter().filter_map(|x| if let Tok::Word(w) = x { unit_of(w) } else { None }).collect()
        };
        let got_units = units(&got);
        if !got_units.is_empty() && got_units != units(&exp) {
            return Match::Wrong;
        }
        let (exp_ops, got_ops) = (ops(&exp), ops(&got));
        return if got_ops == exp_ops {
            Match::Right
        } else if got_ops.is_empty() {
            Match::Close
        } else {
            Match::Wrong
        };
    }

    let (exp_words, got_words) = (words(&exp), words(&got));
    if exp_words == got_words {
        return Match::Right;
    }
    let (a, b) = (exp_words.concat(), got_words.concat());
    if a == b {
        return Match::Right;
    }
    let swapped = if exp_words.len() == got_words.len() {
        exp_words.iter().zip(&got_words).any(|(e, g)| contrasting(e, g))
    } else {
        contrasting(&a, &b)
    };
    if swapped {
        return Match::Wrong;
    }
    let len = a.chars().count();
    let allowed = if len >= 9 {
        2
    } else if len >= 5 {
        1
    } else {
        0
    };
    if allowed > 0 && edit_distance(&a, &b) <= allowed { Match::Close } else { Match::Wrong }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != cb)).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The grade a cloze attempt suggests: all right is Good, near misses Hard,
/// anything wrong or revealed Again. The learner can still pick another.
pub fn suggest_cloze(results: &[Match]) -> Grade {
    if results.contains(&Match::Wrong) {
        Grade::Again
    } else if results.contains(&Match::Close) {
        Grade::Hard
    } else {
        Grade::Good
    }
}

const STOP: &[&str] =
    &["the", "and", "of", "in", "with", "for", "a", "an", "or", "on", "by", "to", "from", "its"];

fn key_words(s: &str) -> Vec<String> {
    tokens(s)
        .into_iter()
        .filter_map(|t| match t {
            Tok::Word(w) if w.chars().count() >= 2 && !STOP.contains(&w.as_str()) => Some(w),
            Tok::Num(n) => Some(n.to_string()),
            _ => None,
        })
        .collect()
}

/// Words that name a kind of thing rather than the thing ("syndrome",
/// "hypertension") or qualify it ("recurrent"). A point is only recalled when
/// its most specific word is.
const GENERIC: &[&str] = &[
    "syndrome",
    "disease",
    "disorder",
    "failure",
    "injury",
    "hypertension",
    "infection",
    "infections",
    "deficiency",
    "cancer",
    "carcinoma",
    "tumor",
    "tumour",
    "test",
    "level",
    "levels",
    "cell",
    "cells",
    "type",
    "liver",
    "hepatic",
    "acute",
    "chronic",
    "primary",
    "secondary",
    "recurrent",
    "refractory",
    "severe",
    "mild",
    "early",
    "late",
    "high",
    "low",
    "increased",
    "decreased",
    "raised",
    "reduced",
    "elevated",
    "risk",
    "factor",
    "factors",
];

/// A typed word covers a wanted one when they match, or share a stem that
/// is most of the wanted word ("bleed" for "bleeding", not "hepa" for
/// "hepatorenal").
fn word_hit(want: &str, have: &[String]) -> bool {
    let want_len = want.chars().count();
    have.iter().any(|h| {
        h == want || {
            let common = want.chars().zip(h.chars()).take_while(|(a, b)| a == b).count();
            common >= 4 && common * 5 >= want_len * 3
        }
    })
}

/// Which points a free-typed recall already covers, to pre-tick them. A point
/// counts when at least half of its key words appear somewhere in what was
/// typed, its most specific word among them ("cardiomyopathy" covers
/// "Cirrhotic cardiomyopathy"; "pulmonary hypertension" doesn't cover
/// "Portal hypertension").
pub fn recalled(points: &[String], typed: &str) -> Vec<bool> {
    let have = key_words(typed);
    points
        .iter()
        .map(|p| {
            let want = key_words(p);
            if want.is_empty() || have.is_empty() {
                return false;
            }
            let hits = want.iter().filter(|w| word_hit(w, &have)).count();
            let key =
                want.iter().filter(|w| !GENERIC.contains(&w.as_str())).max_by_key(|w| w.chars().count());
            hits * 2 >= want.len() && key.is_none_or(|k| word_hit(k, &have))
        })
        .collect()
}

/// The headings of an essay outline with their bullet points.
pub fn outline_sections(outline: &[OutlineEntry]) -> Vec<(String, Vec<String>)> {
    outline
        .iter()
        .flat_map(|e| match e {
            OutlineEntry::Heading(h) => vec![(h.clone(), Vec::new())],
            OutlineEntry::Section(map) => map.iter().map(|(h, b)| (h.clone(), b.clone())).collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Lecture;

    fn sample() -> Lecture {
        serde_yaml_ng::from_str(include_str!("../../../fixtures/content/sample.yaml")).unwrap()
    }

    const STUDY: &[&str] = &["card", "cloze", "enumerate", "essay"];

    fn compose_with(lecture: &Lecture, due: &[&str], seen: &[&str], budget: u32) -> Vec<Planned> {
        let lectures = [lecture];
        let due: Vec<String> = due.iter().map(|s| s.to_string()).collect();
        let seen: HashSet<String> = seen.iter().map(|s| s.to_string()).collect();
        let empty = HashSet::new();
        compose(&Inputs {
            lectures: &lectures,
            topics: &empty,
            kinds: STUDY,
            due: &due,
            seen: &seen,
            hidden: &empty,
            max_new: usize::MAX,
            budget_secs: budget,
        })
    }

    fn ids(plan: &[Planned]) -> Vec<&str> {
        plan.iter().map(|p| p.item_id.as_str()).collect()
    }

    #[test]
    fn new_items_follow_book_order_and_only_playable_kinds() {
        let plan = compose_with(&sample(), &[], &[], 600);
        assert_eq!(ids(&plan), ["hb.L01.001", "hb.L01.002", "hb.L01.004", "hb.L01.005"]);
        assert!(plan.iter().all(|p| p.fresh));
    }

    #[test]
    fn due_reviews_come_first() {
        let plan = compose_with(&sample(), &["hb.L01.005", "hb.L01.002"], &["hb.L01.002", "hb.L01.005"], 600);
        assert_eq!(&ids(&plan)[..2], ["hb.L01.005", "hb.L01.002"]);
        assert!(!plan[0].fresh && !plan[1].fresh);
        assert_eq!(&ids(&plan)[2..], ["hb.L01.001", "hb.L01.004"]);
    }

    #[test]
    fn due_reviews_interleave_topics() {
        let mut lecture = sample();
        // Two more child-pugh clozes, so three of the same topic are due together.
        for n in ["101", "102"] {
            let mut extra = lecture.items[1].clone();
            extra.id = format!("hb.L01.{n}");
            lecture.items.push(extra);
        }
        let due = ["hb.L01.002", "hb.L01.101", "hb.L01.102", "hb.L01.001"];
        let plan = compose_with(&lecture, &due, &due, 600);
        let topics: Vec<&str> = plan.iter().take(4).map(|p| p.topic.as_str()).collect();
        assert_eq!(
            topics,
            ["hb.L01/child-pugh", "hb.L01/definition", "hb.L01/child-pugh", "hb.L01/child-pugh"]
        );
    }

    #[test]
    fn new_items_are_blocked_by_topic() {
        let mut lecture = sample();
        let mut late = lecture.items[0].clone();
        late.id = "hb.L01.200".into();
        lecture.items.push(late); // a "definition" card at the end of the file
        let plan = compose_with(&lecture, &[], &[], 6000);
        assert_eq!(&ids(&plan)[..2], ["hb.L01.001", "hb.L01.200"]);
    }

    #[test]
    fn budget_caps_the_session_but_never_empties_it() {
        let plan = compose_with(&sample(), &[], &[], 70);
        let total: u32 = plan.iter().map(|p| p.est_secs).sum();
        assert!(total <= 70, "{total}");
        assert_eq!(ids(&plan), ["hb.L01.001", "hb.L01.002"]);
        let tiny = compose_with(&sample(), &[], &[], 1);
        assert_eq!(tiny.len(), 1);
    }

    #[test]
    fn daily_new_cap_holds_back_new_items_not_reviews() {
        let lecture = sample();
        let lectures = [&lecture];
        let due = vec!["hb.L01.004".to_string()];
        let seen: HashSet<String> = due.iter().cloned().collect();
        let empty = HashSet::new();
        let plan = compose(&Inputs {
            lectures: &lectures,
            topics: &empty,
            kinds: STUDY,
            due: &due,
            seen: &seen,
            hidden: &empty,
            max_new: 1,
            budget_secs: 600,
        });
        assert_eq!(ids(&plan), ["hb.L01.004", "hb.L01.001"]);
    }

    #[test]
    fn rejected_hidden_and_filtered_topics_stay_out() {
        let mut lecture = sample();
        lecture.items[0].status = Status::Rejected;
        let lectures = [&lecture];
        let topics: HashSet<String> = ["hb.L01/child-pugh".to_string(), "hb.L01/presentation".into()].into();
        let hidden: HashSet<String> = ["hb.L01.004".to_string()].into();
        let empty = HashSet::new();
        let plan = compose(&Inputs {
            lectures: &lectures,
            topics: &topics,
            kinds: STUDY,
            due: &[],
            seen: &empty,
            hidden: &hidden,
            max_new: usize::MAX,
            budget_secs: 600,
        });
        assert_eq!(ids(&plan), ["hb.L01.002"]);
    }

    #[test]
    fn hidden_items_are_skipped_and_never_retried() {
        let plan = compose_with(&sample(), &[], &[], 600);
        let mut s = Session::new(plan);
        // He marks 001 ✗ and grades it Again; 002 arrives ✗ by sync.
        let hidden = |id: &str| matches!(id, "hb.L01.001" | "hb.L01.002");
        s.grade_unless(Grade::Again, hidden);
        assert_ne!(s.current().unwrap().item_id, "hb.L01.002");
        while !s.is_done() {
            assert!(!hidden(&s.current().unwrap().item_id));
            s.grade_unless(Grade::Good, hidden);
        }
        assert_eq!(s.outcomes.iter().filter(|o| o.item_id == "hb.L01.001").count(), 1);
    }

    #[test]
    fn again_comes_back_once_at_the_end() {
        let plan = compose_with(&sample(), &[], &[], 600);
        let mut s = Session::new(plan);
        s.grade(Grade::Again); // 001
        s.grade(Grade::Good);
        s.grade(Grade::Hard);
        s.grade(Grade::Easy);
        assert_eq!(s.current().unwrap().item_id, "hb.L01.001");
        assert!(s.is_retry());
        s.grade(Grade::Again); // no second retry
        assert!(s.is_done());
        let sum = s.summary();
        assert_eq!(sum.items, 4);
        assert_eq!(sum.right, 2);
        assert_eq!(sum.fresh, 4);
        assert_eq!(sum.shaky, ["hb.L01.001", "hb.L01.004"]);
    }

    #[test]
    fn typed_numbers_ignore_units_and_spacing() {
        assert_eq!(check_typed("5-6", "5 - 6"), Match::Right);
        assert_eq!(check_typed("5-6", "5 to 6"), Match::Right);
        assert_eq!(check_typed("5-6", "٥-٦"), Match::Right);
        assert_eq!(check_typed("≥250 cells/mm³", ">= 250"), Match::Right);
        assert_eq!(check_typed("≥250", "250"), Match::Close);
        assert_eq!(check_typed("≥250", "<250"), Match::Wrong);
        assert_eq!(check_typed("1.1 g/dL", "1,1"), Match::Right);
        assert_eq!(check_typed("1,000", "1000"), Match::Right);
        assert_eq!(check_typed("10-15", "10-16"), Match::Wrong);
        assert_eq!(check_typed("10-15", ""), Match::Wrong);
    }

    #[test]
    fn typed_words_allow_case_and_small_typos() {
        assert_eq!(check_typed("SBP", "sbp"), Match::Right);
        assert_eq!(check_typed("Hepatopulmonary syndrome", "hepatopulmonary-syndrome"), Match::Right);
        assert_eq!(check_typed("Terlipressin", "terlipresin"), Match::Close);
        assert_eq!(check_typed("TIPS", "TIPSS"), Match::Wrong);
        assert_eq!(check_typed("Propranolol", "Carvedilol"), Match::Wrong);
    }

    #[test]
    fn opposite_prefixes_are_wrong_not_typos() {
        assert_eq!(check_typed("Hypokalaemia", "hyperkalaemia"), Match::Wrong);
        assert_eq!(check_typed("Hypothyroidism", "hyperthyroidism"), Match::Wrong);
        assert_eq!(check_typed("Intrahepatic", "extrahepatic"), Match::Wrong);
        assert_eq!(check_typed("Intrahepatic", "interhepatic"), Match::Wrong);
        assert_eq!(check_typed("Bradycardia", "tachycardia"), Match::Wrong);
        assert_eq!(check_typed("Microcytic", "macrocytic"), Match::Wrong);
        assert_eq!(check_typed("Afferent", "efferent"), Match::Wrong);
        assert_eq!(check_typed("Prehepatic", "posthepatic"), Match::Wrong);
        assert_eq!(check_typed("Primary hypoaldosteronism", "primary hyperaldosteronism"), Match::Wrong);
        assert_eq!(check_typed("Abduction", "adduction"), Match::Wrong);
        // British and American spellings are the same answer.
        assert_eq!(check_typed("Hypokalaemia", "hypokalemia"), Match::Right);
        assert_eq!(check_typed("Haemolysis", "hemolysis"), Match::Right);
        assert_eq!(check_typed("Oesophageal varices", "esophageal varices"), Match::Right);
        assert_eq!(check_typed("Haemochromatosis", "hemochromatosis"), Match::Right);
        assert_eq!(check_typed("Tumour", "tumor"), Match::Right);
        // Shared first letters are not a prefix swap: typos stay close.
        assert_eq!(check_typed("Hemolysis", "hemolisis"), Match::Close);
        assert_eq!(check_typed("Abdominal", "adbominal"), Match::Close);
        assert_eq!(check_typed("Infection", "infaction"), Match::Close);
        assert_eq!(check_typed("Hyponatremia", "hyponatreima"), Match::Close);
    }

    #[test]
    fn numbers_keep_units_signs_and_thousands() {
        assert_eq!(check_typed("7 days", "7 weeks"), Match::Wrong);
        assert_eq!(check_typed("7 days", "7 d"), Match::Right);
        assert_eq!(check_typed("7 days", "7"), Match::Right);
        assert_eq!(check_typed("1.5 mg/dL", "1.5 mmol/L"), Match::Wrong);
        assert_eq!(check_typed("−2.60", "2.60"), Match::Wrong);
        assert_eq!(check_typed("−2.60", "-2.6"), Match::Right);
        assert_eq!(check_typed("≤ −2.60", "<=-2.60"), Match::Right);
        assert_eq!(check_typed("−2.60 to −1.39", "-2.6 to -1.39"), Match::Right);
        assert_eq!(check_typed("5-6", "5 - 6"), Match::Right);
        assert_eq!(check_typed("1,000,000", "1000000"), Match::Right);
        assert_eq!(check_typed("10,000,000 copies/mL", "10000000"), Match::Right);
        assert_eq!(check_typed("1,000.5", "1000.5"), Match::Right);
        assert_eq!(check_typed("1,5", "1.5"), Match::Right);
    }

    #[test]
    fn undo_restores_the_session_exactly() {
        let plan = compose_with(&sample(), &[], &[], 600);
        let mut s = Session::new(plan);
        let start = (s.progress(), s.current().cloned());
        s.grade_logged(Grade::Again, Some("e1".into()), |_| false);
        s.grade_logged(Grade::Good, Some("e2".into()), |_| false);
        let undone = s.undo().unwrap();
        assert_eq!(undone.event.as_deref(), Some("e2"));
        assert_eq!(undone.outcome.grade, Grade::Good);
        let undone = s.undo().unwrap();
        assert_eq!(undone.event.as_deref(), Some("e1"));
        // The Again retry is gone with it, and the first item is current again.
        assert_eq!((s.progress(), s.current().cloned()), start);
        assert!(s.outcomes.is_empty());
        assert!(!s.can_undo());
        assert!(s.undo().is_none());
        // Grading it Again afterwards still earns its one retry.
        s.grade(Grade::Again);
        assert_eq!(s.progress().1, start.0.1 + 1);
    }

    #[test]
    fn undo_after_the_last_item_reopens_it_and_keeps_appended_items() {
        let p = |id: &str| Planned {
            item_id: id.into(),
            lecture: "L".into(),
            topic: "L".into(),
            est_secs: 10,
            fresh: true,
        };
        let mut s = Session::new(vec![p("a"), p("b")]);
        s.grade(Grade::Again); // a: retry queued at the end
        s.append(vec![p("c")]);
        s.grade(Grade::Good); // b
        s.grade(Grade::Good); // a (retry)
        s.grade(Grade::Good); // c
        assert!(s.is_done());
        s.undo();
        assert_eq!(s.current().unwrap().item_id, "c");
        s.undo();
        s.undo();
        s.undo();
        // Back at a, its retry gone, c still queued after b.
        let mut seen = Vec::new();
        while let Some(c) = s.current() {
            seen.push(c.item_id.clone());
            s.grade(Grade::Good);
        }
        assert_eq!(seen, ["a", "b", "c"]);
        assert_eq!(s.summary().items, 3);
    }

    #[test]
    fn undo_puts_back_hidden_items_it_skipped() {
        let p = |id: &str| Planned {
            item_id: id.into(),
            lecture: "L".into(),
            topic: "L".into(),
            est_secs: 10,
            fresh: true,
        };
        let mut s = Session::new(vec![p("a"), p("b"), p("c")]);
        s.grade_unless(Grade::Good, |id| id == "b");
        assert_eq!(s.current().unwrap().item_id, "c");
        s.undo();
        assert_eq!(s.current().unwrap().item_id, "a");
    }

    #[test]
    fn append_queues_after_whats_left_without_repeats() {
        let p = |id: &str| Planned {
            item_id: id.into(),
            lecture: "L".into(),
            topic: "L".into(),
            est_secs: 10,
            fresh: true,
        };
        let mut s = Session::new(vec![p("a"), p("b")]);
        s.grade(Grade::Good);
        assert_eq!(s.append(vec![p("b"), p("c"), p("a")]), 2);
        let mut seen = Vec::new();
        while let Some(c) = s.current() {
            seen.push(c.item_id.clone());
            s.grade(Grade::Good);
        }
        assert_eq!(seen, ["b", "c", "a"]);
    }

    #[test]
    fn part_of_a_list_answer_is_close() {
        let e = "vitamin A and retinoids";
        assert_eq!(check_typed(e, "vitamin A"), Match::Close);
        assert_eq!(check_typed(e, "retinoids"), Match::Close);
        assert_eq!(check_typed(e, "retinoids and vitamin A"), Match::Right);
        assert_eq!(check_typed(e, "vitamin A, iron"), Match::Wrong);
        assert_eq!(check_typed(e, "glycogen"), Match::Wrong);
        assert_eq!(check_typed("ALT or AST", "AST"), Match::Right);
        assert_eq!(check_typed("hyperkalaemia and acidosis", "hypokalaemia"), Match::Wrong);
    }

    #[test]
    fn recall_needs_more_than_a_shared_stem() {
        let points: Vec<String> = ["Hepatorenal syndrome", "Hepatopulmonary syndrome", "Portal hypertension"]
            .map(String::from)
            .to_vec();
        assert_eq!(recalled(&points, "hepatorenal syndrome\npulmonary hypertension"), [true, false, false]);
        assert_eq!(recalled(&points, "hepa"), [false, false, false]);
        assert_eq!(recalled(&points, "HRS, HPS\nportal htn"), [false, false, true]);
    }

    #[test]
    fn grade_suggestions() {
        assert_eq!(suggest_cloze(&[Match::Right, Match::Right]), Grade::Good);
        assert_eq!(suggest_cloze(&[Match::Right, Match::Close]), Grade::Hard);
        assert_eq!(suggest_cloze(&[Match::Close, Match::Wrong]), Grade::Again);
    }

    #[test]
    fn free_recall_pre_ticks_matching_points() {
        let points: Vec<String> =
            ["Recurrent variceal bleeding", "Recurrent HE", "Refractory ascites", "SBP", "HRS-AKI", "ACLF"]
                .map(String::from)
                .to_vec();
        let typed = "variceal bleed again\nrefractory ascites\nsbp\nhrs aki";
        assert_eq!(recalled(&points, typed), [true, false, true, true, true, false]);
        assert_eq!(recalled(&points, ""), [false; 6]);
        let headings = ["Cirrhotic cardiomyopathy".to_string(), "Portopulmonary hypertension".into()];
        assert_eq!(recalled(&headings, "cardiomyopathy"), [true, false]);
    }

    #[test]
    fn outline_sections_flatten_headings() {
        let lecture = sample();
        let Body::Essay { outline, .. } = &lecture.items[4].body else { panic!() };
        let s = outline_sections(outline);
        assert_eq!(s.len(), 4);
        assert_eq!(s[0].0, "Hyperdynamic circulatory syndrome");
        assert_eq!(s[3].1.len(), 3);
    }
}
