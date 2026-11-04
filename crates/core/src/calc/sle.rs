//! Classification criteria for systemic lupus erythematosus.
//!
//! - 1997 ACR (the Rheumatology book's main set, Lecture 19, p. 322): 4 or
//!   more of 11. The book's heading reads ">4 of 11"; the published rule
//!   (Hochberg 1997) is 4 or more, which is what is used here.
//! - 2019 EULAR/ACR (Aringer 2019), printed with its points in the
//!   Nephrology book, Lecture 14, p. 448: ANA 1:80 or more as the entry,
//!   10 or more points, at least one clinical domain, and only the highest
//!   item in each domain counts.

use super::{Area, CalcError, CalcSpec, CalcValue, Guide, Inputs, Outcome, Part, choice, flag};

const ACR_1997: &[(&str, &str)] = &[
    ("malar_rash", "Malar rash"),
    ("discoid_rash", "Discoid rash"),
    ("photosensitivity", "Photosensitivity"),
    ("oral_ulcers", "Oral ulcers"),
    ("arthritis", "Arthritis"),
    ("serositis", "Serositis"),
    ("renal", "Renal disorder"),
    ("neurological", "Neurological disorder"),
    ("haematological", "Haematological disorder"),
    ("immunological", "Immunological disorder"),
    ("ana", "ANA positive"),
];

pub const SLE_1997: CalcSpec = CalcSpec {
    id: "sle_acr_1997",
    name: "SLE 1997 ACR criteria",
    area: Area::Rheum,
    fields: &[
        flag("malar_rash", "Malar rash"),
        flag("discoid_rash", "Discoid rash"),
        flag("photosensitivity", "Photosensitivity"),
        flag("oral_ulcers", "Oral ulcers"),
        flag("arthritis", "Arthritis"),
        flag("serositis", "Serositis"),
        flag("renal", "Renal disorder").hint("Proteinuria > 0.5 g/day, or cells and/or casts."),
        flag("neurological", "Neurological disorder").hint("Seizure or psychosis."),
        flag("haematological", "Haematological disorder")
            .hint("Haemolytic anaemia, leukopenia, lymphopenia or thrombocytopenia."),
        flag("immunological", "Immunological disorder").hint("Anti-dsDNA, anti-Sm or anticardiolipin."),
        flag("ana", "ANA positive"),
    ],
    source: "Hochberg, Arthritis Rheum 1997 (ACR, revising Tan 1982); book Rheumatology Lecture 19, p. 322",
    compute: sle_1997,
};

fn sle_1997(i: &Inputs) -> Result<Outcome, CalcError> {
    let mut parts = Vec::with_capacity(ACR_1997.len());
    for (key, label) in ACR_1997 {
        parts.push(Part::met(*label, i.get_flag(key)?));
    }
    let n = parts.iter().filter(|p| p.points > 0.0).count() as f64;
    let (band, label, meaning) = if n >= 4.0 {
        ("sle_acr_1997.met", "4 or more", "Classified as SLE (4 or more of the 11 criteria).")
    } else {
        ("sle_acr_1997.not_met", "Below 4", "Not classified as SLE: fewer than 4 of 11.")
    };
    Ok(Outcome {
        score: Some(n),
        display: format!("{n} / 11"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(322),
        parts,
        notes: Vec::new(),
    })
}

const CUTANEOUS: &[(&str, &str)] = &[
    ("none", "None (0)"),
    ("alopecia", "Non-scarring alopecia (2)"),
    ("oral_ulcers", "Oral ulcers (2)"),
    ("subacute_discoid", "Subacute cutaneous or discoid lupus (4)"),
    ("acute_cutaneous", "Acute cutaneous lupus (6)"),
];
const NEUROLOGIC: &[(&str, &str)] = &[
    ("none", "None (0)"),
    ("delirium", "Delirium (2)"),
    ("psychosis", "Psychosis (3)"),
    ("seizure", "Seizure (5)"),
];
const SEROSAL: &[(&str, &str)] = &[
    ("none", "None (0)"),
    ("effusion", "Pleural or pericardial effusion (5)"),
    ("pericarditis", "Acute pericarditis (6)"),
];
const HAEMATOLOGIC: &[(&str, &str)] = &[
    ("none", "None (0)"),
    ("leukopenia", "Leukopenia (3)"),
    ("thrombocytopenia", "Thrombocytopenia (4)"),
    ("haemolysis", "Autoimmune haemolysis (4)"),
];
const RENAL: &[(&str, &str)] = &[
    ("none", "None (0)"),
    ("proteinuria", "Proteinuria > 0.5 g/24 h (4)"),
    ("class_2_5", "Biopsy: class II or V lupus nephritis (8)"),
    ("class_3_4", "Biopsy: class III or IV lupus nephritis (10)"),
];
const COMPLEMENT: &[(&str, &str)] =
    &[("none", "Normal (0)"), ("c3_or_c4", "Low C3 or low C4 (3)"), ("c3_and_c4", "Low C3 and low C4 (4)")];

pub const SLE_2019: CalcSpec = CalcSpec {
    id: "sle_eular_acr_2019",
    name: "SLE 2019 EULAR/ACR criteria",
    area: Area::Rheum,
    fields: &[
        flag("ana_1_80", "ANA 1:80 or more, ever").hint("The entry criterion."),
        flag("fever", "Fever"),
        choice("cutaneous", "Skin and mucosa", CUTANEOUS).optional(),
        flag("arthritis", "Synovitis or tenderness in 2 or more joints"),
        choice("neurologic", "Neurological", NEUROLOGIC).optional(),
        choice("serosal", "Serosal", SEROSAL).optional(),
        choice("haematologic", "Haematological", HAEMATOLOGIC).optional(),
        choice("renal", "Renal", RENAL).optional(),
        flag("antiphospholipid", "Antiphospholipid antibodies")
            .hint("Anticardiolipin IgG > 40 GPL, anti-β2GP1 IgG > 40 units, or lupus anticoagulant."),
        choice("complement", "Complement", COMPLEMENT).optional(),
        flag("specific_antibody", "Anti-dsDNA or anti-Sm antibody"),
    ],
    source: "Aringer et al., Ann Rheum Dis / Arthritis Rheumatol 2019 (EULAR/ACR); book Nephrology Lecture 14, p. 448",
    compute: sle_2019,
};

/// Points of an optional one-of-a-domain choice; unset counts as none.
fn domain(i: &Inputs, key: &'static str, points: &[(&str, f64)]) -> Result<f64, CalcError> {
    match i.0.get(key) {
        None => return Ok(0.0),
        Some(CalcValue::Text(s)) if s.trim().is_empty() => return Ok(0.0),
        Some(_) => {}
    }
    let ids: Vec<&str> = points.iter().map(|(id, _)| *id).collect();
    let picked = i.get_choice(key, &ids)?;
    Ok(points.iter().find(|(id, _)| *id == picked).map_or(0.0, |(_, p)| *p))
}

fn flag_points(i: &Inputs, key: &'static str, points: f64) -> Result<f64, CalcError> {
    Ok(if i.get_flag(key)? { points } else { 0.0 })
}

fn sle_2019(i: &Inputs) -> Result<Outcome, CalcError> {
    let clinical = vec![
        Part::new("Constitutional: fever", flag_points(i, "fever", 2.0)?),
        Part::new(
            "Cutaneous",
            domain(
                i,
                "cutaneous",
                &[
                    ("none", 0.0),
                    ("alopecia", 2.0),
                    ("oral_ulcers", 2.0),
                    ("subacute_discoid", 4.0),
                    ("acute_cutaneous", 6.0),
                ],
            )?,
        ),
        Part::new("Arthritis", flag_points(i, "arthritis", 6.0)?),
        Part::new(
            "Neurologic",
            domain(
                i,
                "neurologic",
                &[("none", 0.0), ("delirium", 2.0), ("psychosis", 3.0), ("seizure", 5.0)],
            )?,
        ),
        Part::new(
            "Serosal",
            domain(i, "serosal", &[("none", 0.0), ("effusion", 5.0), ("pericarditis", 6.0)])?,
        ),
        Part::new(
            "Haematologic",
            domain(
                i,
                "haematologic",
                &[("none", 0.0), ("leukopenia", 3.0), ("thrombocytopenia", 4.0), ("haemolysis", 4.0)],
            )?,
        ),
        Part::new(
            "Renal",
            domain(
                i,
                "renal",
                &[("none", 0.0), ("proteinuria", 4.0), ("class_2_5", 8.0), ("class_3_4", 10.0)],
            )?,
        ),
    ];
    let immunologic = vec![
        Part::new("Antiphospholipid antibodies", flag_points(i, "antiphospholipid", 2.0)?),
        Part::new(
            "Complement",
            domain(i, "complement", &[("none", 0.0), ("c3_or_c4", 3.0), ("c3_and_c4", 4.0)])?,
        ),
        Part::new("Anti-dsDNA or anti-Sm", flag_points(i, "specific_antibody", 6.0)?),
    ];
    let clinical_points: f64 = clinical.iter().map(|p| p.points).sum();
    let mut parts = clinical;
    parts.extend(immunologic);
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let ana = i.get_flag("ana_1_80")?;
    let mut notes = vec!["Count an item only when nothing more likely explains it.".to_owned()];
    if !ana {
        notes.push("Without ANA 1:80 or more the patient cannot be classified as SLE.".to_owned());
    }
    if total >= 10.0 && clinical_points == 0.0 {
        notes.push("Classification needs points from at least one clinical domain.".to_owned());
    }
    let (band, label, meaning) = if ana && total >= 10.0 && clinical_points > 0.0 {
        ("sle_eular_acr_2019.met", "10 or more", "Classified as SLE.")
    } else {
        ("sle_eular_acr_2019.not_met", "Not met", "Not classified as SLE.")
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{total} points"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(448),
        parts,
        notes,
    })
}

const SLE_MET: Guide = Guide {
    next: &[
        "Measure activity (SLEDAI) and screen organs: urine for protein and casts with urine protein:creatinine ratio, CBC, creatinine, C3, C4, anti-dsDNA.",
        "Check antiphospholipid antibodies (thrombosis and pregnancy risk).",
        "Hydroxychloroquine for every patient (eye screening); steroids at the lowest dose; add immunosuppression by organ (mycophenolate or cyclophosphamide for class III/IV nephritis).",
        "Kidney biopsy if proteinuria ≥ 0.5 g/day or an active sediment.",
    ],
    dd: &[
        "Drug-induced lupus (hydralazine, procainamide, isoniazid, anti-TNF: anti-histone +, no kidney or CNS disease)",
        "Mixed connective tissue disease (anti-U1 RNP), Sjögren's, undifferentiated CTD",
        "Rheumatoid arthritis (erosive, ACPA)",
        "Viral infection (parvovirus B19, HIV, EBV) causing cytopenias and positive ANA",
        "TTP or antiphospholipid syndrome when cytopenias and thrombosis lead",
    ],
};

const SLE_NOT_MET: Guide = Guide {
    next: &[
        "Classification criteria are not diagnostic: if suspicion is high, treat the organ involved and repeat the tests over time.",
        "A low-titre ANA alone is common in healthy people; check anti-dsDNA, anti-Sm, C3 and C4.",
    ],
    dd: &[
        "Undifferentiated connective tissue disease",
        "Early or incomplete SLE",
        "Sjögren's syndrome, MCTD",
        "Fibromyalgia with an incidental positive ANA",
    ],
};

/// Next step and differential diagnosis for each result.
pub const GUIDES: &[(&str, Guide)] = &[
    ("sle_acr_1997.met", SLE_MET),
    ("sle_acr_1997.not_met", SLE_NOT_MET),
    ("sle_eular_acr_2019.met", SLE_MET),
    ("sle_eular_acr_2019.not_met", SLE_NOT_MET),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_of_eleven() {
        let three = Inputs::new().flag("malar_rash", true).flag("arthritis", true).flag("ana", true);
        let o = sle_1997(&three).unwrap();
        assert_eq!((o.score, o.band), (Some(3.0), "sle_acr_1997.not_met"));
        let o = sle_1997(&three.flag("renal", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(4.0), "sle_acr_1997.met"));
    }

    #[test]
    fn eular_2019_highest_per_domain_and_entry() {
        // Class IV nephritis alone is 10 points, a clinical domain.
        let renal = Inputs::new().flag("ana_1_80", true).choice("renal", "class_3_4");
        let o = sle_2019(&renal).unwrap();
        assert_eq!((o.score, o.band), (Some(10.0), "sle_eular_acr_2019.met"));
        // Without ANA it is never classified.
        let o = sle_2019(&renal.clone().flag("ana_1_80", false)).unwrap();
        assert_eq!(o.band, "sle_eular_acr_2019.not_met");
        // Immunological points alone are not enough.
        let lab = Inputs::new()
            .flag("ana_1_80", true)
            .flag("specific_antibody", true)
            .choice("complement", "c3_and_c4")
            .flag("antiphospholipid", true);
        let o = sle_2019(&lab).unwrap();
        assert_eq!((o.score, o.band), (Some(12.0), "sle_eular_acr_2019.not_met"));
        // Fever + leukopenia + low C3 = 8: not met.
        let o = sle_2019(
            &Inputs::new()
                .flag("ana_1_80", true)
                .flag("fever", true)
                .choice("haematologic", "leukopenia")
                .choice("complement", "c3_or_c4"),
        )
        .unwrap();
        assert_eq!((o.score, o.band), (Some(8.0), "sle_eular_acr_2019.not_met"));
        // Unset and empty choices count as none.
        assert_eq!(sle_2019(&Inputs::new().choice("renal", "")).unwrap().score, Some(0.0));
        assert!(sle_2019(&Inputs::new().choice("renal", "class_9")).is_err());
    }
}
