//! The content format `steps.content/v1`: one YAML file per lecture
//! (`content/hb/L01.yaml`). The rules for writing it are in docs/CONTENT.md and
//! `docs/CONTENT.md`; [`crate::validate`] enforces them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "steps.content/v1";

/// One lecture of the book, with every study item made from it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lecture {
    pub schema: String,
    /// `hb.L01`: chapter slug, then the lecture number.
    pub lecture: String,
    pub title: String,
    /// The lecturer named in the book.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub book: BookRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<Topic>,
    pub items: Vec<Item>,
}

/// Where the lecture sits in the book: printed page numbers, not PDF pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookRef {
    pub chapter: u32,
    pub pages: [u32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Topic {
    pub id: String,
    pub title: String,
}

/// One study item. The fields every kind shares live here; the kind's own
/// fields are in [`Body`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// `hb.L01.001`. Never reused or renumbered.
    pub id: String,
    #[serde(flatten)]
    pub body: Body,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    pub src: Source,
    pub status: Status,
    /// Where a current guideline differs from the book. The book stays the answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update: Option<Update>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Body {
    Card {
        front: String,
        back: String,
    },
    /// `text` holds Anki-style blanks: `{{c1::answer}}` or `{{c1::answer::hint}}`.
    Cloze {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        extra: Option<String>,
    },
    /// Single best answer. `why` has one line per option.
    Mcq {
        stem: String,
        options: Vec<String>,
        answer: usize,
        why: Vec<String>,
    },
    /// "Mention / Enumerate": recall `pass` of the `points`.
    Enumerate {
        prompt: String,
        points: Vec<String>,
        pass: usize,
        /// A تحشيشة: a light Egyptian-Arabic memory hook for the whole list
        /// (a mnemonic, a story, a joke). Shown once the points are open.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hook: Option<String>,
    },
    /// A model outline for a short-essay question.
    Essay {
        prompt: String,
        outline: Vec<OutlineEntry>,
    },
    /// Key-feature case: each step reveals data, then asks one decision.
    Case {
        stem: String,
        steps: Vec<CaseStep>,
    },
    /// A comparison grid; `blanks` are `[row, column]` cells to recall.
    /// Column 0 is the row label.
    Table {
        title: String,
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        blanks: Vec<[usize; 2]>,
    },
    /// An algorithm or management sequence, in the correct order.
    Order {
        prompt: String,
        steps: Vec<String>,
    },
    /// A vignette with numbers: compute the score, then interpret it.
    Calc {
        calculator: String,
        stem: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        question: Option<String>,
        inputs: BTreeMap<String, CalcValue>,
        expect: String,
        interpret: String,
    },
}

impl Body {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Card { .. } => "card",
            Self::Cloze { .. } => "cloze",
            Self::Mcq { .. } => "mcq",
            Self::Enumerate { .. } => "enumerate",
            Self::Essay { .. } => "essay",
            Self::Case { .. } => "case",
            Self::Table { .. } => "table",
            Self::Order { .. } => "order",
            Self::Calc { .. } => "calc",
        }
    }
}

/// Every key an item of each kind may have, shared keys included. Serde can't
/// reject unknown keys on a flattened enum, so the validator checks these.
pub fn allowed_keys(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "card" => &["front", "back"],
        "cloze" => &["text", "extra"],
        "mcq" => &["stem", "options", "answer", "why"],
        "enumerate" => &["prompt", "points", "pass", "hook"],
        "essay" => &["prompt", "outline"],
        "case" => &["stem", "steps"],
        "table" => &["title", "columns", "rows", "blanks"],
        "order" => &["prompt", "steps"],
        "calc" => &["calculator", "stem", "question", "inputs", "expect", "interpret"],
        _ => return None,
    })
}

pub const SHARED_KEYS: &[&str] = &["id", "kind", "topic", "src", "status", "update", "tags"];

/// The calculators the app knows, as written in `calculator:`. Implemented in
/// [`crate::calc`] (chunk 5); a test there keeps the two lists equal.
pub const CALCULATORS: &[&str] = &[
    "child_pugh",
    "meld",
    "meld_na",
    "albi",
    "fib4",
    "apri",
    "fli",
    "baveno_vi",
    "r_ratio",
    "maddrey",
    "glasgow_ah",
    "saag",
    "ascitic_pmn",
    "sos_criteria",
    "cha2ds2_vasc",
    "bclc",
    "milan",
    "okuda",
    "page_b",
    "kings_college",
    "clichy",
    "ranson",
    "bisap",
    "ctsi",
    "atlanta_2012",
    "glasgow_blatchford",
    "shock_index",
    "oakland",
    "uc_activity",
    "kyoto_gastritis",
    "rome4_ibs",
    "anion_gap",
    "delta_ratio",
    "expected_compensation",
    "osmolal_gap",
    "kdigo_aki",
    "fena",
    "feurea",
    "bun_creatinine",
    "ckd_epi_2021",
    "kdigo_ckd",
    "aa_gradient",
    "free_water_deficit",
    "plasmic",
    "cockcroft_gault",
    "katz_adl",
    "lawton_iadl",
    "gds15",
    "mna_sf",
    "clinical_frailty",
    "cam",
    "sarc_f",
    "mmse",
    "abi",
    "acr_eular_2010_ra",
    "caspar",
    "asas_axial_spa",
    "asas_peripheral_spa",
    "modified_new_york",
    "sle_acr_1997",
    "sle_eular_acr_2019",
    "gpa_2022",
    "mpa_2022",
    "corrected_sodium",
    "effective_osmolality",
    "dka_severity",
    "premixed_insulin",
    "bmi",
    "ipss_mf",
    "rpi",
    "rai_cll",
    "binet_cll",
    "ann_arbor",
    "ipi_nhl",
    "who_pv",
    "who_et",
    "who_pmf",
    "incidence",
    "prevalence",
    "prevalence_id",
    "attack_rate",
    "sar",
    "cfr",
    "pmr",
    "imr",
    "nmr",
    "relative_risk",
    "odds_ratio",
    "attributable_risk",
    "nnt",
    "diagnostic_accuracy",
    "z_score",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutlineEntry {
    Heading(String),
    Section(BTreeMap<String, Vec<String>>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseStep {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reveal: Option<String>,
    pub question: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<usize>,
    pub why: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CalcValue {
    Bool(bool),
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// Which source the fact comes from: an id in `content/sources.yaml`.
    /// Left out = the primary book, First Aid for the USMLE Step 1 2026.
    /// Other sources (Pathoma, Sketchy, UWorld notes…) add items to the same
    /// First Aid lecture and topic, so one topic gathers every source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book: Option<String>,
    /// Printed page of that source (for First Aid: the printed page, not the PDF page).
    pub page: u32,
    /// The page's own words: as much as proves the fact and gives the full
    /// reference for further reading (no length cap, Mohamed 2026-10-29).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
}

/// `content/sources.yaml`: every source items may cite in `src.book`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sources {
    pub schema: String,
    pub sources: Vec<SourceBook>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBook {
    /// `fa2026`, `pathoma`: what `src.book` says.
    pub id: String,
    /// `First Aid for the USMLE Step 1 2026`.
    pub title: String,
    /// A short name for page chips: `FA`.
    pub short: String,
    /// `book`, `video`, `qbank` or `notes`: what `src.page` counts.
    pub kind: String,
}

pub const SOURCES_SCHEMA: &str = "steps.sources/v1";

/// The primary source's id; `src.book` left out means this one.
pub const PRIMARY_SOURCE: &str = "fa2026";

impl Source {
    /// The source id, with the primary book filled in.
    pub fn book_id(&self) -> &str {
        self.book.as_deref().unwrap_or(PRIMARY_SOURCE)
    }

    /// Whether the fact is from First Aid itself (its page sits inside the lecture's range).
    pub fn is_primary(&self) -> bool {
        self.book_id() == PRIMARY_SOURCE
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Unverified,
    /// Set only from Mohamed's own approvals (`content/approved.txt`).
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub text: String,
    /// The guideline and year, e.g. "Baveno VII, 2021".
    #[serde(rename = "ref")]
    pub reference: String,
}

/// `content/index.yaml`: the lecture list, in book order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub schema: String,
    pub lectures: Vec<IndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexEntry {
    pub id: String,
    pub title: String,
    pub pages: [u32; 2],
    /// The chunk (thread) that writes it; see docs/WORKSTREAMS.md.
    pub chunk: u32,
}

/// Blanks in a cloze text, as `(number, answer)` in order of appearance.
/// `None` when the braces are malformed.
pub fn cloze_blanks(text: &str) -> Option<Vec<(u32, String)>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let end = after.find("}}")?;
        let inner = &after[..end];
        if inner.contains("{{") {
            return None;
        }
        let body = inner.strip_prefix('c')?;
        let (num, tail) = body.split_once("::")?;
        let num: u32 = num.parse().ok()?;
        let answer = tail.split("::").next().unwrap_or("").trim();
        if answer.is_empty() {
            return None;
        }
        out.push((num, answer.to_string()));
        rest = &after[end + 2..];
    }
    if rest.contains("}}") {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloze_blanks_parse_numbers_answers_and_hints() {
        let b = cloze_blanks("PMN {{c1::≥250}} cells/mm³ in {{c2::SBP::infection}}").unwrap();
        assert_eq!(b, vec![(1, "≥250".into()), (2, "SBP".into())]);
        assert_eq!(cloze_blanks("no blanks"), Some(vec![]));
        assert_eq!(cloze_blanks("{{c1::open"), None);
        assert_eq!(cloze_blanks("{{x1::bad}}"), None);
        assert_eq!(cloze_blanks("{{c1::}}"), None);
        assert_eq!(cloze_blanks("stray }}"), None);
    }

    #[test]
    fn items_round_trip_through_yaml() {
        let yaml = r#"
id: hb.L01.001
kind: mcq
topic: child-pugh
stem: Child-Pugh class?
options: [A, B, C, D]
answer: 1
why: [too low, "8 points", too high, not needed]
src: { page: 14 }
status: unverified
"#;
        let item: Item = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(item.body.kind(), "mcq");
        let back: Item = serde_yaml_ng::from_str(&serde_yaml_ng::to_string(&item).unwrap()).unwrap();
        assert_eq!(item, back);
    }
}
