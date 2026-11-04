//! Clinical calculators for hepatology and the pancreas, and the research
//! measures of Basics of Research (rates, RR/OR, NNT, diagnostic accuracy).
//!
//! Every calculator is a pure function from named [`Inputs`] to an
//! [`Outcome`]. The [`CalcSpec`] registry describes each calculator's fields so
//! the app can render one generic form, and so a content item of kind `calc`
//! (`calculator: child_pugh`, `inputs: {…}`) can be computed and checked by
//! the validator. Where the book gives the cut-offs, the book wins and the
//! outcome carries its printed page; other sources are cited in each module.
//!
//! Medical text here is English (like the book and the exam). Units follow
//! the book and Egyptian lab reports: bilirubin mg/dL, albumin g/dL,
//! creatinine mg/dL. Calculators whose published formula uses SI units
//! convert internally.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

pub use crate::content::CalcValue;

mod aa_gradient;
mod aav;
mod aki;
mod albi;
mod apri;
mod ascitic;
mod atlanta;
mod baveno;
mod bclc;
mod bisap;
mod bmi;
mod bun_cr;
mod cha2ds2_vasc;
mod child_pugh;
mod ckd;
mod clichy;
mod cll;
mod cockcroft;
mod ctsi;
mod diabetes;
mod dka;
mod fib4;
mod fli;
mod free_water;
mod gbs;
mod geri;
mod glasgow;
mod insulin;
mod ipss_mf;
mod kings;
mod kyoto;
mod lgib;
mod lymphoma;
mod maddrey;
mod meld;
mod milan;
mod mpn;
mod okuda;
mod page_b;
mod plasmic;
mod r_ratio;
mod ra2010;
mod ranson;
mod renal;
mod research;
mod rome_ibs;
mod rpi;
mod saag;
mod sle;
mod sos;
mod spa;
mod uc_activity;

/// Named inputs for one calculation.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Inputs(pub BTreeMap<String, CalcValue>);

impl Inputs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Content inputs for one calculator. A key may drop its unit suffix
    /// (`alt` for `alt_u_l`, `bilirubin` for `bilirubin_mg_dl`); it is renamed
    /// to the field's key. Keys that match no field stay as written, so
    /// [`unknown_inputs`] can report them.
    pub fn for_spec(spec: &CalcSpec, raw: &BTreeMap<String, CalcValue>) -> Self {
        let rename = |k: &String| {
            spec.fields
                .iter()
                .find(|f| f.key == k || base_key(f.key) == k)
                .map_or_else(|| k.clone(), |f| f.key.to_owned())
        };
        Self(raw.iter().map(|(k, v)| (rename(k), v.clone())).collect())
    }

    pub fn num(mut self, key: &str, v: f64) -> Self {
        self.0.insert(key.to_owned(), CalcValue::Number(v));
        self
    }

    pub fn flag(mut self, key: &str, v: bool) -> Self {
        self.0.insert(key.to_owned(), CalcValue::Bool(v));
        self
    }

    pub fn choice(mut self, key: &str, v: &str) -> Self {
        self.0.insert(key.to_owned(), CalcValue::Text(v.to_owned()));
        self
    }

    /// A required finite number.
    pub fn get_num(&self, key: &'static str) -> Result<f64, CalcError> {
        self.opt_num(key)?.ok_or(CalcError::Missing(key))
    }

    /// An optional finite number. Integers written as choices ("2") are
    /// accepted so form text fields can pass strings straight through.
    pub fn opt_num(&self, key: &'static str) -> Result<Option<f64>, CalcError> {
        let v = match self.0.get(key) {
            None => return Ok(None),
            Some(CalcValue::Number(n)) => *n,
            Some(CalcValue::Text(s)) if s.trim().is_empty() => return Ok(None),
            Some(CalcValue::Text(s)) => s
                .trim()
                .parse::<f64>()
                .map_err(|_| CalcError::Invalid { field: key, reason: "not a number" })?,
            Some(CalcValue::Bool(_)) => {
                return Err(CalcError::Invalid { field: key, reason: "expected a number" });
            }
        };
        if v.is_finite() {
            Ok(Some(v))
        } else {
            Err(CalcError::Invalid { field: key, reason: "not a finite number" })
        }
    }

    /// A positive number, as needed under a logarithm or a division.
    pub fn get_pos(&self, key: &'static str) -> Result<f64, CalcError> {
        let v = self.get_num(key)?;
        if v > 0.0 { Ok(v) } else { Err(CalcError::Invalid { field: key, reason: "must be above zero" }) }
    }

    /// A yes/no input; missing means no.
    pub fn get_flag(&self, key: &'static str) -> Result<bool, CalcError> {
        match self.0.get(key) {
            None => Ok(false),
            Some(CalcValue::Bool(b)) => Ok(*b),
            Some(CalcValue::Text(s)) => match s.as_str() {
                "yes" | "true" => Ok(true),
                "no" | "false" | "" => Ok(false),
                _ => Err(CalcError::Invalid { field: key, reason: "expected yes or no" }),
            },
            Some(CalcValue::Number(_)) => {
                Err(CalcError::Invalid { field: key, reason: "expected yes or no" })
            }
        }
    }

    /// A required choice, matched against each option's id and its aliases
    /// (content writers say "mild" as often as "mild_moderate"). Returns the id.
    pub fn get_option(
        &self,
        key: &'static str,
        options: &[(&'static str, &[&str])],
    ) -> Result<&'static str, CalcError> {
        let text = match self.0.get(key) {
            None => return Err(CalcError::Missing(key)),
            Some(CalcValue::Text(s)) => s.trim().to_ascii_lowercase().replace([' ', '-'], "_"),
            Some(_) => return Err(CalcError::Invalid { field: key, reason: "not one of the options" }),
        };
        options
            .iter()
            .find(|(id, aliases)| *id == text || aliases.contains(&text.as_str()))
            .map(|(id, _)| *id)
            .ok_or(CalcError::Invalid { field: key, reason: "not one of the options" })
    }

    /// A required choice that must be one of `allowed`.
    pub fn get_choice(&self, key: &'static str, allowed: &[&str]) -> Result<&str, CalcError> {
        match self.0.get(key) {
            None => Err(CalcError::Missing(key)),
            Some(CalcValue::Text(s)) if allowed.contains(&s.as_str()) => Ok(s.as_str()),
            Some(_) => Err(CalcError::Invalid { field: key, reason: "not one of the options" }),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CalcError {
    Unknown(String),
    Missing(&'static str),
    Invalid { field: &'static str, reason: &'static str },
}

impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(id) => write!(f, "unknown calculator `{id}`"),
            Self::Missing(k) => write!(f, "missing input `{k}`"),
            Self::Invalid { field, reason } => write!(f, "input `{field}`: {reason}"),
        }
    }
}

impl std::error::Error for CalcError {}

/// One line of the breakdown: a criterion and what it contributed.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Part {
    pub label: String,
    /// Points for scores; 1/0 for criteria met or not.
    pub points: f64,
    /// `Some` for yes/no criteria (King's, Ranson, BISAP), `None` for points.
    pub met: Option<bool>,
}

impl Part {
    fn new(label: impl Into<String>, points: f64) -> Self {
        Self { label: label.into(), points, met: None }
    }

    fn met(label: impl Into<String>, met: bool) -> Self {
        Self { label: label.into(), points: if met { 1.0 } else { 0.0 }, met: Some(met) }
    }
}

/// The result of one calculation.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Outcome {
    /// Numeric result; `None` for pure decision tables (BCLC, King's).
    pub score: Option<f64>,
    /// The result as shown, e.g. "9", "1.4 g/dL", "Stage B".
    pub display: String,
    /// Stable band id, e.g. `child_pugh.b`. Content `expect:` and
    /// interpretation MCQs key on this.
    pub band: &'static str,
    /// Short English label of the band, e.g. "Class B (7–9 points)".
    pub label: &'static str,
    /// What the band means, in the book's words where it has them.
    pub meaning: String,
    /// Printed book page of the cut-offs, when the book has them.
    pub page: Option<u32>,
    pub parts: Vec<Part>,
    /// Caveats, e.g. "48-hour criteria not entered".
    pub notes: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FieldKind {
    Number {
        unit: &'static str,
        min: f64,
        max: f64,
        step: f64,
    },
    Flag,
    /// (id, English label) pairs.
    Choice(&'static [(&'static str, &'static str)]),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    pub optional: bool,
    pub hint: &'static str,
}

const fn num(
    key: &'static str,
    label: &'static str,
    unit: &'static str,
    min: f64,
    max: f64,
    step: f64,
) -> Field {
    Field { key, label, kind: FieldKind::Number { unit, min, max, step }, optional: false, hint: "" }
}

const fn flag(key: &'static str, label: &'static str) -> Field {
    Field { key, label, kind: FieldKind::Flag, optional: true, hint: "" }
}

const fn choice(
    key: &'static str,
    label: &'static str,
    options: &'static [(&'static str, &'static str)],
) -> Field {
    Field { key, label, kind: FieldKind::Choice(options), optional: false, hint: "" }
}

impl Field {
    const fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    const fn hint(mut self, hint: &'static str) -> Self {
        self.hint = hint;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Liver,
    Ascites,
    Tumour,
    AcuteFailure,
    Pancreas,
    Gi,
    Renal,
    Elderly,
    Rheum,
    Diabetes,
    Blood,
    Research,
}

pub struct CalcSpec {
    /// The id used in content (`calculator: child_pugh`).
    pub id: &'static str,
    pub name: &'static str,
    pub area: Area,
    pub fields: &'static [Field],
    /// Formula source, shown under the result.
    pub source: &'static str,
    pub compute: fn(&Inputs) -> Result<Outcome, CalcError>,
}

impl fmt::Debug for CalcSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalcSpec").field("id", &self.id).finish_non_exhaustive()
    }
}

/// What to do with a result and what else it could be: the next step and
/// the differential diagnosis for one band. Kept apart from [`Outcome`] so a
/// calculator without a guide still builds; each module lists its own as
/// `GUIDES` and [`GUIDE_LISTS`] collects them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guide {
    /// Next steps for this result, most important first.
    pub next: &'static [&'static str],
    /// The differential diagnosis (or causes to look for) for this result.
    pub dd: &'static [&'static str],
}

// One line per module that has guides, so branches adding them don't touch
// the same line.
#[rustfmt::skip]
static GUIDE_LISTS: &[&[(&str, Guide)]] = &[
    geri::GUIDES,
    spa::GUIDES,
    sle::GUIDES,
    aav::GUIDES,
    page_b::GUIDES,
    milan::GUIDES,
    okuda::GUIDES,
    ctsi::GUIDES,
    atlanta::GUIDES,
    baveno::GUIDES,
    fli::GUIDES,
    sos::GUIDES,
    cha2ds2_vasc::GUIDES,
    rpi::GUIDES,
    cll::GUIDES,
    lymphoma::GUIDES,
    mpn::GUIDES,
    lgib::GUIDES,
    uc_activity::GUIDES,
    kyoto::GUIDES,
    rome_ibs::GUIDES,
    dka::GUIDES,
    insulin::GUIDES,
    bmi::GUIDES,
    bun_cr::GUIDES,
    ckd::GUIDES,
    aa_gradient::GUIDES,
    free_water::GUIDES,
    plasmic::GUIDES,
];

/// The next step and differential for a band id (`cam.positive`), if
/// written.
pub fn guide(band: &str) -> Option<&'static Guide> {
    GUIDE_LISTS.iter().flat_map(|l| l.iter()).find(|(b, _)| *b == band).map(|(_, g)| g)
}

/// Every calculator, in the order the Calculators screen lists them.
pub fn all() -> &'static [CalcSpec] {
    REGISTRY
}

pub fn spec(id: &str) -> Option<&'static CalcSpec> {
    REGISTRY.iter().find(|s| s.id == id)
}

pub fn compute(id: &str, inputs: &Inputs) -> Result<Outcome, CalcError> {
    let spec = spec(id).ok_or_else(|| CalcError::Unknown(id.to_owned()))?;
    (spec.compute)(inputs)
}

// A slice, not an array, so adding one doesn't touch a count that every
// other branch adding one also changes.
static REGISTRY: &[CalcSpec] = &[
    child_pugh::SPEC,
    meld::MELD,
    meld::MELD_NA,
    albi::SPEC,
    fib4::SPEC,
    apri::SPEC,
    fli::SPEC,
    baveno::SPEC,
    r_ratio::SPEC,
    maddrey::SPEC,
    glasgow::SPEC,
    saag::SPEC,
    ascitic::SPEC,
    sos::SPEC,
    cha2ds2_vasc::SPEC,
    bclc::SPEC,
    milan::SPEC,
    okuda::SPEC,
    page_b::SPEC,
    kings::SPEC,
    clichy::SPEC,
    ranson::SPEC,
    bisap::SPEC,
    ctsi::SPEC,
    atlanta::SPEC,
    gbs::SPEC,
    lgib::SHOCK_INDEX,
    lgib::OAKLAND,
    uc_activity::SPEC,
    kyoto::SPEC,
    rome_ibs::SPEC,
    renal::ANION_GAP,
    renal::DELTA_RATIO,
    renal::COMPENSATION,
    renal::OSMOLAL_GAP,
    aki::KDIGO,
    aki::FENA,
    aki::FEUREA,
    bun_cr::SPEC,
    ckd::EGFR,
    ckd::STAGE,
    aa_gradient::SPEC,
    free_water::SPEC,
    plasmic::SPEC,
    cockcroft::SPEC,
    geri::KATZ,
    geri::LAWTON,
    geri::GDS,
    geri::MNA_SF,
    geri::CFS,
    geri::CAM,
    geri::SARC_F,
    geri::MMSE,
    geri::ABI,
    ra2010::SPEC,
    spa::CASPAR,
    spa::ASAS_AXIAL,
    spa::ASAS_PERIPHERAL,
    spa::MODIFIED_NEW_YORK,
    sle::SLE_1997,
    sle::SLE_2019,
    aav::GPA_2022,
    aav::MPA_2022,
    diabetes::CORRECTED_NA,
    diabetes::OSMOLALITY,
    dka::SPEC,
    insulin::PREMIXED,
    bmi::SPEC,
    ipss_mf::SPEC,
    rpi::SPEC,
    cll::RAI,
    cll::BINET,
    lymphoma::ANN_ARBOR,
    lymphoma::IPI,
    mpn::PV,
    mpn::ET,
    mpn::PMF,
    research::INCIDENCE,
    research::PREVALENCE,
    research::PREVALENCE_ID,
    research::ATTACK_RATE,
    research::SAR,
    research::CFR,
    research::PMR,
    research::IMR,
    research::NMR,
    research::RELATIVE_RISK,
    research::ODDS_RATIO,
    research::ATTRIBUTABLE_RISK,
    research::NNT,
    research::DIAGNOSTIC,
    research::Z_SCORE,
];

const UNIT_SUFFIXES: &[&str] = &[
    "_10e9_l", "_per_mm3", "_mm_hg", "_meq_l", "_mmol_l", "_mg_dl", "_g_dl", "_iu_l", "_u_l", "_years",
    "_points", "_days", "_pct", "_kpa", "_cm", "_kg", "_s", "_l",
];

/// A field key without its unit suffix: `alt_uln_u_l` → `alt_uln`.
fn base_key(key: &str) -> &str {
    UNIT_SUFFIXES.iter().find_map(|s| key.strip_suffix(s)).unwrap_or(key)
}

/// Input keys in `inputs` that the calculator does not read: almost always a
/// typo in content.
pub fn unknown_inputs<'a>(spec: &CalcSpec, inputs: &'a Inputs) -> Vec<&'a str> {
    inputs.0.keys().map(String::as_str).filter(|k| !spec.fields.iter().any(|f| f.key == *k)).collect()
}

/// Whether an answer Mohamed typed for a `calc` item is right: anything
/// [`expect_matches`] accepts, or a text that names only the right band
/// ("Child-Pugh C", "class c", "met"). The validator keeps the stricter
/// [`expect_matches`], since content should show the number.
pub fn answer_matches(outcome: &Outcome, typed: &str) -> bool {
    if expect_matches(outcome, typed) {
        return true;
    }
    let named = named_bands(typed, outcome.band);
    !named.is_empty() && named.iter().all(|b| *b == outcome.band)
}

/// Whether a content item's `expect` agrees with the computed outcome. It
/// may be the band id (`child_pugh.b`) or free text that shows the result,
/// such as "Child-Pugh B (8 points)" (contains "8") or "Stage C" for BCLC.
pub fn expect_matches(outcome: &Outcome, expect: &str) -> bool {
    let expect = expect.trim();
    if expect == outcome.band {
        return true;
    }
    // A text that names another band ("Below 9", "Child-Pugh C") is wrong
    // even when one of its numbers equals the score.
    if named_bands(expect, outcome.band).iter().any(|b| *b != outcome.band) {
        return false;
    }
    if let Some(score) = outcome.score {
        // A number written in `expect` equal to the score at the precision it
        // is written with: "0.9" and "0.90" both match 0.898.
        let matches = |n: &String| {
            let dp = n.split_once('.').map_or(0, |(_, f)| f.len()) as i32;
            let scale = 10f64.powi(dp);
            n.parse::<f64>().is_ok_and(|v| ((score * scale).round() - v * scale).abs() < 0.5)
        };
        if numbers(expect).iter().any(matches) {
            return true;
        }
    }
    expect.eq_ignore_ascii_case(&outcome.display) || expect.contains(outcome.label)
}

/// Words that name a band in a written result, per band id. Calculators
/// whose bands are only numbers (MELD, Ranson) have none. Phrases are
/// lower case and match whole words; the longest phrase wins, so "not met"
/// is never read as "met".
const BAND_WORDS: &[(&str, &[&str])] = &[
    ("child_pugh.a", &["child-pugh a", "child pugh a", "class a"]),
    ("child_pugh.b", &["child-pugh b", "child pugh b", "class b"]),
    ("child_pugh.c", &["child-pugh c", "child pugh c", "class c"]),
    ("albi.1", &["grade 1"]),
    ("albi.2", &["grade 2"]),
    ("albi.3", &["grade 3"]),
    ("fib4.low", &["low risk"]),
    ("fib4.mid", &["intermediate", "indeterminate"]),
    ("fib4.high", &["high risk"]),
    ("r_ratio.hepatocellular", &["hepatocellular"]),
    ("r_ratio.cholestatic", &["cholestatic"]),
    ("r_ratio.mixed", &["mixed"]),
    ("maddrey.severe", &["severe"]),
    ("maddrey.not_severe", &["not severe"]),
    ("glasgow.high", &["9 or more", "9 or above"]),
    ("glasgow.low", &["below 9", "less than 9"]),
    ("saag.high", &["high"]),
    ("saag.low", &["low"]),
    ("ascitic.no_sbp", &["no sbp"]),
    ("ascitic.bacterascites", &["bacterascites"]),
    ("ascitic.secondary", &["secondary"]),
    ("ascitic.polymicrobial", &["polymicrobial"]),
    ("bclc.0", &["stage 0"]),
    ("bclc.a", &["stage a"]),
    ("bclc.b", &["stage b"]),
    ("bclc.c", &["stage c"]),
    ("bclc.d", &["stage d"]),
    ("milan.within", &["within"]),
    ("milan.outside", &["outside"]),
    ("okuda.1", &["stage i", "stage 1"]),
    ("okuda.2", &["stage ii", "stage 2"]),
    ("okuda.3", &["stage iii", "stage 3"]),
    ("page_b.low", &["low risk", "low"]),
    ("page_b.mid", &["intermediate"]),
    ("page_b.high", &["high risk", "high"]),
    ("kings.met", &["met"]),
    ("kings.not_met", &["not met"]),
    ("clichy.met", &["met"]),
    ("clichy.not_met", &["not met"]),
    ("bisap.severe", &["severe", "2 or more"]),
    ("bisap.mild", &["mild", "not severe"]),
    ("atlanta_2012.mild", &["mild"]),
    ("atlanta_2012.moderately_severe", &["moderately severe", "moderate"]),
    ("atlanta_2012.severe", &["severe"]),
    ("baveno_vi.spare", &["met", "skip", "no endoscopy"]),
    ("baveno_vi.screen", &["not met", "screen"]),
    ("fli.low", &["below 30", "ruled out"]),
    ("fli.mid", &["indeterminate", "intermediate"]),
    ("fli.high", &["60 or above", "likely"]),
    ("sos_criteria.met", &["met"]),
    ("sos_criteria.not_met", &["not met"]),
    ("cha2ds2_vasc.low", &["low risk", "low"]),
    ("cha2ds2_vasc.high", &["high risk", "high"]),
    ("anion_gap.high", &["high"]),
    ("anion_gap.normal", &["normal"]),
    ("anion_gap.low", &["low"]),
    ("expected_compensation.appropriate", &["appropriate"]),
    ("osmolal_gap.present", &["gap present"]),
    ("osmolal_gap.normal", &["no gap"]),
    ("katz_adl.independent", &["independent"]),
    ("katz_adl.partial", &["partly dependent", "partially dependent"]),
    ("katz_adl.very_dependent", &["very dependent"]),
    ("lawton_iadl.independent", &["independent"]),
    ("lawton_iadl.partial", &["partly dependent", "partially dependent"]),
    ("lawton_iadl.dependent", &["dependent"]),
    ("gds15.normal", &["normal", "no depression"]),
    ("gds15.depression", &["suggests depression", "depression", "depressed"]),
    ("mna_sf.normal", &["normal"]),
    ("mna_sf.at_risk", &["at risk"]),
    ("mna_sf.malnourished", &["malnourished", "malnutrition"]),
    ("cfs.1", &["very fit"]),
    ("cfs.2", &["well"]),
    ("cfs.3", &["managing well"]),
    ("cfs.4", &["vulnerable"]),
    ("cfs.5", &["mildly frail"]),
    ("cfs.6", &["moderately frail"]),
    ("cfs.7", &["severely frail"]),
    ("cfs.8", &["very severely frail"]),
    ("cfs.9", &["terminally ill"]),
    ("cam.positive", &["positive", "delirium"]),
    ("cam.negative", &["negative", "no delirium"]),
    ("sarc_f.positive", &["positive"]),
    ("sarc_f.negative", &["negative"]),
    ("mmse.normal", &["normal"]),
    ("mmse.borderline", &["borderline"]),
    ("mmse.marked", &["marked"]),
    ("mmse.severe", &["severe"]),
    ("abi.normal", &["normal"]),
    ("abi.pad", &["pad", "peripheral arterial disease"]),
    ("abi.critical", &["critical"]),
    ("acr_eular_2010_ra.definite", &["definite"]),
    ("acr_eular_2010_ra.not_met", &["not met"]),
    ("caspar.met", &["met", "psoriatic arthritis"]),
    ("caspar.not_met", &["not met"]),
    ("asas_axial_spa.imaging", &["imaging arm"]),
    ("asas_axial_spa.clinical", &["clinical arm", "hla-b27 arm"]),
    ("asas_axial_spa.not_met", &["not met"]),
    ("asas_peripheral_spa.met", &["met"]),
    ("asas_peripheral_spa.not_met", &["not met"]),
    ("modified_new_york.met", &["met", "definite as", "definite"]),
    ("modified_new_york.not_met", &["not met"]),
    ("sle_acr_1997.met", &["met", "4 or more"]),
    ("sle_acr_1997.not_met", &["not met", "below 4"]),
    ("sle_eular_acr_2019.met", &["met", "10 or more"]),
    ("sle_eular_acr_2019.not_met", &["not met"]),
    ("gpa_2022.met", &["met", "5 or more"]),
    ("gpa_2022.not_met", &["not met", "below 5"]),
    ("mpa_2022.met", &["met", "5 or more"]),
    ("mpa_2022.not_met", &["not met", "below 5"]),
    ("effective_osmolality.hhs", &["hhs", "meets"]),
    ("ipss_mf.low", &["low"]),
    ("ipss_mf.int1", &["intermediate-1", "intermediate 1", "int-1"]),
    ("ipss_mf.int2", &["intermediate-2", "intermediate 2", "int-2"]),
    ("ipss_mf.high", &["high"]),
    ("rpi.low", &["below 2", "less than 2", "underproduction", "inadequate"]),
    ("rpi.adequate", &["2 or more", "adequate"]),
    ("rai_cll.0", &["stage 0"]),
    ("rai_cll.1", &["stage i", "stage 1"]),
    ("rai_cll.2", &["stage ii", "stage 2"]),
    ("rai_cll.3", &["stage iii", "stage 3"]),
    ("rai_cll.4", &["stage iv", "stage 4"]),
    ("binet_cll.a", &["stage a"]),
    ("binet_cll.b", &["stage b"]),
    ("binet_cll.c", &["stage c"]),
    ("ann_arbor.1a", &["stage ia", "stage 1a"]),
    ("ann_arbor.1b", &["stage ib", "stage 1b"]),
    ("ann_arbor.2a", &["stage iia", "stage 2a"]),
    ("ann_arbor.2b", &["stage iib", "stage 2b"]),
    ("ann_arbor.3a", &["stage iiia", "stage 3a"]),
    ("ann_arbor.3b", &["stage iiib", "stage 3b"]),
    ("ann_arbor.4a", &["stage iva", "stage 4a"]),
    ("ann_arbor.4b", &["stage ivb", "stage 4b"]),
    ("ipi_nhl.low", &["low risk", "low"]),
    ("ipi_nhl.low_int", &["low-intermediate", "low intermediate"]),
    ("ipi_nhl.high_int", &["high-intermediate", "high intermediate"]),
    ("ipi_nhl.high", &["high risk", "high"]),
    ("who_pv.met", &["met", "meets"]),
    ("who_pv.not_met", &["not met"]),
    ("who_et.met", &["met", "meets"]),
    ("who_et.not_met", &["not met"]),
    ("who_pmf.met", &["met", "meets"]),
    ("who_pmf.not_met", &["not met"]),
    ("kdigo_aki.none", &["no aki"]),
    ("kdigo_aki.1", &["stage 1"]),
    ("kdigo_aki.2", &["stage 2"]),
    ("kdigo_aki.3", &["stage 3"]),
    ("fena.prerenal", &["prerenal", "pre-renal"]),
    ("fena.atn", &["atn", "tubular necrosis"]),
    ("feurea.prerenal", &["prerenal", "pre-renal"]),
    ("feurea.atn", &["atn", "tubular necrosis"]),
    ("aa_gradient.high", &["raised", "high"]),
    ("aa_gradient.normal", &["normal"]),
    ("bmi.class1", &["class i", "class 1"]),
    ("bmi.class2", &["class ii", "class 2"]),
    ("bmi.class3", &["class iii", "class 3"]),
    ("bmi.normal", &["normal"]),
    ("bmi.overweight", &["overweight"]),
    ("bmi.under", &["underweight"]),
    ("bun_creatinine.atn", &["atn", "tubular necrosis"]),
    ("bun_creatinine.indeterminate", &["indeterminate"]),
    ("bun_creatinine.prerenal", &["prerenal", "pre-renal"]),
    ("ckd_epi_2021.g1", &["g1"]),
    ("ckd_epi_2021.g2", &["g2"]),
    ("ckd_epi_2021.g3a", &["g3a"]),
    ("ckd_epi_2021.g3b", &["g3b"]),
    ("ckd_epi_2021.g4", &["g4"]),
    ("ckd_epi_2021.g5", &["g5"]),
    ("dka_severity.mild", &["mild"]),
    ("dka_severity.moderate", &["moderate"]),
    ("dka_severity.not_met", &["not dka", "not met"]),
    ("dka_severity.severe", &["severe"]),
    ("kdigo_ckd.high", &["high risk"]),
    ("kdigo_ckd.low", &["low risk"]),
    ("kdigo_ckd.moderate", &["moderately increased"]),
    ("kdigo_ckd.very_high", &["very high risk", "very high"]),
    ("kyoto_gastritis.cancer_risk", &["cancer risk"]),
    ("kyoto_gastritis.current", &["current infection"]),
    ("kyoto_gastritis.none", &["no h. pylori", "no infection"]),
    ("oakland.admit", &["admit", "above 8"]),
    ("oakland.discharge", &["discharge", "8 or less"]),
    ("plasmic.high", &["high"]),
    ("plasmic.intermediate", &["intermediate"]),
    ("plasmic.low", &["low"]),
    ("rome4_ibs.c", &["ibs-c"]),
    ("rome4_ibs.d", &["ibs-d"]),
    ("rome4_ibs.m", &["ibs-m"]),
    ("rome4_ibs.not_met", &["not met"]),
    ("rome4_ibs.u", &["ibs-u"]),
    ("shock_index.stable", &["stable"]),
    ("shock_index.unstable", &["unstable"]),
    ("uc_activity.mild", &["mild"]),
    ("uc_activity.moderate", &["moderate"]),
    ("uc_activity.severe", &["severe"]),
];

/// The bands of `band`'s calculator that `text` names, longest phrase first.
fn named_bands(text: &str, band: &str) -> Vec<&'static str> {
    let calculator = band.split_once('.').map_or(band, |(c, _)| c);
    let mut phrases: Vec<(&'static str, &'static str)> = BAND_WORDS
        .iter()
        .filter(|(b, _)| b.split_once('.').is_some_and(|(c, _)| c == calculator))
        .flat_map(|(b, ws)| ws.iter().map(move |w| (*b, *w)))
        .collect();
    phrases.sort_by_key(|(_, w)| std::cmp::Reverse(w.len()));
    let mut text: Vec<char> = text.to_lowercase().replace('–', "-").chars().collect();
    let mut found = Vec::new();
    for (b, w) in phrases {
        let w: Vec<char> = w.chars().collect();
        let mut i = 0;
        while i + w.len() <= text.len() {
            let edge = |j: Option<&char>| j.is_none_or(|c| !c.is_alphanumeric());
            if text[i..i + w.len()] == w[..]
                && edge(i.checked_sub(1).and_then(|j| text.get(j)))
                && edge(text.get(i + w.len()))
            {
                // Blank it out so a shorter phrase inside it can't match too.
                text[i..i + w.len()].fill(' ');
                found.push(b);
                i += w.len();
            } else {
                i += 1;
            }
        }
    }
    found
}

/// The numbers written in a text, as written ("8", "−2.59", "1.4").
fn numbers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        let sign = (c == '-' || c == '−') && cur.is_empty();
        if c.is_ascii_digit() || (c == '.' && !cur.is_empty()) || sign {
            cur.push(if c == '−' { '-' } else { c });
        } else if !cur.is_empty() {
            let n = std::mem::take(&mut cur);
            if n.chars().any(|c| c.is_ascii_digit()) {
                out.push(n.trim_end_matches('.').to_owned());
            }
        }
    }
    out
}

/// `docs/CALCULATORS.md`: every calculator's id and inputs, for content
/// writers. A test keeps the file equal to this.
pub fn reference_markdown() -> String {
    let mut out = String::from(
        "# Calculators\n\nGenerated from `crates/core/src/calc` (run \
         `cargo test -p steps-core -- --ignored print_reference`). A `calc` item names one of these in \
         `calculator:` and gives its `inputs` by key (a key may drop its unit suffix: `alt` for \
         `alt_u_l`). `just validate` computes every item, so a typo, a \
         missing input or an `expect` that disagrees fails the check. `expect` may be the band id \
         (`child_pugh.b`) or text that contains the result (`Child-Pugh B (8 points)`, `Stage C`).\n",
    );
    for s in all() {
        out.push_str(&format!(
            "\n## `{}`: {}\n\nSource: {}\n\n| input | kind | |\n|---|---|---|\n",
            s.id, s.name, s.source
        ));
        for f in s.fields {
            let kind = match f.kind {
                FieldKind::Number { unit: "", .. } => "number".to_owned(),
                FieldKind::Number { unit, .. } => format!("number, {unit}"),
                FieldKind::Flag => "true / false (default false)".to_owned(),
                FieldKind::Choice(opts) => {
                    opts.iter().map(|(id, _)| format!("`{id}`")).collect::<Vec<_>>().join(", ")
                }
            };
            let note = if f.optional && !matches!(f.kind, FieldKind::Flag) { "optional" } else { "" };
            out.push_str(&format!(
                "| `{}` | {kind} | {}{}{} |\n",
                f.key,
                f.label,
                if note.is_empty() { "" } else { "; " },
                note
            ));
        }
    }
    out
}

/// Round to `dp` decimal places for display.
fn show(v: f64, dp: usize) -> String {
    format!("{v:.dp$}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guides_name_real_bands_once() {
        let mut bands: Vec<&str> = GUIDE_LISTS.iter().flat_map(|l| l.iter()).map(|(b, _)| *b).collect();
        let n = bands.len();
        bands.sort_unstable();
        bands.dedup();
        assert_eq!(bands.len(), n, "a band has two guides");
        for b in bands {
            let calc = b.split_once('.').map_or(b, |(c, _)| c);
            // Band ids start with their calculator's id or a short form of
            // it (`kings.met`), which BAND_WORDS also uses.
            let known = all().iter().any(|s| s.id == calc)
                || BAND_WORDS.iter().any(|(w, _)| w.split_once('.').is_some_and(|(c, _)| c == calc));
            assert!(known, "guide for unknown band {b}");
            let g = guide(b).unwrap();
            assert!(!g.next.is_empty() || !g.dd.is_empty(), "{b}: empty guide");
        }
    }

    #[test]
    fn registry_ids_are_unique_and_known() {
        let mut ids: Vec<_> = all().iter().map(|s| s.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all().len());
        assert!(spec("child_pugh").is_some());
        assert!(matches!(compute("nope", &Inputs::new()), Err(CalcError::Unknown(_))));
    }

    #[test]
    fn every_field_key_is_unique_per_calculator() {
        for s in all() {
            let mut keys: Vec<_> = s.fields.iter().map(|f| f.key).collect();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(keys.len(), s.fields.len(), "{}", s.id);
        }
    }

    #[test]
    fn empty_inputs_fail_cleanly() {
        for s in all() {
            if s.fields.iter().any(|f| !f.optional) {
                assert!((s.compute)(&Inputs::new()).is_err(), "{}", s.id);
            }
        }
    }

    #[test]
    fn expect_accepts_band_ids_and_written_results() {
        let i = Inputs::new()
            .choice("encephalopathy", "none")
            .choice("ascites", "mild")
            .num("bilirubin_mg_dl", 2.5)
            .num("albumin_g_dl", 3.0)
            .num("pt_prolonged_s", 3.0);
        let o = compute("child_pugh", &i).unwrap();
        assert!(expect_matches(&o, "child_pugh.b"));
        assert!(expect_matches(&o, "Child-Pugh B (8 points)"));
        assert!(!expect_matches(&o, "Child-Pugh B (9 points)"));
        assert!(!expect_matches(&o, "child_pugh.c"));
        let albi =
            compute("albi", &Inputs::new().num("bilirubin_mg_dl", 1.0).num("albumin_g_dl", 4.0)).unwrap();
        assert!(expect_matches(&albi, "ALBI −2.59, grade 2"));
        let bclc = compute(
            "bclc",
            &Inputs::new()
                .num("performance_status", 1.0)
                .num("child_pugh_points", 5.0)
                .num("nodules", 1.0)
                .num("largest_cm", 3.0),
        )
        .unwrap();
        assert!(expect_matches(&bclc, "Stage C"));
        assert!(!expect_matches(&bclc, "Stage B"));
    }

    // Issue #42: a number equal to the score must not carry a wrong band.
    #[test]
    fn expect_naming_another_band_fails() {
        let glasgow = |o: &Outcome| (o.band, o.score);
        let i = Inputs::new()
            .num("age_years", 55.0)
            .num("wbc_10e9_l", 16.0)
            .num("urea_mmol_l", 6.0)
            .num("inr", 1.6)
            .num("bilirubin_mg_dl", 5.0);
        let o = compute("glasgow_ah", &i).unwrap();
        assert_eq!(glasgow(&o), ("glasgow.high", Some(9.0)));
        assert!(expect_matches(&o, "9 or more"));
        assert!(expect_matches(&o, "GAHS 9"));
        assert!(!expect_matches(&o, "Below 9"));
        assert!(!expect_matches(&o, "glasgow.low"));

        let cp = Inputs::new()
            .choice("encephalopathy", "none")
            .choice("ascites", "mild")
            .num("bilirubin_mg_dl", 2.5)
            .num("albumin_g_dl", 2.5)
            .num("pt_prolonged_s", 3.0);
        let o = compute("child_pugh", &cp).unwrap();
        assert_eq!((o.band, o.score), ("child_pugh.b", Some(9.0)));
        assert!(expect_matches(&o, "Child-Pugh B (9 points)"));
        assert!(!expect_matches(&o, "Child-Pugh C (9 points)"));
        assert!(!expect_matches(&o, "Class C, 9 points"));

        let r = compute(
            "r_ratio",
            &Inputs::new()
                .num("alt_u_l", 200.0)
                .num("alt_uln_u_l", 40.0)
                .num("alp_u_l", 120.0)
                .num("alp_uln_u_l", 120.0),
        )
        .unwrap();
        // The book's bands (p. 129): exactly 5 is mixed, not the CIOMS
        // hepatocellular.
        assert_eq!(r.band, "r_ratio.mixed");
        assert!(expect_matches(&r, "R = 5, mixed"));
        assert!(!expect_matches(&r, "R = 5, hepatocellular"));
    }

    // Issue #252: naming the right band is a right typed answer.
    #[test]
    fn typed_band_names_are_accepted() {
        let i = Inputs::new()
            .choice("encephalopathy", "grade_3_4")
            .choice("ascites", "severe")
            .num("bilirubin_mg_dl", 5.0)
            .num("albumin_g_dl", 2.0)
            .num("pt_prolonged_s", 8.0);
        let o = compute("child_pugh", &i).unwrap();
        assert_eq!((o.band, o.score), ("child_pugh.c", Some(15.0)));
        for t in ["Child-Pugh C", "class c", "Child-Pugh class C", "C (15)", "15"] {
            assert!(answer_matches(&o, t), "{t}");
        }
        for t in ["Child-Pugh B", "class b", "14", "", "C"] {
            assert!(!answer_matches(&o, t), "{t}");
        }
        // The validator stays strict: content shows the number.
        assert!(!expect_matches(&o, "Child-Pugh C"));
    }

    #[test]
    fn longer_band_words_win() {
        assert_eq!(named_bands("Criteria not met", "kings.met"), ["kings.not_met"]);
        assert_eq!(named_bands("Criteria met: INR > 6.5", "kings.met"), ["kings.met"]);
        assert_eq!(named_bands("DF 40, not severe", "maddrey.severe"), ["maddrey.not_severe"]);
        assert_eq!(named_bands("Grade 2", "child_pugh.b"), Vec::<&str>::new());
        assert_eq!(named_bands("metabolic", "clichy.met"), Vec::<&str>::new());
    }

    #[test]
    fn keys_may_drop_their_unit_suffix() {
        let spec = spec("r_ratio").unwrap();
        let raw: BTreeMap<String, CalcValue> =
            [("alt", 120.0), ("alt_uln", 40.0), ("alp", 600.0), ("alp_uln_u_l", 120.0)]
                .into_iter()
                .map(|(k, v)| (k.to_owned(), CalcValue::Number(v)))
                .collect();
        let i = Inputs::for_spec(spec, &raw);
        assert!(unknown_inputs(spec, &i).is_empty());
        let o = (spec.compute)(&i).unwrap();
        assert!(expect_matches(&o, "R = 0.6, cholestatic"));
    }

    #[test]
    fn numbers_match_at_the_written_precision() {
        let i = Inputs::new()
            .num("age_years", 45.0)
            .num("ast_u_l", 30.0)
            .num("alt_u_l", 36.0)
            .num("platelets_10e9_l", 250.0);
        let o = compute("fib4", &i).unwrap();
        assert!(expect_matches(&o, "FIB-4 0.9"));
        assert!(expect_matches(&o, "FIB-4 0.90"));
        assert!(!expect_matches(&o, "FIB-4 1.9"));
    }

    #[test]
    fn unknown_inputs_are_reported() {
        let i = Inputs::new().num("bilirubin", 2.0).num("albumin_g_dl", 3.0);
        assert_eq!(unknown_inputs(spec("albi").unwrap(), &i), vec!["bilirubin"]);
    }

    #[test]
    fn registry_matches_the_content_schema_list() {
        let mut ids: Vec<_> = all().iter().map(|s| s.id).collect();
        let mut listed = crate::content::CALCULATORS.to_vec();
        ids.sort_unstable();
        listed.sort_unstable();
        assert_eq!(ids, listed);
    }

    #[test]
    fn the_reference_doc_is_current() {
        let doc = include_str!("../../../../docs/CALCULATORS.md");
        assert!(doc == reference_markdown(), "docs/CALCULATORS.md is stale: run the print_reference test");
    }

    #[test]
    #[ignore = "prints docs/CALCULATORS.md"]
    fn print_reference() {
        print!("{}", reference_markdown());
    }

    #[test]
    fn inputs_parse_from_yaml_shapes() {
        let i = Inputs::new().choice("x", "2.5").choice("f", "yes");
        assert_eq!(i.get_num("x"), Ok(2.5));
        assert_eq!(i.get_flag("f"), Ok(true));
        assert!(Inputs::new().num("x", f64::NAN).get_num("x").is_err());
        assert!(Inputs::new().num("x", 0.0).get_pos("x").is_err());
    }
}
