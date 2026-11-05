//! The oral session itself, without UI: which items can be asked aloud, how
//! each one is phrased, and the order they come in (Again comes back later in
//! the same session; an examiner's follow-up comes straight after a card).

use std::collections::{BTreeSet, VecDeque};

use steps_core::content::{Body, Item, Lecture, OutlineEntry, Status};
use steps_core::review::Grade;

/// How many items later an Again comes back.
pub const AGAIN_GAP: usize = 3;
/// At most one essay follow-up per this many cards.
const FOLLOW_UP_EVERY: u32 = 3;

/// One question as the examiner asks it.
#[derive(Clone, Debug, PartialEq)]
pub struct Prompt {
    pub item_id: String,
    pub kind: &'static str,
    pub lecture: String,
    pub topic: Option<String>,
    /// Printed book page, to check the answer against.
    pub page: u32,
    /// The book's own words, shown when the page chip is hovered.
    pub quote: Option<String>,
    /// What is read aloud and shown, in English. A case has its stem first.
    pub question: Vec<String>,
    /// The model answer, one line per point or heading.
    pub answer: Vec<String>,
    /// "Enumerate": how many points count as a pass, of how many.
    pub pass: Option<(usize, usize)>,
    /// Ask "And what else?" once before the answer (enumerate).
    pub what_else: bool,
}

impl Prompt {
    /// The oral version of an item, if it has a short answer that can be said
    /// aloud: cards, enumerations, essay outlines and a case's first step.
    pub fn from_item(lecture: &str, item: &Item) -> Option<Prompt> {
        if item.status == Status::Rejected {
            return None;
        }
        let (question, answer, pass, what_else) = match &item.body {
            Body::Card { front, back } => (vec![front.clone()], vec![back.clone()], None, false),
            Body::Enumerate { prompt, points, pass, .. } => {
                (vec![prompt.clone()], points.clone(), Some((*pass, points.len())), points.len() > 1)
            }
            Body::Essay { prompt, outline } => (vec![prompt.clone()], outline_lines(outline), None, false),
            Body::Case { stem, steps } => {
                let step = steps.first()?;
                let mut question = vec![stem.clone()];
                question.extend(step.reveal.clone());
                question.push(step.question.clone());
                let mut answer: Vec<String> =
                    step.answer.and_then(|i| step.options.get(i)).cloned().into_iter().collect();
                answer.push(step.why.clone());
                (question, answer, None, false)
            }
            _ => return None,
        };
        if question.iter().all(|q| q.trim().is_empty()) || answer.is_empty() {
            return None;
        }
        Some(Prompt {
            item_id: item.id.clone(),
            kind: item.body.kind(),
            lecture: lecture.to_owned(),
            topic: item.topic.clone(),
            page: item.src.page,
            quote: item.src.quote.clone(),
            question,
            answer,
            pass,
            what_else,
        })
    }

    pub fn spoken_question(&self) -> String {
        speakable(&self.question.join(". "))
    }

    pub fn spoken_answer(&self) -> String {
        speakable(&self.answer.join(". "))
    }
}

/// Essay headings, each with its bullets after a colon.
fn outline_lines(outline: &[OutlineEntry]) -> Vec<String> {
    outline
        .iter()
        .flat_map(|entry| match entry {
            OutlineEntry::Heading(h) => vec![h.clone()],
            OutlineEntry::Section(map) => map
                .iter()
                .map(
                    |(h, bullets)| {
                        if bullets.is_empty() { h.clone() } else { format!("{h}: {}", bullets.join("; ")) }
                    },
                )
                .collect(),
        })
        .collect()
}

/// Symbols a voice would read badly, spelled out.
pub fn speakable(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '→' | '⇒' => out.push_str(" then "),
            '≥' => out.push_str(" at least "),
            '≤' => out.push_str(" at most "),
            '>' => out.push_str(" more than "),
            '<' => out.push_str(" less than "),
            '±' => out.push_str(" with or without "),
            '~' | '≈' => out.push_str(" about "),
            '↑' => out.push_str(" raised "),
            '↓' => out.push_str(" low "),
            '%' => out.push_str(" percent"),
            _ => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Which items go into a session.
#[derive(Clone, Debug, PartialEq)]
pub struct Pick {
    /// One lecture (`cvs.L04`), or every written lecture.
    pub lecture: Option<String>,
    /// Questions in the session (Agains come on top).
    pub size: usize,
    pub seed: u64,
    /// Items due for review: asked first, then the rest.
    pub due: BTreeSet<String>,
}

/// Every askable item of the chosen lectures.
#[cfg(test)]
pub fn pool<'a>(lectures: impl IntoIterator<Item = &'a Lecture>, lecture: Option<&str>) -> Vec<Prompt> {
    pool_with(lectures, lecture, |item| Some(item.clone()))
}

/// Like [`pool`], with each item passed through `prep` first: the review
/// store drops what Mohamed marked wrong (`None`) and applies his edits.
pub fn pool_with<'a>(
    lectures: impl IntoIterator<Item = &'a Lecture>,
    lecture: Option<&str>,
    prep: impl Fn(&Item) -> Option<Item>,
) -> Vec<Prompt> {
    lectures
        .into_iter()
        .filter(|l| lecture.is_none_or(|id| l.lecture == id))
        .flat_map(|l| {
            let prep = &prep;
            l.items.iter().filter_map(move |item| Prompt::from_item(&l.lecture, &prep(item)?))
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub prompt: Prompt,
    /// Asked as the examiner's follow-up to the previous card.
    pub follow_up: bool,
    /// Already asked once and rated Again.
    pub again: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tally {
    pub good: usize,
    pub again: usize,
    /// Heard without a rating (auto-continue while walking).
    pub heard: usize,
    /// Items rated Again at least once, in order.
    pub missed: Vec<Prompt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    queue: VecDeque<Entry>,
    /// Essays held back so a card can lead into them.
    essays: Vec<Prompt>,
    pub current: Option<Entry>,
    pub tally: Tally,
    pub total: usize,
    /// Counts every question asked, so the UI knows when a new one starts
    /// (even the same item again).
    pub asked: u64,
    cards_since_follow_up: u32,
}

impl Session {
    pub fn new(mut pool: Vec<Prompt>, pick: &Pick) -> Session {
        shuffle(&mut pool, pick.seed);
        // Stable, so due items and the rest each keep their shuffled order.
        pool.sort_by_key(|p| !pick.due.contains(&p.item_id));
        pool.truncate(pick.size.max(1));
        let total = pool.len();
        let essays = pool.iter().filter(|p| p.kind == "essay").cloned().collect();
        let queue = pool.into_iter().map(|prompt| Entry { prompt, follow_up: false, again: false }).collect();
        Session {
            queue,
            essays,
            current: None,
            tally: Tally::default(),
            total,
            asked: 0,
            cards_since_follow_up: 0,
        }
    }

    /// Moves to the next question. `None` when the session is over.
    pub fn advance(&mut self) -> Option<&Entry> {
        self.current = self.queue.pop_front();
        self.asked += u64::from(self.current.is_some());
        self.current.as_ref()
    }

    /// Stops early; the summary shows what was done.
    pub fn end(&mut self) {
        self.current = None;
        self.queue.clear();
    }

    pub fn left(&self) -> usize {
        self.queue.len() + usize::from(self.current.is_some())
    }

    /// Rates question number `turn` (its `asked` count) and moves on. A
    /// stale turn, such as a double tap landing after the session moved on,
    /// does nothing, so one question takes one rating. Returns what was rated.
    pub fn rate_turn(&mut self, turn: u64, grade: Option<Grade>) -> Option<Prompt> {
        if turn != self.asked {
            return None;
        }
        let prompt = self.current.as_ref()?.prompt.clone();
        self.rate(grade);
        self.advance();
        Some(prompt)
    }

    /// Rates the current question and lines up what comes after it.
    pub fn rate(&mut self, grade: Option<Grade>) {
        let Some(entry) = self.current.take() else { return };
        match grade {
            Some(Grade::Again) => {
                self.tally.again += 1;
                if !entry.again {
                    self.tally.missed.push(entry.prompt.clone());
                }
                let at = AGAIN_GAP.min(self.queue.len());
                self.queue.insert(at, Entry { again: true, follow_up: false, ..entry.clone() });
            }
            Some(_) => self.tally.good += 1,
            None => self.tally.heard += 1,
        }
        if entry.prompt.kind == "card" && !entry.again {
            self.cards_since_follow_up += 1;
            if self.cards_since_follow_up >= FOLLOW_UP_EVERY && self.follow_up(&entry.prompt) {
                self.cards_since_follow_up = 0;
            }
        }
    }

    /// Like an examiner: "Good. Now, how would you write about…?" Pulls the
    /// session's essay on the same topic to the front, once.
    fn follow_up(&mut self, card: &Prompt) -> bool {
        let Some(topic) = &card.topic else { return false };
        let Some(i) =
            self.essays.iter().position(|e| e.lecture == card.lecture && e.topic.as_ref() == Some(topic))
        else {
            return false;
        };
        let essay = self.essays.remove(i);
        let Some(at) = self.queue.iter().position(|e| e.prompt.item_id == essay.item_id && !e.again) else {
            return false;
        };
        let mut entry = self.queue.remove(at).expect("position is in range");
        entry.follow_up = true;
        self.queue.push_front(entry);
        true
    }
}

/// A small deterministic Fisher–Yates (xorshift64*), so a session needs no
/// random-number crate and tests can fix the order.
fn shuffle<T>(items: &mut [T], seed: u64) {
    let mut state = seed | 1;
    for i in (1..items.len()).rev() {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let r = state.wrapping_mul(0x2545_f491_4f6c_dd1d);
        items.swap(i, (r % (i as u64 + 1)) as usize);
    }
}

#[cfg(test)]
mod tests;
