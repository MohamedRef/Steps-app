//! The hover glossary: `content/glossary.yaml` (shared terms) plus one
//! `content/glossary/<course>.yaml` per course, so content threads never
//! edit the same file. Abbreviations and drug names
//! with a one-line expansion. The app shows an entry when the S Pen or the
//! mouse rests on the word (PARITY.md §6.3); `validate::glossary` checks the
//! file. Entries are content like any other: `status: unverified` until
//! Mohamed approves them.

use serde::{Deserialize, Serialize};

use crate::content::Status;

pub const SCHEMA: &str = "steps.glossary/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Glossary {
    pub schema: String,
    pub entries: Vec<Entry>,
    /// Words in capitals that are not abbreviations (`NOT`, `II`), so the
    /// lesson check ([`Glossary::unknown_in`]) leaves them alone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The word as written in the content: `PDA`, `nitroprusside`.
    pub term: String,
    /// Other spellings that show the same entry (`ToF`, `NTG`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    pub kind: Kind,
    /// The expansion, short enough to read at a glance.
    pub text: String,
    pub status: Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Abbr,
    Drug,
}

impl Entry {
    /// The id used in `content/approved.txt`: `glossary.PDA`.
    pub fn id(&self) -> String {
        format!("glossary.{}", self.term)
    }

    /// The term and its aliases.
    pub fn spellings(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.term.as_str()).chain(self.aliases.iter().map(String::as_str))
    }
}

impl Glossary {
    /// Adds a course file's entries and plain words to this glossary. Each
    /// course file keeps every term its lectures use, even one another file
    /// also has (nothing is removed for being repeated elsewhere); the merged
    /// glossary holds one entry per term, from the file read first
    /// (`glossary.yaml`, then the course files in name order). A term listed
    /// twice within one file is still caught by `validate::glossary`.
    pub fn absorb(&mut self, part: Glossary) {
        let known: std::collections::HashSet<String> =
            self.entries.iter().flat_map(|e| e.spellings()).map(str::to_string).collect();
        let words: std::collections::HashSet<String> = self.plain.iter().cloned().collect();
        let added: Vec<Entry> =
            part.entries.into_iter().filter(|e| !e.spellings().any(|s| known.contains(s))).collect();
        // A word one course keeps plain and another explains: the entry
        // wins, so the pointer still explains it. Within one file both
        // stay, and validation names the mix-up.
        let spelled: std::collections::HashSet<String> =
            added.iter().flat_map(|e| e.spellings()).map(str::to_string).collect();
        self.plain.retain(|w| !spelled.contains(w));
        self.entries.extend(added);
        self.plain.extend(part.plain.into_iter().filter(|w| !words.contains(w) && !known.contains(w)));
    }

    /// Finds the entry for a word under the pointer. Abbreviations match
    /// exactly (`HE` is not `he`); drug names ignore case, so a drug at the
    /// start of a sentence still matches.
    pub fn find(&self, word: &str) -> Option<&Entry> {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric());
        if word.is_empty() {
            return None;
        }
        self.entries
            .iter()
            .filter(|e| e.status != Status::Rejected)
            .find(|e| e.spellings().any(|s| s == word))
            .or_else(|| {
                self.entries
                    .iter()
                    .filter(|e| e.status != Status::Rejected && e.kind == Kind::Drug)
                    .find(|e| e.spellings().any(|s| s.eq_ignore_ascii_case(word)))
            })
    }
}

impl Glossary {
    /// The table the app's hover script looks words up in, as JSON:
    /// `{"exact": {spelling: [text, approved]}, "lower": {drug: [...]}}`.
    /// Same rules as [`Glossary::find`]; rejected entries are left out.
    pub fn peek_table(&self) -> serde_json::Value {
        let mut exact = serde_json::Map::new();
        let mut lower = serde_json::Map::new();
        for e in self.entries.iter().filter(|e| e.status != Status::Rejected) {
            let v = serde_json::json!([e.text, e.status == Status::Approved]);
            for s in e.spellings() {
                exact.entry(s.to_string()).or_insert_with(|| v.clone());
                if e.kind == Kind::Drug {
                    lower.entry(s.to_lowercase()).or_insert_with(|| v.clone());
                }
            }
        }
        serde_json::json!({ "exact": exact, "lower": lower })
    }
}

/// A spelling that reads as an abbreviation: two or more capital letters
/// (`MI`, `ARBs`, `HbA1c`, `CHA2DS2-VASc`), unlike `nitroprusside` or `Beck`.
pub fn looks_like_abbr(word: &str) -> bool {
    word.chars().filter(char::is_ascii_uppercase).count() >= 2
}

/// A letter or digit of a Latin (or Greek) word. Arabic letters don't
/// count, so `وSLE` ("and SLE") still shows `SLE`.
fn word_char(c: char) -> bool {
    c.is_alphanumeric() && !('\u{0600}'..='\u{06FF}').contains(&c)
}

/// Looks abbreviations up fast: every abbreviation spelling (and drug
/// spellings written like one, `NTG`) by exact case, plus the `plain:`
/// words. Build it once with [`Glossary::finder`] and reuse it for many
/// texts (a course's lessons, the validator).
pub struct Finder<'a> {
    spellings: std::collections::HashMap<&'a str, &'a Entry>,
    plain: std::collections::HashSet<&'a str>,
}

/// One run of Latin words joined by hyphens (`HRS-AKI`, `anti-HBc`,
/// `IL-6`), split at the hyphens, with each part's place in the text.
fn hyphen_runs(text: &str) -> Vec<Vec<(usize, &str)>> {
    let mut runs: Vec<Vec<(usize, &str)>> = Vec::new();
    let mut run: Vec<(usize, &str)> = Vec::new();
    let mut start: Option<usize> = None;
    let mut last_hyphen = false;
    for (i, c) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
        if word_char(c) {
            if start.is_none() {
                if !last_hyphen && !run.is_empty() {
                    runs.push(std::mem::take(&mut run));
                }
                start = Some(i);
            }
            last_hyphen = false;
            continue;
        }
        if let Some(s) = start.take() {
            run.push((s, &text[s..i]));
        }
        // A hyphen right after a word may join it to the next one.
        last_hyphen = c == '-' && !run.is_empty() && text[..i].ends_with(|p: char| word_char(p));
        if !last_hyphen && !run.is_empty() {
            runs.push(std::mem::take(&mut run));
        }
    }
    if !run.is_empty() {
        runs.push(run);
    }
    runs
}

impl<'a> Finder<'a> {
    /// The entry for a spelling, allowing a plural `s` (`PPIs` → `PPI`).
    fn get(&self, word: &str) -> Option<&'a Entry> {
        self.spellings.get(word).copied().or_else(|| {
            let single = word.strip_suffix('s').filter(|w| !w.is_empty())?;
            self.spellings.get(single).copied()
        })
    }

    /// Where the abbreviations stand in `text`: (start, end, entry), the
    /// longest hyphen-joined spelling first (`NSTEMI` over `STEMI`).
    fn matches(&self, text: &str) -> Vec<(usize, usize, &'a Entry)> {
        let mut out = Vec::new();
        for run in hyphen_runs(text) {
            let mut i = 0;
            while i < run.len() {
                let found = (i + 1..=run.len()).rev().find_map(|j| {
                    let (s, last) = (run[i].0, run[j - 1]);
                    let end = last.0 + last.1.len();
                    self.get(&text[s..end]).map(|e| (j, s, end, e))
                });
                match found {
                    Some((j, s, end, e)) => {
                        out.push((s, end, e));
                        i = j;
                    }
                    None => i += 1,
                }
            }
        }
        out
    }

    /// See [`Glossary::abbreviations_in`].
    pub fn abbreviations_in<S: AsRef<str>>(&self, texts: &[S]) -> Vec<&'a Entry> {
        let mut out: Vec<&'a Entry> = Vec::new();
        for text in texts {
            for (_, _, e) in self.matches(text.as_ref()) {
                if !out.iter().any(|o| std::ptr::eq(*o, e)) {
                    out.push(e);
                }
            }
        }
        out
    }

    /// See [`Glossary::unknown_in`].
    pub fn unknown_in(&self, text: &str) -> Vec<String> {
        let kept = self.matches(text);
        let mut out: Vec<String> = Vec::new();
        for run in hyphen_runs(text) {
            for (s, word) in run {
                let covered = kept.iter().any(|k| k.0 <= s && s + word.len() <= k.1);
                if looks_like_abbr(word)
                    && !covered
                    && !self.plain.contains(word)
                    && !out.iter().any(|o| o == word)
                {
                    out.push(word.to_string());
                }
            }
        }
        out
    }
}

impl Glossary {
    /// A [`Finder`] over this glossary; rejected entries are left out.
    pub fn finder(&self) -> Finder<'_> {
        let mut spellings = std::collections::HashMap::new();
        for e in self.entries.iter().filter(|e| e.status != Status::Rejected) {
            for s in e.spellings().filter(|s| e.kind == Kind::Abbr || looks_like_abbr(s)) {
                spellings.entry(s).or_insert(e);
            }
        }
        Finder { spellings, plain: self.plain.iter().map(String::as_str).collect() }
    }

    /// The abbreviations written in `texts`, each once, in the order they
    /// first appear: what a lesson step's "اختصارات" chip lists. A longer
    /// spelling wins over the shorter ones inside it (`NSTEMI` over `STEMI`).
    /// Drug names count only when written like an abbreviation (`NTG`).
    pub fn abbreviations_in<S: AsRef<str>>(&self, texts: &[S]) -> Vec<&Entry> {
        self.finder().abbreviations_in(texts)
    }

    /// Words in `text` that look like abbreviations (two or more capitals)
    /// but have no glossary entry and aren't listed as `plain`. The lesson
    /// check reports them, so every abbreviation a step shows can be
    /// explained.
    pub fn unknown_in(&self, text: &str) -> Vec<String> {
        self.finder().unknown_in(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g() -> Glossary {
        serde_yaml_ng::from_str(
            "schema: steps.glossary/v1
entries:
  - { term: MI, kind: abbr, text: Myocardial infarction, status: unverified }
  - { term: NTG, aliases: [GTN], kind: drug, text: Nitroglycerin, status: unverified }
  - { term: nitroprusside, kind: drug, text: Short-acting arteriolar and venous dilator, status: unverified }
  - { term: XYZ, kind: abbr, text: Wrong, status: rejected }",
        )
        .unwrap()
    }

    #[test]
    fn abbreviations_match_exactly() {
        let g = g();
        assert_eq!(g.find("MI").map(|e| e.text.as_str()), Some("Myocardial infarction"));
        assert_eq!(g.find("(MI),").map(|e| e.term.as_str()), Some("MI"));
        assert!(g.find("mi").is_none());
    }

    #[test]
    fn drugs_ignore_case_and_aliases_work() {
        let g = g();
        assert_eq!(g.find("Nitroprusside").map(|e| e.term.as_str()), Some("nitroprusside"));
        assert_eq!(g.find("GTN").map(|e| e.term.as_str()), Some("NTG"));
        assert_eq!(g.find("ntg").map(|e| e.term.as_str()), Some("NTG"));
    }

    #[test]
    fn rejected_entries_never_show() {
        assert!(g().find("XYZ").is_none());
    }

    #[test]
    fn peek_table_matches_find() {
        let t = g().peek_table();
        assert_eq!(t["exact"]["MI"][0], "Myocardial infarction");
        assert_eq!(t["exact"]["MI"][1], false);
        assert!(t["lower"].get("mi").is_none());
        assert_eq!(t["lower"]["gtn"][0], "Nitroglycerin");
        assert!(t["exact"].get("XYZ").is_none());
    }

    #[test]
    fn a_course_file_adds_its_terms() {
        let mut base = g();
        let part: Glossary = serde_yaml_ng::from_str(
            "schema: steps.glossary/v1\nentries:\n  - { term: PDA, kind: abbr, text: Patent ductus arteriosus, status: unverified }\nplain: [J♥NES]",
        )
        .unwrap();
        base.absorb(part);
        assert_eq!(base.find("PDA").map(|e| e.text.as_str()), Some("Patent ductus arteriosus"));
        assert_eq!(base.plain, ["J♥NES"]);
    }

    fn lesson_glossary() -> Glossary {
        serde_yaml_ng::from_str(
            "schema: steps.glossary/v1
entries:
  - { term: NSTE, kind: abbr, text: Non-ST-elevation, status: unverified }
  - { term: ACS, kind: abbr, text: Acute coronary syndrome, status: unverified }
  - { term: NSTE-ACS, kind: abbr, text: Non-ST-elevation acute coronary syndrome, status: unverified }
  - { term: ARB, kind: abbr, text: Angiotensin II receptor blocker, status: unverified }
  - { term: SLE, kind: abbr, text: Systemic lupus erythematosus, status: unverified }
  - { term: NTG, kind: drug, text: Nitroglycerin, status: unverified }
  - { term: nitroprusside, kind: drug, text: Arteriolar and venous dilator, status: unverified }
plain: [NOT]",
        )
        .unwrap()
    }

    fn terms<'a>(found: &[&'a Entry]) -> Vec<&'a str> {
        found.iter().map(|e| e.term.as_str()).collect()
    }

    #[test]
    fn abbreviations_in_a_step_come_once_in_order_longest_first() {
        let g = lesson_glossary();
        let found =
            g.abbreviations_in(&["Give nitroprusside for NSTE-ACS, then NTG.", "ACS and ARBs, وSLE."]);
        // NSTE-ACS swallows NSTE and ACS in the first text; ACS shows from the second.
        assert_eq!(terms(&found), ["NSTE-ACS", "NTG", "ACS", "ARB", "SLE"]);
        // Drug names written in lower case are not abbreviations.
        assert!(!terms(&found).contains(&"nitroprusside"));
        // Inside a longer word, nothing matches.
        assert!(g.abbreviations_in(&["NSTEX ARBX"]).is_empty());
    }

    #[test]
    fn unknown_capitals_are_reported_but_plain_words_are_not() {
        let g = lesson_glossary();
        assert_eq!(
            g.unknown_in("Do NOT stop the ARB in HCM or NSTE-ACS; TIMI-3 flow is back."),
            ["HCM", "TIMI"]
        );
        assert!(g.unknown_in("Wolff-Parkinson-White and mmHg are fine, وSLE too.").is_empty());
    }
}
