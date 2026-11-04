//! The question bank and mock exams: which MCQs to ask, in what order, how
//! long the paper may take, and how it scored. Pure logic: the app passes in
//! the clock, the seed and the answer history.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::content::{Body, Lecture, Status};
use crate::review::{Grade, Origin, ReviewEvent};

/// 90 seconds per question in timed mode and in blocks, as on Step 1
/// (a one-hour block of up to 40 questions).
pub const MS_PER_QUESTION: u64 = 90_000;
/// The default block length: a full Step 1 block.
pub const MOCK_SIZE: usize = 40;

/// One MCQ from the library, with the lecture it came from.
#[derive(Debug, Clone, Copy)]
pub struct Mcq<'a> {
    pub lecture: &'a str,
    pub id: &'a str,
    pub topic: Option<&'a str>,
    pub tags: &'a [String],
    pub stem: &'a str,
    pub options: &'a [String],
    pub answer: usize,
}

/// Every askable MCQ in these lectures, in file order. Items Mohamed marked
/// wrong (`rejected`) never come up.
pub fn mcqs<'a>(lectures: impl IntoIterator<Item = &'a Lecture>) -> Vec<Mcq<'a>> {
    let mut out = Vec::new();
    for lecture in lectures {
        for item in &lecture.items {
            if item.status == Status::Rejected {
                continue;
            }
            if let Body::Mcq { stem, options, answer, .. } = &item.body {
                out.push(Mcq {
                    lecture: &lecture.lecture,
                    id: &item.id,
                    topic: item.topic.as_deref(),
                    tags: &item.tags,
                    stem,
                    options,
                    answer: *answer,
                });
            }
        }
    }
    out
}

/// The last result of each item answered in the bank, by item id.
pub type LastResults = BTreeMap<String, bool>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Only {
    #[default]
    All,
    /// Never answered, or wrong the last time.
    WrongOrUnseen,
}

/// What the bank screen asks for. An empty set means "any".
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Filter {
    pub lectures: BTreeSet<String>,
    /// `(lecture, topic)`: topic ids are only unique inside a lecture.
    pub topics: BTreeSet<(String, String)>,
    /// Any of these tags.
    pub tags: BTreeSet<String>,
    pub only: Only,
}

impl Filter {
    pub fn matches(&self, q: &Mcq, last: &LastResults) -> bool {
        let lecture_ok = self.lectures.is_empty() || self.lectures.contains(q.lecture);
        let topic_ok = self.topics.is_empty()
            || q.topic.is_some_and(|t| self.topics.contains(&(q.lecture.to_string(), t.to_string())));
        let tags_ok = self.tags.is_empty() || q.tags.iter().any(|t| self.tags.contains(t));
        let only_ok = match self.only {
            Only::All => true,
            Only::WrongOrUnseen => last.get(q.id) != Some(&true),
        };
        lecture_ok && topic_ok && tags_ok && only_ok
    }

    pub fn select<'a>(&self, pool: &[Mcq<'a>], last: &LastResults) -> Vec<Mcq<'a>> {
        pool.iter().filter(|q| self.matches(q, last)).copied().collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Explanation after every question, no clock.
    Tutor,
    /// A clock, and the review at the end.
    Timed,
    /// A timed paper mixed across lectures, scored per lecture and topic.
    Mock,
}

impl Mode {
    pub fn timed(self) -> bool {
        self != Self::Tutor
    }

    pub fn origin(self) -> Origin {
        match self {
            Self::Mock => Origin::Mock,
            _ => Origin::Bank,
        }
    }
}

/// One question on a paper. `order[shown] = original option index`, so the
/// options can be shuffled while `answer` keeps pointing at the original.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub lecture: String,
    pub topic: Option<String>,
    pub answer: usize,
    pub order: Vec<usize>,
}

impl Question {
    fn new(q: &Mcq, rng: &mut Rng) -> Self {
        let order = option_order(q.options, rng);
        Self {
            id: q.id.to_string(),
            lecture: q.lecture.to_string(),
            topic: q.topic.map(str::to_string),
            answer: q.answer,
            order,
        }
    }
}

/// The order to show `options` in: `order[shown] = original index`.
/// Numeric options go smallest first, options that point at other options
/// keep the book's order, and the rest are shuffled. The Anki export uses it
/// too, seeded per note.
pub fn option_order(options: &[String], rng: &mut Rng) -> Vec<usize> {
    let mut order: Vec<usize> = (0..options.len()).collect();
    if let Some(sorted) = numeric_order(options) {
        order = sorted;
    } else if shufflable(options) {
        rng.shuffle(&mut order);
    }
    order
}

/// Options that all start with a number ("250/mm³", "< 1.1 g/dL") read best
/// smallest first, as exam papers print them.
fn numeric_order(options: &[String]) -> Option<Vec<usize>> {
    let values: Vec<f64> = options.iter().map(|o| leading_number(o)).collect::<Option<_>>()?;
    let mut order: Vec<usize> = (0..options.len()).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    Some(order)
}

fn leading_number(o: &str) -> Option<f64> {
    let o = o.trim_start();
    // "Above 10 ng/ml", "Less than 2": the comparison word goes like `>`.
    let o = ["above ", "below ", "over ", "under ", "more than ", "less than "]
        .iter()
        .find_map(|w| o.get(..w.len()).filter(|p| p.eq_ignore_ascii_case(w)).map(|_| &o[w.len()..]))
        .unwrap_or(o);
    let o = o.trim_start_matches(|c: char| c.is_whitespace() || "<>≤≥=~".contains(c));
    let end = o.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',')).unwrap_or(o.len());
    o[..end].replace(',', "").parse().ok()
}

/// Options that point at other options ("All of the above", "Both A and B")
/// only make sense in the book's order.
pub fn shufflable(options: &[String]) -> bool {
    !options.iter().any(|o| {
        let o = o.trim().to_ascii_lowercase();
        o.contains("of the above")
            || o.contains("of these")
            || matches!(o.as_str(), "both" | "neither" | "none")
            || o.starts_with("both ")
            || o.starts_with("neither ")
            || refers_by_letter(&o)
    })
}

/// "A and C", "A, B or D": an option made only of option letters.
fn refers_by_letter(o: &str) -> bool {
    let words: Vec<&str> = o.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect();
    words.len() >= 2
        && words.iter().any(|w| matches!(*w, "and" | "or"))
        && words.iter().all(|w| matches!(*w, "a" | "b" | "c" | "d" | "e" | "and" | "or" | "only"))
}

/// A practice set from the bank: the filtered questions in random order,
/// at most `count` of them.
pub fn practice(selected: &[Mcq], count: usize, seed: u64) -> Vec<Question> {
    let mut rng = Rng::new(seed);
    let mut picked: Vec<&Mcq> = selected.iter().collect();
    rng.shuffle(&mut picked);
    picked.truncate(count);
    picked.into_iter().map(|q| Question::new(q, &mut rng)).collect()
}

/// A mock paper of `count` questions, spread across lectures in proportion
/// to how many MCQs each has, and across each lecture's topics in turn.
pub fn mock(pool: &[Mcq], count: usize, seed: u64) -> Vec<Question> {
    let mut rng = Rng::new(seed);
    let count = count.min(pool.len());
    let mut by_lecture: BTreeMap<&str, Vec<&Mcq>> = BTreeMap::new();
    for q in pool {
        by_lecture.entry(q.lecture).or_default().push(q);
    }
    let sizes: Vec<usize> = by_lecture.values().map(Vec::len).collect();
    let quotas = largest_remainder(&sizes, count);

    let mut picked = Vec::with_capacity(count);
    for (qs, quota) in by_lecture.into_values().zip(quotas) {
        // Shuffle, then deal round-robin across topics so one topic can't
        // fill a lecture's whole share.
        let mut by_topic: BTreeMap<Option<&str>, Vec<&Mcq>> = BTreeMap::new();
        for q in qs {
            by_topic.entry(q.topic).or_default().push(q);
        }
        let mut piles: Vec<Vec<&Mcq>> = by_topic.into_values().collect();
        for pile in &mut piles {
            rng.shuffle(pile);
        }
        rng.shuffle(&mut piles);
        let mut taken = 0;
        while taken < quota {
            for pile in &mut piles {
                if taken < quota
                    && let Some(q) = pile.pop()
                {
                    picked.push(q);
                    taken += 1;
                }
            }
        }
    }
    rng.shuffle(&mut picked);
    picked.into_iter().map(|q| Question::new(q, &mut rng)).collect()
}

/// Split `total` across groups in proportion to `sizes`, never more than a
/// group has.
fn largest_remainder(sizes: &[usize], total: usize) -> Vec<usize> {
    let sum: usize = sizes.iter().sum();
    if sum == 0 {
        return vec![0; sizes.len()];
    }
    let total = total.min(sum);
    let mut quotas: Vec<usize> = sizes.iter().map(|s| s * total / sum).collect();
    let mut rest: Vec<(usize, usize)> = sizes.iter().enumerate().map(|(i, s)| (s * total % sum, i)).collect();
    rest.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut left = total - quotas.iter().sum::<usize>();
    for (_, i) in rest.iter().cycle() {
        if left == 0 {
            break;
        }
        if quotas[*i] < sizes[*i] {
            quotas[*i] += 1;
            left -= 1;
        }
    }
    quotas
}

/// How long a timed paper of `n` questions may take.
pub fn time_limit_ms(n: usize) -> u64 {
    n as u64 * MS_PER_QUESTION
}

/// Time left on a timed paper; 0 once it is over.
pub fn remaining_ms(n: usize, started_ms: i64, now_ms: i64) -> u64 {
    let used = now_ms.saturating_sub(started_ms).max(0) as u64;
    time_limit_ms(n).saturating_sub(used)
}

/// What Mohamed did with one question.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    /// The original option index, not the shown position.
    pub chosen: Option<usize>,
    pub took_ms: u32,
    pub flagged: bool,
}

impl Answer {
    pub fn correct(&self, q: &Question) -> bool {
        self.chosen == Some(q.answer)
    }
}

/// An MCQ answered wrong is Again for that item, a right one is Good.
/// Questions left blank on a timed paper send nothing: he never got to them.
pub fn review_event(q: &Question, a: &Answer, mode: Mode, at_ms: i64) -> Option<ReviewEvent> {
    a.chosen?;
    Some(ReviewEvent {
        item_id: q.id.clone(),
        grade: if a.correct(q) { Grade::Good } else { Grade::Again },
        origin: mode.origin(),
        at_ms,
        took_ms: a.took_ms,
    })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Score {
    pub right: u32,
    pub total: u32,
}

impl Score {
    fn add(&mut self, right: bool) {
        self.total += 1;
        self.right += u32::from(right);
    }

    /// Whole percent, rounded half up.
    pub fn percent(self) -> u32 {
        (self.right * 200 + self.total).checked_div(self.total * 2).unwrap_or(0)
    }
}

/// A finished paper. Blank answers count as wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub mode: Mode,
    pub at_ms: i64,
    pub took_ms: u64,
    pub total: Score,
    pub unanswered: u32,
    pub by_lecture: BTreeMap<String, Score>,
    /// Keyed `lecture/topic`.
    pub by_topic: BTreeMap<String, Score>,
}

pub fn score(questions: &[Question], answers: &[Answer], mode: Mode, at_ms: i64, took_ms: u64) -> Outcome {
    let mut out = Outcome {
        mode,
        at_ms,
        took_ms,
        total: Score::default(),
        unanswered: 0,
        by_lecture: BTreeMap::new(),
        by_topic: BTreeMap::new(),
    };
    let blank = Answer::default();
    for (i, q) in questions.iter().enumerate() {
        let a = answers.get(i).unwrap_or(&blank);
        let right = a.correct(q);
        out.total.add(right);
        out.unanswered += u32::from(a.chosen.is_none());
        out.by_lecture.entry(q.lecture.clone()).or_default().add(right);
        if let Some(t) = &q.topic {
            out.by_topic.entry(format!("{}/{t}", q.lecture)).or_default().add(right);
        }
    }
    out
}

/// Topics scoring under `below` percent with at least `min` questions,
/// weakest first: "what to revise next" on the result screen.
pub fn weak_topics(o: &Outcome, below: u32, min: u32) -> Vec<(&str, Score)> {
    let mut weak: Vec<(&str, Score)> = o
        .by_topic
        .iter()
        .filter(|(_, s)| s.total >= min && s.percent() < below)
        .map(|(k, s)| (k.as_str(), *s))
        .collect();
    weak.sort_by_key(|(k, s)| (s.percent(), *k));
    weak
}

/// What to revise first after a paper: topics under `below` percent with at
/// least two questions, or, when a mixed mock spreads too thin for that, any
/// topic under `below` (#233). Empty only when no topic is under `below`.
pub fn focus_topics(o: &Outcome, below: u32) -> Vec<(&str, Score)> {
    let weak = weak_topics(o, below, 2);
    if weak.is_empty() { weak_topics(o, below, 1) } else { weak }
}

/// The bank's own memory: the last result per item (for "wrong or unseen")
/// and every finished timed paper. Serde so the store can keep it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    pub last: LastResults,
    pub papers: Vec<Outcome>,
}

impl History {
    pub fn record(&mut self, questions: &[Question], answers: &[Answer]) {
        for (q, a) in questions.iter().zip(answers) {
            if a.chosen.is_some() {
                self.last.insert(q.id.clone(), a.correct(q));
            }
        }
    }
}

/// A small seeded generator (SplitMix64), so papers are reproducible in
/// tests and core stays free of platform randomness.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            xs.swap(i, self.below(i + 1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Item, Source};

    fn mcq(id: &str, topic: &str, tags: &[&str], options: &[&str], answer: usize) -> Item {
        Item {
            id: id.into(),
            body: Body::Mcq {
                stem: format!("stem {id}"),
                options: options.iter().map(|s| s.to_string()).collect(),
                answer,
                why: options.iter().map(|_| "because".to_string()).collect(),
            },
            topic: Some(topic.into()),
            src: Source { book: None, page: 1, quote: None },
            status: Status::Unverified,
            update: None,
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn lecture(id: &str, items: Vec<Item>) -> Lecture {
        Lecture {
            schema: crate::content::SCHEMA.into(),
            lecture: id.into(),
            title: id.into(),
            author: None,
            book: crate::content::BookRef { chapter: 1, pages: [1, 99] },
            topics: vec![],
            items,
        }
    }

    const ABCD: &[&str] = &["Class A", "Class B", "Class C", "Cannot be scored"];

    fn library() -> Vec<Lecture> {
        let mut l01: Vec<Item> = (0..30)
            .map(|i| {
                let topic = if i % 3 == 0 { "child-pugh" } else { "management" };
                let tags: &[&str] = if i % 5 == 0 { &["high-yield"] } else { &[] };
                mcq(&format!("hb.L01.{i:03}"), topic, tags, ABCD, 1)
            })
            .collect();
        l01.push(Item {
            body: Body::Card { front: "f".into(), back: "b".into() },
            ..mcq("hb.L01.900", "management", &[], ABCD, 0)
        });
        l01.push(Item { status: Status::Rejected, ..mcq("hb.L01.901", "management", &[], ABCD, 0) });
        let l09 = (0..10).map(|i| mcq(&format!("hb.L09.{i:03}"), "sbp", &[], ABCD, 2)).collect();
        vec![lecture("hb.L01", l01), lecture("hb.L09", l09)]
    }

    #[test]
    fn pool_skips_other_kinds_and_rejected_items() {
        let lib = library();
        let pool = mcqs(&lib);
        assert_eq!(pool.len(), 40);
        assert!(pool.iter().all(|q| q.id != "hb.L01.900" && q.id != "hb.L01.901"));
    }

    #[test]
    fn filters_combine() {
        let lib = library();
        let pool = mcqs(&lib);
        let last = LastResults::new();
        let mut f = Filter::default();
        assert_eq!(f.select(&pool, &last).len(), 40);
        f.lectures.insert("hb.L09".into());
        assert_eq!(f.select(&pool, &last).len(), 10);
        f.lectures.clear();
        f.topics.insert(("hb.L01".into(), "child-pugh".into()));
        assert_eq!(f.select(&pool, &last).len(), 10);
        f.tags.insert("high-yield".into());
        // i % 3 == 0 and i % 5 == 0: 0, 15.
        assert_eq!(f.select(&pool, &last).len(), 2);
    }

    #[test]
    fn wrong_or_unseen_drops_only_last_right() {
        let lib = library();
        let pool = mcqs(&lib);
        let mut last = LastResults::new();
        last.insert("hb.L09.000".into(), true);
        last.insert("hb.L09.001".into(), false);
        let f = Filter {
            lectures: ["hb.L09".to_string()].into(),
            only: Only::WrongOrUnseen,
            ..Filter::default()
        };
        let ids: Vec<&str> = f.select(&pool, &last).iter().map(|q| q.id).collect();
        assert_eq!(ids.len(), 9);
        assert!(!ids.contains(&"hb.L09.000"));
        assert!(ids.contains(&"hb.L09.001"));
    }

    #[test]
    fn practice_is_seeded_and_capped() {
        let lib = library();
        let pool = mcqs(&lib);
        let a = practice(&pool, 12, 7);
        assert_eq!(a.len(), 12);
        assert_eq!(a, practice(&pool, 12, 7));
        assert_ne!(a, practice(&pool, 12, 8));
        let ids: BTreeSet<&str> = a.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(ids.len(), 12);
        assert_eq!(practice(&pool, 500, 1).len(), 40);
    }

    #[test]
    fn mock_spreads_by_lecture_and_topic() {
        let lib = library();
        let pool = mcqs(&lib);
        let paper = mock(&pool, 20, 42);
        assert_eq!(paper.len(), 20);
        let l01 = paper.iter().filter(|q| q.lecture == "hb.L01").count();
        // 30 of 40 MCQs are L01, so 15 of 20.
        assert_eq!(l01, 15);
        let cp = paper.iter().filter(|q| q.topic.as_deref() == Some("child-pugh")).count();
        assert!((7..=8).contains(&cp), "child-pugh got {cp} of 15");
        let ids: BTreeSet<&str> = paper.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(ids.len(), 20);
        assert_eq!(mock(&pool, 99, 1).len(), 40);
        assert!(mock(&[], 40, 1).is_empty());
    }

    #[test]
    fn largest_remainder_sums_and_respects_sizes() {
        assert_eq!(largest_remainder(&[30, 10], 20), vec![15, 5]);
        assert_eq!(largest_remainder(&[1, 1, 1], 2).iter().sum::<usize>(), 2);
        assert_eq!(largest_remainder(&[2, 50], 40), vec![2, 38]);
        assert_eq!(largest_remainder(&[], 5), Vec::<usize>::new());
    }

    #[test]
    fn shuffled_options_keep_the_answer() {
        let lib = library();
        let pool = mcqs(&lib);
        for q in practice(&pool, 40, 3) {
            let mut sorted = q.order.clone();
            sorted.sort();
            assert_eq!(sorted, vec![0, 1, 2, 3]);
            let src = pool.iter().find(|p| p.id == q.id).unwrap();
            assert_eq!(src.options[q.answer], src.options[src.answer]);
        }
    }

    #[test]
    fn options_that_name_others_stay_in_order() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert!(shufflable(&s(&["Propranolol", "Carvedilol", "Band ligation", "TIPS"])));
        assert!(!shufflable(&s(&["a", "b", "c", "All of the above"])));
        assert!(!shufflable(&s(&["a", "b", "Both a and b", "d"])));
        assert!(!shufflable(&s(&["x", "y", "z", "A and C"])));
        assert!(!shufflable(&s(&["x", "y", "z", "All of these"])));
        assert!(!shufflable(&s(&["Hepatitis B", "Hepatitis C", "Both", "Neither"])));
        assert!(!shufflable(&s(&["x", "y", "None"])));
        assert!(shufflable(&s(&["None while compensated", "x", "y"])));
        assert!(shufflable(&s(&["Vitamin A and D deficiency", "x", "y", "z"])));
    }

    #[test]
    fn numeric_options_go_smallest_first() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            numeric_order(&s(&["500/mm³", "100/mm³", "≥ 250/mm³", "1,000/mm³"])),
            Some(vec![1, 2, 0, 3])
        );
        assert_eq!(numeric_order(&s(&["1.1 g/dL", "0.5 g/dL"])), Some(vec![1, 0]));
        assert_eq!(numeric_order(&s(&["Class A", "5"])), None);
        assert_eq!(
            numeric_order(&s(&["Above 20 ng/ml", "above 10 ng/ml", "Less than 5"])),
            Some(vec![2, 1, 0])
        );
    }

    #[test]
    fn timing() {
        assert_eq!(time_limit_ms(40), 48 * 60_000);
        assert_eq!(remaining_ms(1, 1_000, 1_000), 90_000);
        // A full 40-question block is Step 1's hour.
        assert_eq!(time_limit_ms(MOCK_SIZE), 3_600_000);
        assert_eq!(remaining_ms(1, 1_000, 61_000), 12_000);
        assert_eq!(remaining_ms(1, 1_000, 999_999), 0);
    }

    fn q(id: &str, lecture: &str, topic: &str, answer: usize) -> Question {
        Question {
            id: id.into(),
            lecture: lecture.into(),
            topic: Some(topic.into()),
            answer,
            order: vec![0, 1, 2, 3],
        }
    }

    fn a(chosen: Option<usize>) -> Answer {
        Answer { chosen, took_ms: 5_000, flagged: false }
    }

    #[test]
    fn scoring_counts_blanks_as_wrong() {
        let qs = [q("1", "hb.L01", "cp", 1), q("2", "hb.L01", "mgmt", 0), q("3", "hb.L09", "sbp", 2)];
        let ans = [a(Some(1)), a(None), a(Some(0))];
        let o = score(&qs, &ans, Mode::Mock, 0, 60_000);
        assert_eq!(o.total, Score { right: 1, total: 3 });
        assert_eq!(o.unanswered, 1);
        assert_eq!(o.by_lecture["hb.L01"], Score { right: 1, total: 2 });
        assert_eq!(o.by_topic["hb.L09/sbp"], Score { right: 0, total: 1 });
        assert_eq!(o.total.percent(), 33);
        assert_eq!(Score { right: 1, total: 2 }.percent(), 50);
        assert_eq!(Score::default().percent(), 0);
        let weak = weak_topics(&o, 60, 1);
        assert_eq!(weak.iter().map(|w| w.0).collect::<Vec<_>>(), ["hb.L01/mgmt", "hb.L09/sbp"]);
    }

    #[test]
    fn a_thin_mock_still_names_topics_to_revise() {
        // One question per topic, mostly wrong: no topic has two answers.
        let qs = [q("1", "hb.L01", "cp", 1), q("2", "hb.L09", "sbp", 0), q("3", "hb.L13", "aih", 2)];
        let o = score(&qs, &[a(Some(0)), a(Some(0)), a(None)], Mode::Mock, 0, 60_000);
        assert!(weak_topics(&o, 60, 2).is_empty());
        let focus: Vec<_> = focus_topics(&o, 60).into_iter().map(|w| w.0).collect();
        assert_eq!(focus, ["hb.L01/cp", "hb.L13/aih"]);
        // With two answers in a topic, the stricter list wins.
        let qs = [q("1", "hb.L01", "cp", 1), q("2", "hb.L01", "cp", 0), q("3", "hb.L09", "sbp", 2)];
        let o = score(&qs, &[a(Some(0)), a(None), a(Some(0))], Mode::Mock, 0, 60_000);
        assert_eq!(focus_topics(&o, 60).into_iter().map(|w| w.0).collect::<Vec<_>>(), ["hb.L01/cp"]);
    }

    #[test]
    fn wrong_is_again_right_is_good_blank_sends_nothing() {
        let question = q("hb.L01.003", "hb.L01", "cp", 1);
        let right = review_event(&question, &a(Some(1)), Mode::Tutor, 9).unwrap();
        assert_eq!((right.grade, right.origin), (Grade::Good, Origin::Bank));
        let wrong = review_event(&question, &a(Some(0)), Mode::Mock, 9).unwrap();
        assert_eq!((wrong.grade, wrong.origin), (Grade::Again, Origin::Mock));
        assert!(review_event(&question, &a(None), Mode::Timed, 9).is_none());
    }

    #[test]
    fn history_keeps_the_last_result() {
        let qs = [q("1", "hb.L01", "cp", 1), q("2", "hb.L01", "cp", 1)];
        let mut h = History::default();
        h.record(&qs, &[a(Some(0)), a(None)]);
        h.record(&qs[..1], &[a(Some(1))]);
        assert_eq!(h.last.get("1"), Some(&true));
        assert_eq!(h.last.get("2"), None);
    }
}
