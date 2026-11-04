//! Ann Arbor staging and the International Prognostic Index for lymphoma.
//!
//! Source: the Hematology book (Chapter 4, Lecture 12, pp. 192–193). Ann
//! Arbor: I a single lymph node group; II two or more groups on one side of
//! the diaphragm; III groups on both sides; IV extralymphatic sites such as
//! bone marrow or liver. A: no systemic symptoms; B: any one of night
//! sweats, fever or weight loss (10% in the past 6 months).
//!
//! IPI: the book (p. 193) lists extranodal disease, advanced stage, raised
//! LDH and age without cut-offs or points. The index itself (Shipp 1993)
//! gives 1 point each for age over 60, LDH above normal, ECOG performance
//! status 2 or more, Ann Arbor stage III–IV and more than one extranodal
//! site; 0–1 low, 2 low-intermediate, 3 high-intermediate, 4–5 high risk.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice, flag, num};

const SPREAD: &[(&str, &str)] = &[
    ("single_group", "A single lymph node group"),
    ("one_side", "2 or more groups, one side of the diaphragm"),
    ("both_sides", "Groups on both sides of the diaphragm"),
    ("extralymphatic", "Extralymphatic sites (e.g. bone marrow, liver)"),
];

pub const ANN_ARBOR: CalcSpec = CalcSpec {
    id: "ann_arbor",
    name: "Ann Arbor stage (lymphoma)",
    area: Area::Blood,
    fields: &[
        choice("spread", "Disease extent", SPREAD),
        flag("night_sweats", "Night sweats"),
        flag("fever", "Fever"),
        flag("weight_loss", "Weight loss > 10% in 6 months"),
    ],
    source: "Carbone 1971, Lugano 2014; book Hematology Lecture 12, pp. 192–193",
    compute: ann_arbor,
};

pub const IPI: CalcSpec = CalcSpec {
    id: "ipi_nhl",
    name: "International Prognostic Index (NHL)",
    area: Area::Blood,
    fields: &[
        num("age_years", "Age", "years", 15.0, 110.0, 1.0),
        flag("ldh_high", "LDH above normal"),
        flag("ecog_2_plus", "ECOG performance status 2 or more")
            .hint("2 = up and about over half the day but unable to work."),
        flag("stage_3_4", "Ann Arbor stage III or IV"),
        flag("extranodal_2_plus", "More than one extranodal site"),
    ],
    source: "Shipp 1993, NEJM (points and groups, not in the book); book Hematology Lecture 12, p. 193",
    compute: ipi,
};

fn ann_arbor(i: &Inputs) -> Result<Outcome, CalcError> {
    let spread = i.get_option(
        "spread",
        &[
            ("single_group", &["1", "i", "single"]),
            ("one_side", &["2", "ii", "same_side"]),
            ("both_sides", &["3", "iii"]),
            ("extralymphatic", &["4", "iv", "extranodal", "disseminated"]),
        ],
    )?;
    let symptoms = [
        Part::met("Night sweats", i.get_flag("night_sweats")?),
        Part::met("Fever", i.get_flag("fever")?),
        Part::met("Weight loss > 10% in 6 months", i.get_flag("weight_loss")?),
    ];
    let b = symptoms.iter().any(|p| p.met == Some(true));
    let stage = match spread {
        "single_group" => 1,
        "one_side" => 2,
        "both_sides" => 3,
        _ => 4,
    };
    let (band, label) = match (stage, b) {
        (1, false) => ("ann_arbor.1a", "Stage IA"),
        (1, true) => ("ann_arbor.1b", "Stage IB"),
        (2, false) => ("ann_arbor.2a", "Stage IIA"),
        (2, true) => ("ann_arbor.2b", "Stage IIB"),
        (3, false) => ("ann_arbor.3a", "Stage IIIA"),
        (3, true) => ("ann_arbor.3b", "Stage IIIB"),
        (_, false) => ("ann_arbor.4a", "Stage IVA"),
        (_, true) => ("ann_arbor.4b", "Stage IVB"),
    };
    let extent = SPREAD.iter().find(|(id, _)| *id == spread).map_or("", |(_, l)| l);
    let meaning = format!(
        "{extent}; {}. {}",
        if b { "B: systemic symptoms" } else { "A: no systemic symptoms" },
        if stage >= 3 { "Advanced stage: worse prognosis." } else { "Limited stage." },
    );
    let mut parts = vec![Part::new(format!("Extent: {extent}"), f64::from(stage))];
    parts.extend(symptoms);
    Ok(Outcome {
        score: Some(f64::from(stage)),
        display: label.to_owned(),
        band,
        label,
        meaning,
        page: Some(193),
        parts,
        notes: vec![
            "The E suffix (one extranodal site by direct spread) and X (bulk) are not entered here (Lugano 2014)."
                .to_owned(),
        ],
    })
}

fn ipi(i: &Inputs) -> Result<Outcome, CalcError> {
    let parts = vec![
        Part::met("Age > 60", i.get_num("age_years")? > 60.0),
        Part::met("LDH above normal", i.get_flag("ldh_high")?),
        Part::met("ECOG ≥ 2", i.get_flag("ecog_2_plus")?),
        Part::met("Stage III–IV", i.get_flag("stage_3_4")?),
        Part::met("> 1 extranodal site", i.get_flag("extranodal_2_plus")?),
    ];
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = match total as u8 {
        0 | 1 => ("ipi_nhl.low", "Low (0–1)", "Low risk: 5-year survival 73% (Shipp 1993)."),
        2 => ("ipi_nhl.low_int", "Low-intermediate (2)", "Low-intermediate risk: 5-year survival 51%."),
        3 => ("ipi_nhl.high_int", "High-intermediate (3)", "High-intermediate risk: 5-year survival 43%."),
        _ => ("ipi_nhl.high", "High (4–5)", "High risk: 5-year survival 26%."),
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{total}"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(193),
        parts,
        notes: vec![
            "The book lists extranodal disease, advanced stage, raised LDH and age as poor prognostic factors; the cut-offs, ECOG and the risk groups are from the original IPI (Shipp 1993)."
                .to_owned(),
            "The survival figures are from before rituximab; with R-CHOP every group does better (R-IPI, Sehn 2007)."
                .to_owned(),
        ],
    })
}

/// Next step and differential per stage (Lugano 2014, Cheson).
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "ann_arbor.1a",
        Guide {
            next: &[
                "PET-CT to confirm the disease is limited.",
                "Short chemotherapy with or without involved-site radiotherapy.",
                "Work out the prognosis: the IPI for aggressive NHL, risk factors for Hodgkin lymphoma.",
            ],
            dd: &[
                "Reactive nodes: local infection, EBV, CMV, toxoplasmosis, HIV.",
                "TB lymphadenitis, sarcoidosis, SLE.",
                "Drugs (phenytoin), metastatic carcinoma, CLL.",
                "An excision biopsy of the node decides, not an FNA.",
            ],
        },
    ),
    (
        "ann_arbor.1b",
        Guide {
            next: &[
                "PET-CT to confirm the disease is limited.",
                "Short chemotherapy with or without involved-site radiotherapy.",
                "Work out the prognosis: the IPI for aggressive NHL, risk factors for Hodgkin lymphoma.",
            ],
            dd: &[
                "TB, HIV, infectious mononucleosis or CMV, brucellosis.",
                "Sarcoidosis, SLE, adult-onset Still's disease.",
                "Metastatic carcinoma.",
                "An excision biopsy of the node decides, not an FNA.",
            ],
        },
    ),
    (
        "ann_arbor.2a",
        Guide {
            next: &[
                "PET-CT to confirm the disease is limited.",
                "Short chemotherapy with or without involved-site radiotherapy.",
                "Work out the prognosis: the IPI for aggressive NHL, risk factors for Hodgkin lymphoma.",
            ],
            dd: &[
                "Reactive nodes: local infection, EBV, CMV, toxoplasmosis, HIV.",
                "TB lymphadenitis, sarcoidosis, SLE.",
                "Drugs (phenytoin), metastatic carcinoma, CLL.",
                "An excision biopsy of the node decides, not an FNA.",
            ],
        },
    ),
    (
        "ann_arbor.2b",
        Guide {
            next: &[
                "PET-CT to confirm the disease is limited.",
                "Short chemotherapy with or without involved-site radiotherapy.",
                "Work out the prognosis: the IPI for aggressive NHL, risk factors for Hodgkin lymphoma.",
            ],
            dd: &[
                "TB, HIV, infectious mononucleosis or CMV, brucellosis.",
                "Sarcoidosis, SLE, adult-onset Still's disease.",
                "Metastatic carcinoma.",
                "An excision biopsy of the node decides, not an FNA.",
            ],
        },
    ),
    (
        "ann_arbor.3a",
        Guide {
            next: &[
                "PET-CT staging; in NHL a marrow biopsy if PET doesn't settle it.",
                "Systemic chemotherapy (R-CHOP for DLBCL, ABVD-based for Hodgkin).",
                "Work out the IPI (NHL); with a high IPI consider CNS prophylaxis (CNS-IPI).",
            ],
            dd: &[
                "Reactive nodes: local infection, EBV, CMV, toxoplasmosis, HIV.",
                "TB lymphadenitis, sarcoidosis, SLE.",
                "Drugs (phenytoin), metastatic carcinoma, CLL.",
                "An excision biopsy of the node decides, not an FNA.",
            ],
        },
    ),
    (
        "ann_arbor.3b",
        Guide {
            next: &[
                "PET-CT staging; in NHL a marrow biopsy if PET doesn't settle it.",
                "Systemic chemotherapy (R-CHOP for DLBCL, ABVD-based for Hodgkin).",
                "Work out the IPI (NHL); with a high IPI consider CNS prophylaxis (CNS-IPI).",
            ],
            dd: &[
                "TB, HIV, infectious mononucleosis or CMV, brucellosis.",
                "Sarcoidosis, SLE, adult-onset Still's disease.",
                "Metastatic carcinoma.",
                "An excision biopsy of the node decides, not an FNA.",
            ],
        },
    ),
    (
        "ann_arbor.4a",
        Guide {
            next: &[
                "PET-CT staging; in NHL a marrow biopsy if PET doesn't settle it.",
                "Systemic chemotherapy (R-CHOP for DLBCL, ABVD-based for Hodgkin).",
                "Work out the IPI (NHL); with a high IPI consider CNS prophylaxis (CNS-IPI).",
            ],
            dd: &[
                "Reactive nodes: local infection, EBV, CMV, toxoplasmosis, HIV.",
                "TB lymphadenitis, sarcoidosis, SLE.",
                "Drugs (phenytoin), metastatic carcinoma, CLL.",
                "An excision biopsy of the node decides, not an FNA.",
            ],
        },
    ),
    (
        "ann_arbor.4b",
        Guide {
            next: &[
                "PET-CT staging; in NHL a marrow biopsy if PET doesn't settle it.",
                "Systemic chemotherapy (R-CHOP for DLBCL, ABVD-based for Hodgkin).",
                "Work out the IPI (NHL); with a high IPI consider CNS prophylaxis (CNS-IPI).",
            ],
            dd: &[
                "TB, HIV, infectious mononucleosis or CMV, brucellosis.",
                "Sarcoidosis, SLE, adult-onset Still's disease.",
                "Metastatic carcinoma.",
                "An excision biopsy of the node decides, not an FNA.",
            ],
        },
    ),
    (
        "ipi_nhl.low",
        Guide {
            next: &["Standard R-CHOP; limited stage may need fewer cycles."],
            dd: &[
                "The IPI is for aggressive NHL such as DLBCL: follicular lymphoma uses the FLIPI, Hodgkin lymphoma the Hasenclever score, myeloma the ISS.",
                "Survival figures are pre-rituximab (Shipp 1993); the R-IPI (Sehn 2007) fits R-CHOP patients better.",
            ],
        },
    ),
    (
        "ipi_nhl.low_int",
        Guide {
            next: &["Standard R-CHOP."],
            dd: &[
                "The IPI is for aggressive NHL such as DLBCL: follicular lymphoma uses the FLIPI, Hodgkin lymphoma the Hasenclever score, myeloma the ISS.",
                "Survival figures are pre-rituximab (Shipp 1993); the R-IPI (Sehn 2007) fits R-CHOP patients better.",
            ],
        },
    ),
    (
        "ipi_nhl.high_int",
        Guide {
            next: &[
                "R-CHOP; consider a clinical trial.",
                "Work out the CNS-IPI; at high risk, CNS imaging and CSF.",
            ],
            dd: &[
                "The IPI is for aggressive NHL such as DLBCL: follicular lymphoma uses the FLIPI, Hodgkin lymphoma the Hasenclever score, myeloma the ISS.",
                "Survival figures are pre-rituximab (Shipp 1993); the R-IPI (Sehn 2007) fits R-CHOP patients better.",
            ],
        },
    ),
    (
        "ipi_nhl.high",
        Guide {
            next: &[
                "R-CHOP or a clinical trial.",
                "Work out the CNS-IPI; at high risk, CNS imaging, CSF and CNS prophylaxis.",
            ],
            dd: &[
                "The IPI is for aggressive NHL such as DLBCL: follicular lymphoma uses the FLIPI, Hodgkin lymphoma the Hasenclever score, myeloma the ISS.",
                "Survival figures are pre-rituximab (Shipp 1993); the R-IPI (Sehn 2007) fits R-CHOP patients better.",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn stage(spread: &str, fever: bool) -> &'static str {
        ann_arbor(&Inputs::new().choice("spread", spread).flag("fever", fever)).unwrap().band
    }

    #[test]
    fn ann_arbor_stages() {
        assert_eq!(stage("single_group", false), "ann_arbor.1a");
        assert_eq!(stage("one_side", true), "ann_arbor.2b");
        assert_eq!(stage("both_sides", false), "ann_arbor.3a");
        assert_eq!(stage("extralymphatic", true), "ann_arbor.4b");
        assert_eq!(stage("IV", false), "ann_arbor.4a");
        let o = ann_arbor(&Inputs::new().choice("spread", "both_sides").flag("weight_loss", true)).unwrap();
        assert_eq!(o.display, "Stage IIIB");
    }

    #[test]
    fn ipi_groups() {
        let run = |age: f64, ldh: bool, ecog: bool, st: bool, en: bool| {
            ipi(&Inputs::new()
                .num("age_years", age)
                .flag("ldh_high", ldh)
                .flag("ecog_2_plus", ecog)
                .flag("stage_3_4", st)
                .flag("extranodal_2_plus", en))
            .unwrap()
        };
        assert_eq!(run(60.0, true, false, false, false).band, "ipi_nhl.low");
        assert_eq!(run(61.0, true, false, false, false).band, "ipi_nhl.low_int");
        assert_eq!(run(70.0, true, true, false, false).band, "ipi_nhl.high_int");
        let o = run(70.0, true, true, true, true);
        assert_eq!((o.score, o.band), (Some(5.0), "ipi_nhl.high"));
    }
}
