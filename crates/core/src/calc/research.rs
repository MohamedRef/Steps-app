//! Basics of Research: measures of disease frequency, association, effect
//! and diagnostic accuracy.
//!
//! Source: the course slides (steps/courses/res/). Lecture 2, Measures of
//! disease frequency: incidence (slide 25), attack rate ×100 (26–29),
//! point/period prevalence (31), P = I × D (37), secondary attack rate
//! = secondary cases / susceptible contacts ×100 (46–49), PMR ×100 (59),
//! CFR ×100 (60), IMR and neonatal mortality per 1000 live births (63–64).
//! Lecture 5, Analytic studies: the 2×2 table a/b exposed, c/d not exposed;
//! OR = ad/bc (32), RR = Ie/Io and ARP = (Ie − Io)/Ie ×100 (50–51), RR > 1
//! risk factor, = 1 no association, < 1 protective (53). NNT = 1/ARR and the
//! accuracy measures follow the standard definitions taught in Lectures 6
//! and 14. `page` stays `None`: these pages are slides, not book pages.

use super::{Area, CalcError, CalcSpec, Field, Inputs, Outcome, Part, choice, num, show};

const COUNT_MAX: f64 = 1.0e9;

const fn count(key: &'static str, label: &'static str) -> Field {
    num(key, label, "", 0.0, COUNT_MAX, 1.0)
}

/// A count, which must be a whole number and not below zero.
fn get_count(i: &Inputs, key: &'static str) -> Result<f64, CalcError> {
    let v = i.get_num(key)?;
    if v < 0.0 || v.fract() != 0.0 {
        return Err(CalcError::Invalid { field: key, reason: "must be a whole number, 0 or more" });
    }
    Ok(v)
}

/// A denominator count: above zero.
fn get_denominator(i: &Inputs, key: &'static str) -> Result<f64, CalcError> {
    let v = get_count(i, key)?;
    if v > 0.0 { Ok(v) } else { Err(CalcError::Invalid { field: key, reason: "must be above zero" }) }
}

const PER: &[(&str, &str)] =
    &[("100", "per 100 (%)"), ("1000", "per 1,000"), ("10000", "per 10,000"), ("100000", "per 100,000")];

fn get_per(i: &Inputs) -> Result<f64, CalcError> {
    let p =
        i.get_option("per", &[("100", &["percent", "%"]), ("1000", &[]), ("10000", &[]), ("100000", &[])])?;
    Ok(p.parse().unwrap_or(1000.0))
}

fn per_label(per: f64) -> String {
    if per == 100.0 { "%".to_owned() } else { format!("per {}", group(per)) }
}

/// 100000 → "100,000".
fn group(v: f64) -> String {
    let s = format!("{v:.0}");
    let mut out = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A rate: `n / d × per`, rounded to two decimals.
fn rate(
    i: &Inputs,
    (num_key, num_label): (&'static str, &'static str),
    (den_key, den_label): (&'static str, &'static str),
    per: f64,
    band: &'static str,
    label: &'static str,
    meaning: &str,
) -> Result<Outcome, CalcError> {
    let n = get_count(i, num_key)?;
    let d = get_denominator(i, den_key)?;
    if n > d {
        return Err(CalcError::Invalid { field: num_key, reason: "can't be more than the denominator" });
    }
    let v = round2(n / d * per);
    let unit = per_label(per);
    let display = if per == 100.0 { format!("{}%", show2(v)) } else { format!("{} {unit}", show2(v)) };
    Ok(Outcome {
        score: Some(v),
        display,
        band,
        label,
        meaning: meaning.to_owned(),
        page: None,
        parts: vec![Part::new(num_label, n), Part::new(den_label, d)],
        notes: Vec::new(),
    })
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Two decimals, without trailing zeros: 61.33, 10, 4.9.
fn show2(v: f64) -> String {
    let s = show(v, 2);
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

pub const INCIDENCE: CalcSpec = CalcSpec {
    id: "incidence",
    name: "Incidence rate",
    area: Area::Research,
    fields: &[
        count("new_cases", "New cases during the period"),
        count("population_at_risk", "Population at risk (mid-period)"),
        choice("per", "Expressed", PER),
    ],
    source: "Research L02: measures of disease frequency, slide 25",
    compute: |i| {
        let per = get_per(i)?;
        rate(
            i,
            ("new_cases", "New cases"),
            ("population_at_risk", "Population at risk"),
            per,
            "incidence.rate",
            "Incidence rate",
            "New cases in a population at risk over a period: the risk of getting the disease.",
        )
    },
};

pub const PREVALENCE: CalcSpec = CalcSpec {
    id: "prevalence",
    name: "Prevalence",
    area: Area::Research,
    fields: &[
        count("existing_cases", "Existing (old + new) cases"),
        count("population", "Total population (mid-point or mid-period)"),
        choice("per", "Expressed", PER),
    ],
    source: "Research L02: measures of disease frequency, slide 31",
    compute: |i| {
        let per = get_per(i)?;
        rate(
            i,
            ("existing_cases", "Existing cases"),
            ("population", "Population"),
            per,
            "prevalence.rate",
            "Prevalence",
            "All existing cases at a point or over a period: the burden of illness needing care.",
        )
    },
};

pub const PREVALENCE_ID: CalcSpec = CalcSpec {
    id: "prevalence_id",
    name: "Prevalence = Incidence × Duration",
    area: Area::Research,
    fields: &[
        num("incidence", "Incidence", "per 1,000 per year", 0.0, 1000.0, 0.1),
        num("duration_years", "Mean duration of disease", "years", 0.0, 100.0, 0.1),
    ],
    source: "Research L02: measures of disease frequency, slide 37",
    compute: |i| {
        let inc = i.get_num("incidence")?;
        let dur = i.get_num("duration_years")?;
        if inc < 0.0 || dur < 0.0 {
            return Err(CalcError::Invalid { field: "incidence", reason: "can't be below zero" });
        }
        let p = round2(inc * dur);
        Ok(Outcome {
            score: Some(p),
            display: format!("{} per 1,000", show2(p)),
            band: "prevalence_id.prevalence",
            label: "Prevalence (P = I × D)",
            meaning: "A long-lasting disease piles up: low incidence can still give a high prevalence."
                .to_owned(),
            page: None,
            parts: vec![Part::new("Incidence", inc), Part::new("Duration (years)", dur)],
            notes: vec!["Holds in a steady state, when incidence and duration stay stable.".to_owned()],
        })
    },
};

pub const ATTACK_RATE: CalcSpec = CalcSpec {
    id: "attack_rate",
    name: "Attack rate",
    area: Area::Research,
    fields: &[count("cases", "Cases during the outbreak"), count("at_risk", "Population at risk")],
    source: "Research L02: measures of disease frequency, slides 26–29",
    compute: |i| {
        rate(
            i,
            ("cases", "Cases"),
            ("at_risk", "At risk"),
            100.0,
            "attack_rate.rate",
            "Attack rate (%)",
            "An incidence rate for a narrow population over a short time, as in an outbreak.",
        )
    },
};

pub const SAR: CalcSpec = CalcSpec {
    id: "sar",
    name: "Secondary attack rate",
    area: Area::Research,
    fields: &[
        count("secondary_cases", "Secondary cases (within one incubation period)"),
        count("contacts", "Contacts (household members, all)"),
        count("primary_cases", "Primary cases among them").hint("Left out of the susceptibles"),
    ],
    source: "Research L02: measures of disease frequency, slides 46–49",
    compute: |i| {
        let secondary = get_count(i, "secondary_cases")?;
        let contacts = get_count(i, "contacts")?;
        let primary = get_count(i, "primary_cases")?;
        let susceptible = contacts - primary;
        if susceptible <= 0.0 {
            return Err(CalcError::Invalid {
                field: "contacts",
                reason: "must be more than the primary cases",
            });
        }
        if secondary > susceptible {
            return Err(CalcError::Invalid {
                field: "secondary_cases",
                reason: "can't be more than the susceptibles",
            });
        }
        let v = round2(secondary / susceptible * 100.0);
        Ok(Outcome {
            score: Some(v),
            display: format!("{}%", show2(v)),
            band: "sar.rate",
            label: "Secondary attack rate (%)",
            meaning: "How easily the disease spreads from primary cases to their susceptible contacts."
                .to_owned(),
            page: None,
            parts: vec![
                Part::new("Secondary cases", secondary),
                Part::new("Susceptibles (contacts − primary cases)", susceptible),
            ],
            notes: vec![
                "Count within one incubation period; immune contacts are not susceptible.".to_owned(),
            ],
        })
    },
};

pub const CFR: CalcSpec = CalcSpec {
    id: "cfr",
    name: "Case fatality rate",
    area: Area::Research,
    fields: &[count("deaths", "Deaths from the disease"), count("cases", "People with the disease")],
    source: "Research L02: measures of disease frequency, slide 60",
    compute: |i| {
        rate(
            i,
            ("deaths", "Deaths"),
            ("cases", "Cases"),
            100.0,
            "cfr.rate",
            "Case fatality rate (%)",
            "The share of cases who die: a measure of how severe (killing) an acute disease is.",
        )
    },
};

pub const PMR: CalcSpec = CalcSpec {
    id: "pmr",
    name: "Proportionate mortality ratio",
    area: Area::Research,
    fields: &[count("cause_deaths", "Deaths from the cause"), count("all_deaths", "Deaths from all causes")],
    source: "Research L02: measures of disease frequency, slide 59",
    compute: |i| {
        rate(
            i,
            ("cause_deaths", "Deaths from the cause"),
            ("all_deaths", "All deaths"),
            100.0,
            "pmr.ratio",
            "Proportionate mortality (%)",
            "The cause's share of all deaths. It needs no population count.",
        )
    },
};

pub const IMR: CalcSpec = CalcSpec {
    id: "imr",
    name: "Infant mortality rate",
    area: Area::Research,
    fields: &[
        count("infant_deaths", "Deaths under one year"),
        count("live_births", "Live births, same year"),
    ],
    source: "Research L02: measures of disease frequency, slide 63",
    compute: |i| {
        rate(
            i,
            ("infant_deaths", "Infant deaths"),
            ("live_births", "Live births"),
            1000.0,
            "imr.rate",
            "Infant mortality rate (per 1,000 live births)",
            "Infant deaths per 1,000 live births in a year.",
        )
    },
};

pub const NMR: CalcSpec = CalcSpec {
    id: "nmr",
    name: "Neonatal mortality rate",
    area: Area::Research,
    fields: &[
        count("neonatal_deaths", "Deaths in the first 28 days"),
        count("live_births", "Live births, same year"),
    ],
    source: "Research L02: measures of disease frequency, slide 64",
    compute: |i| {
        rate(
            i,
            ("neonatal_deaths", "Neonatal deaths"),
            ("live_births", "Live births"),
            1000.0,
            "nmr.rate",
            "Neonatal mortality rate (per 1,000 live births)",
            "Deaths from birth to 28 days per 1,000 live births in a year.",
        )
    },
};

/// The 2×2 table, as the slides draw it: a/b exposed (disease yes/no or
/// case/control), c/d not exposed.
const TABLE: &[Field] = &[
    Field { hint: "Exposed, disease (or case)", ..count("a", "a") },
    Field { hint: "Exposed, no disease (or control)", ..count("b", "b") },
    Field { hint: "Not exposed, disease (or case)", ..count("c", "c") },
    Field { hint: "Not exposed, no disease (or control)", ..count("d", "d") },
];

fn table(i: &Inputs) -> Result<[f64; 4], CalcError> {
    Ok([get_count(i, "a")?, get_count(i, "b")?, get_count(i, "c")?, get_count(i, "d")?])
}

fn table_parts([a, b, c, d]: [f64; 4]) -> Vec<Part> {
    vec![Part::new("a", a), Part::new("b", b), Part::new("c", c), Part::new("d", d)]
}

/// RR or OR read the slides' way.
fn association(v: f64, prefix: &'static str) -> (&'static str, &'static str, &'static str) {
    let (band, label, meaning) = if v > 1.0 {
        ("risk", "Above 1: risk factor", "The exposure goes with more disease.")
    } else if v < 1.0 {
        ("protective", "Below 1: protective factor", "The exposure goes with less disease.")
    } else {
        ("none", "Equal to 1: no association", "The exposure makes no difference.")
    };
    let band = match (prefix, band) {
        ("rr", "risk") => "relative_risk.risk",
        ("rr", "protective") => "relative_risk.protective",
        ("rr", _) => "relative_risk.none",
        (_, "risk") => "odds_ratio.risk",
        (_, "protective") => "odds_ratio.protective",
        _ => "odds_ratio.none",
    };
    (band, label, meaning)
}

pub const RELATIVE_RISK: CalcSpec = CalcSpec {
    id: "relative_risk",
    name: "Relative risk (cohort)",
    area: Area::Research,
    fields: TABLE,
    source: "Research L05: analytic studies, slides 50–53",
    compute: |i| {
        let t @ [a, b, c, d] = table(i)?;
        if a + b == 0.0 || c + d == 0.0 {
            return Err(CalcError::Invalid { field: "a", reason: "each row needs people in it" });
        }
        let ie = a / (a + b);
        let io = c / (c + d);
        if io == 0.0 {
            return Err(CalcError::Invalid {
                field: "c",
                reason: "no disease among the unexposed: RR is undefined",
            });
        }
        let rr = round2(ie / io);
        let (band, label, meaning) = association(rr, "rr");
        let mut notes = vec![
            format!("Ie = a/(a+b) = {}%, Io = c/(c+d) = {}%", show2(ie * 100.0), show2(io * 100.0)),
            format!("Attributable risk Ie − Io = {}%", show2((ie - io) * 100.0)),
        ];
        if ie > 0.0 && ie > io {
            notes
                .push(format!("Attributable risk percent (Ie − Io)/Ie = {}%", show2((ie - io) / ie * 100.0)));
        }
        Ok(Outcome {
            score: Some(rr),
            display: format!("RR = {}", show2(rr)),
            band,
            label,
            meaning: meaning.to_owned(),
            page: None,
            parts: table_parts(t),
            notes,
        })
    },
};

pub const ODDS_RATIO: CalcSpec = CalcSpec {
    id: "odds_ratio",
    name: "Odds ratio (case-control)",
    area: Area::Research,
    fields: TABLE,
    source: "Research L05: analytic studies, slides 32–34",
    compute: |i| {
        let t @ [a, b, c, d] = table(i)?;
        if b * c == 0.0 {
            return Err(CalcError::Invalid { field: "b", reason: "b × c is 0: OR is undefined" });
        }
        let or = round2(a * d / (b * c));
        let (band, label, meaning) = association(or, "or");
        Ok(Outcome {
            score: Some(or),
            display: format!("OR = {}", show2(or)),
            band,
            label,
            meaning: meaning.to_owned(),
            page: None,
            parts: table_parts(t),
            notes: vec![format!("OR = ad / bc = ({a} × {d}) / ({b} × {c})")],
        })
    },
};

pub const ATTRIBUTABLE_RISK: CalcSpec = CalcSpec {
    id: "attributable_risk",
    name: "Attributable risk",
    area: Area::Research,
    fields: &[
        num("incidence_exposed", "Incidence among exposed (Ie)", "per 1,000", 0.0, 1000.0, 0.1),
        num("incidence_unexposed", "Incidence among non-exposed (Io)", "per 1,000", 0.0, 1000.0, 0.1),
    ],
    source: "Research L05: analytic studies, slide 51",
    compute: |i| {
        let ie = i.get_num("incidence_exposed")?;
        let io = i.get_num("incidence_unexposed")?;
        if ie < 0.0 || io < 0.0 {
            return Err(CalcError::Invalid { field: "incidence_exposed", reason: "can't be below zero" });
        }
        let ar = round2(ie - io);
        let mut notes = Vec::new();
        if ie > 0.0 {
            notes.push(format!(
                "Attributable risk percent = (Ie − Io)/Ie × 100 = {}%",
                show2((ie - io) / ie * 100.0)
            ));
        }
        Ok(Outcome {
            score: Some(ar),
            display: format!("{} per 1,000", show2(ar)),
            band: "attributable_risk.ar",
            label: "Attributable risk (Ie − Io)",
            meaning: "The extra disease among the exposed that the exposure accounts for.".to_owned(),
            page: None,
            parts: vec![Part::new("Ie", ie), Part::new("Io", io)],
            notes,
        })
    },
};

pub const NNT: CalcSpec = CalcSpec {
    id: "nnt",
    name: "NNT / NNH",
    area: Area::Research,
    fields: &[
        num("control_event_rate", "Event rate, control group (CER)", "%", 0.0, 100.0, 0.1),
        num("experimental_event_rate", "Event rate, treated group (EER)", "%", 0.0, 100.0, 0.1),
    ],
    source: "Research L06–L07: experimental studies (NNT = 1/ARR)",
    compute: |i| {
        let cer = i.get_num("control_event_rate")?;
        let eer = i.get_num("experimental_event_rate")?;
        if !(0.0..=100.0).contains(&cer) || !(0.0..=100.0).contains(&eer) {
            return Err(CalcError::Invalid { field: "control_event_rate", reason: "must be 0–100%" });
        }
        let arr = cer - eer;
        if arr == 0.0 {
            return Err(CalcError::Invalid {
                field: "experimental_event_rate",
                reason: "same rate in both groups: no NNT",
            });
        }
        let n = (100.0 / arr.abs()).ceil();
        let mut notes = vec![format!("ARR = CER − EER = {}%", show2(arr))];
        if cer > 0.0 {
            notes.push(format!("RRR = ARR / CER = {}%", show2(arr / cer * 100.0)));
        }
        notes.push("Rounded up to a whole patient.".to_owned());
        let (band, label, meaning) = if arr > 0.0 {
            ("nnt.benefit", "NNT: number needed to treat", "Treat this many patients to prevent one event.")
        } else {
            ("nnt.harm", "NNH: number needed to harm", "Treat this many patients and one extra is harmed.")
        };
        Ok(Outcome {
            score: Some(n),
            display: format!("{} {}", if arr > 0.0 { "NNT" } else { "NNH" }, show(n, 0)),
            band,
            label,
            meaning: meaning.to_owned(),
            page: None,
            parts: vec![Part::new("CER (%)", cer), Part::new("EER (%)", eer)],
            notes,
        })
    },
};

const MEASURES: &[(&str, &str)] = &[
    ("sensitivity", "Sensitivity"),
    ("specificity", "Specificity"),
    ("ppv", "Positive predictive value"),
    ("npv", "Negative predictive value"),
    ("accuracy", "Accuracy"),
    ("lr_positive", "LR+"),
    ("lr_negative", "LR−"),
];

pub const DIAGNOSTIC: CalcSpec = CalcSpec {
    id: "diagnostic_accuracy",
    name: "Diagnostic accuracy (2×2)",
    area: Area::Research,
    fields: &[
        Field { hint: "Test +, disease +", ..count("tp", "True positives (a)") },
        Field { hint: "Test +, disease −", ..count("fp", "False positives (b)") },
        Field { hint: "Test −, disease +", ..count("fn", "False negatives (c)") },
        Field { hint: "Test −, disease −", ..count("tn", "True negatives (d)") },
        choice("measure", "Answer with", MEASURES),
    ],
    source: "Research L14: measures of diagnostic accuracy",
    compute: |i| {
        let tp = get_count(i, "tp")?;
        let fp = get_count(i, "fp")?;
        let fn_ = get_count(i, "fn")?;
        let tn = get_count(i, "tn")?;
        let measure = i.get_option(
            "measure",
            &[
                ("sensitivity", &["sens"]),
                ("specificity", &["spec"]),
                ("ppv", &["positive_predictive_value"]),
                ("npv", &["negative_predictive_value"]),
                ("accuracy", &[]),
                ("lr_positive", &["lr+", "plr"]),
                ("lr_negative", &["lr-", "nlr"]),
            ],
        )?;
        let pct = |n: f64, d: f64| if d > 0.0 { Some(n / d * 100.0) } else { None };
        let sens = pct(tp, tp + fn_);
        let spec = pct(tn, tn + fp);
        let ppv = pct(tp, tp + fp);
        let npv = pct(tn, tn + fn_);
        let acc = pct(tp + tn, tp + fp + fn_ + tn);
        let lr_pos = match (sens, spec) {
            (Some(se), Some(sp)) if sp < 100.0 => Some(se / (100.0 - sp)),
            _ => None,
        };
        let lr_neg = match (sens, spec) {
            (Some(se), Some(sp)) if sp > 0.0 => Some((100.0 - se) / sp),
            _ => None,
        };
        let (value, band, label, meaning, is_pct) = match measure {
            "sensitivity" => (
                sens,
                "diagnostic_accuracy.sensitivity",
                "Sensitivity",
                "Of those with the disease, the share the test picks up: a/(a+c).",
                true,
            ),
            "specificity" => (
                spec,
                "diagnostic_accuracy.specificity",
                "Specificity",
                "Of those without the disease, the share the test clears: d/(b+d).",
                true,
            ),
            "ppv" => (
                ppv,
                "diagnostic_accuracy.ppv",
                "Positive predictive value",
                "Of the positives, the share who really have it: a/(a+b). Rises with prevalence.",
                true,
            ),
            "npv" => (
                npv,
                "diagnostic_accuracy.npv",
                "Negative predictive value",
                "Of the negatives, the share who really don't: d/(c+d). Falls as prevalence rises.",
                true,
            ),
            "accuracy" => (
                acc,
                "diagnostic_accuracy.accuracy",
                "Accuracy",
                "The share of all results that are right: (a+d)/total.",
                true,
            ),
            "lr_positive" => (
                lr_pos,
                "diagnostic_accuracy.lr_positive",
                "LR+",
                "How much a positive result raises the odds of disease: sensitivity/(1 − specificity).",
                false,
            ),
            _ => (
                lr_neg,
                "diagnostic_accuracy.lr_negative",
                "LR−",
                "How much a negative result lowers the odds of disease: (1 − sensitivity)/specificity.",
                false,
            ),
        };
        let v = round2(
            value.ok_or(CalcError::Invalid { field: "tp", reason: "a needed row or column is all zero" })?,
        );
        let all = |o: Option<f64>, unit: &str| {
            o.map_or("—".to_owned(), |v| format!("{}{unit}", show2(round2(v))))
        };
        let notes = vec![
            format!(
                "Sensitivity {} · Specificity {} · PPV {} · NPV {}",
                all(sens, "%"),
                all(spec, "%"),
                all(ppv, "%"),
                all(npv, "%")
            ),
            format!("Accuracy {} · LR+ {} · LR− {}", all(acc, "%"), all(lr_pos, ""), all(lr_neg, "")),
            format!("Prevalence in this table {}", all(pct(tp + fn_, tp + fp + fn_ + tn), "%")),
        ];
        Ok(Outcome {
            score: Some(v),
            display: if is_pct { format!("{}%", show2(v)) } else { show2(v) },
            band,
            label,
            meaning: meaning.to_owned(),
            page: None,
            parts: vec![
                Part::new("a (TP)", tp),
                Part::new("b (FP)", fp),
                Part::new("c (FN)", fn_),
                Part::new("d (TN)", tn),
            ],
            notes,
        })
    },
};

pub const Z_SCORE: CalcSpec = CalcSpec {
    id: "z_score",
    name: "Z-score",
    area: Area::Research,
    fields: &[
        num("value", "Observation (x)", "", -1.0e6, 1.0e6, 0.1),
        num("mean", "Mean (μ)", "", -1.0e6, 1.0e6, 0.1),
        num("sd", "Standard deviation (σ)", "", 0.0, 1.0e6, 0.1),
    ],
    source: "Research L16: normal distribution and Z-score",
    compute: |i| {
        let x = i.get_num("value")?;
        let mean = i.get_num("mean")?;
        let sd = i.get_pos("sd")?;
        let z = round2((x - mean) / sd);
        let (band, label, meaning) = if z.abs() > 1.96 {
            ("z_score.outside_95", "Beyond ±1.96", "Outside the middle 95% of a normal distribution.")
        } else {
            ("z_score.within_95", "Within ±1.96", "Inside the middle 95% of a normal distribution.")
        };
        Ok(Outcome {
            score: Some(z),
            display: format!("z = {}", show2(z)),
            band,
            label,
            meaning: meaning.to_owned(),
            page: None,
            parts: vec![Part::new("x", x), Part::new("Mean", mean), Part::new("SD", sd)],
            notes: vec!["±1 SD holds about 68%, ±2 SD about 95%, ±3 SD about 99.7%.".to_owned()],
        })
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    fn run(spec: &CalcSpec, i: Inputs) -> Outcome {
        (spec.compute)(&i).unwrap()
    }

    fn t(a: f64, b: f64, c: f64, d: f64) -> Inputs {
        Inputs::new().num("a", a).num("b", b).num("c", c).num("d", d)
    }

    #[test]
    fn the_slides_worked_examples() {
        // L02 slide 29: 46 of 75 dinner guests → 61%.
        assert_eq!(
            run(&ATTACK_RATE, Inputs::new().num("cases", 46.0).num("at_risk", 75.0)).display,
            "61.33%"
        );
        // Slide 37: 10 per 1000 per year × 5 years = 50 per 1000.
        let o = run(&PREVALENCE_ID, Inputs::new().num("incidence", 10.0).num("duration_years", 5.0));
        assert_eq!(o.score, Some(50.0));
        // Slide 49: 5 secondary cases, 32 family members, 7 primary → 20%.
        let o = run(
            &SAR,
            Inputs::new().num("secondary_cases", 5.0).num("contacts", 32.0).num("primary_cases", 7.0),
        );
        assert_eq!(o.score, Some(20.0));
        // Slide 60: 5 deaths among 500 → 1%.
        assert_eq!(run(&CFR, Inputs::new().num("deaths", 5.0).num("cases", 500.0)).score, Some(1.0));
        // Slide 63: 270 infant deaths / 30,000 live births → 9 per 1000.
        let o = run(&IMR, Inputs::new().num("infant_deaths", 270.0).num("live_births", 30000.0));
        assert_eq!(o.score, Some(9.0));
        // Slide 64: 150 / 30,000 → 5.
        let o = run(&NMR, Inputs::new().num("neonatal_deaths", 150.0).num("live_births", 30000.0));
        assert_eq!(o.score, Some(5.0));
        // L05 slide 34: alcohol and laryngeal cancer, OR = 160×110 / 90×40 = 4.89.
        let o = run(&ODDS_RATIO, t(160.0, 90.0, 40.0, 110.0));
        assert_eq!((o.score, o.band), (Some(4.89), "odds_ratio.risk"));
        // L05 slide 54: smoking and lung cancer, 20/200 vs 2/200 → RR 10.
        let o = run(&RELATIVE_RISK, t(20.0, 180.0, 2.0, 198.0));
        assert_eq!((o.score, o.band), (Some(10.0), "relative_risk.risk"));
        assert!(o.notes.iter().any(|n| n.contains("90%")), "{:?}", o.notes);
    }

    #[test]
    fn incidence_uses_the_chosen_multiplier() {
        let i = Inputs::new().num("new_cases", 200.0).num("population_at_risk", 210_000.0);
        assert_eq!(run(&INCIDENCE, i.clone().choice("per", "1000")).score, Some(0.95));
        assert_eq!(run(&INCIDENCE, i.choice("per", "100000")).display, "95.24 per 100,000");
        assert!(
            (INCIDENCE.compute)(
                &Inputs::new().num("new_cases", 5.0).num("population_at_risk", 2.0).choice("per", "100")
            )
            .is_err()
        );
    }

    #[test]
    fn association_bands() {
        assert_eq!(run(&RELATIVE_RISK, t(10.0, 90.0, 10.0, 90.0)).band, "relative_risk.none");
        assert_eq!(run(&ODDS_RATIO, t(10.0, 90.0, 30.0, 70.0)).band, "odds_ratio.protective");
    }

    #[test]
    fn nnt_and_nnh() {
        let o = run(&NNT, Inputs::new().num("control_event_rate", 20.0).num("experimental_event_rate", 15.0));
        assert_eq!((o.score, o.band), (Some(20.0), "nnt.benefit"));
        let o = run(&NNT, Inputs::new().num("control_event_rate", 10.0).num("experimental_event_rate", 13.0));
        assert_eq!((o.score, o.band), (Some(34.0), "nnt.harm"));
    }

    #[test]
    fn diagnostic_accuracy() {
        let base = Inputs::new().num("tp", 90.0).num("fp", 40.0).num("fn", 10.0).num("tn", 860.0);
        assert_eq!(run(&DIAGNOSTIC, base.clone().choice("measure", "sensitivity")).score, Some(90.0));
        assert_eq!(run(&DIAGNOSTIC, base.clone().choice("measure", "specificity")).score, Some(95.56));
        assert_eq!(run(&DIAGNOSTIC, base.clone().choice("measure", "ppv")).score, Some(69.23));
        assert_eq!(run(&DIAGNOSTIC, base.clone().choice("measure", "npv")).score, Some(98.85));
        assert_eq!(run(&DIAGNOSTIC, base.choice("measure", "lr+")).score, Some(20.25));
    }

    #[test]
    fn z_score() {
        let o = run(&Z_SCORE, Inputs::new().num("value", 130.0).num("mean", 100.0).num("sd", 15.0));
        assert_eq!((o.score, o.band), (Some(2.0), "z_score.outside_95"));
    }
}
