//! 2022 ACR/EULAR classification criteria for granulomatosis with
//! polyangiitis (Robson 2022) and microscopic polyangiitis (Suppiah 2022).
//!
//! Source: the Rheumatology book, Lecture 8, p. 147, which prints both
//! tables with their points. A score of 5 or more classifies. The criteria
//! are applied once small- or medium-vessel vasculitis is diagnosed and its
//! mimics are excluded.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, flag};

type Item = (&'static str, &'static str, f64);

const GPA_ITEMS: &[Item] = &[
    ("nasal", "Nasal involvement", 3.0),
    ("cartilage", "Cartilaginous involvement", 2.0),
    ("hearing_loss", "Hearing loss", 1.0),
    ("c_anca_pr3", "c-ANCA or anti-PR3", 5.0),
    ("lung_nodules", "Lung nodules, mass or cavitation", 2.0),
    ("granuloma", "Granulomatous inflammation on biopsy", 2.0),
    ("sinus_imaging", "Sinus or mastoid inflammation on imaging", 1.0),
    ("pauci_immune_gn", "Pauci-immune glomerulonephritis", 1.0),
    ("p_anca_mpo", "p-ANCA or anti-MPO", -1.0),
    ("eosinophils", "Eosinophils ≥ 1 × 10⁹/L", -4.0),
];

const MPA_ITEMS: &[Item] = &[
    ("nasal", "Nasal involvement", -3.0),
    ("p_anca_mpo", "p-ANCA or anti-MPO", 6.0),
    ("lung_fibrosis", "Fibrosis or ILD on chest imaging", 3.0),
    ("pauci_immune_gn", "Pauci-immune glomerulonephritis", 3.0),
    ("c_anca_pr3", "c-ANCA or anti-PR3", -1.0),
    ("eosinophils", "Eosinophils ≥ 1 × 10⁹/L", -4.0),
];

pub const GPA_2022: CalcSpec = CalcSpec {
    id: "gpa_2022",
    name: "GPA 2022 ACR/EULAR criteria",
    area: Area::Rheum,
    fields: &[
        flag(
            "nasal",
            "Nasal bloody discharge, ulcers, crusting, congestion, blockage or septal perforation (+3)",
        ),
        flag(
            "cartilage",
            "Cartilage: ear or nose chondritis, hoarseness, stridor, endobronchial, saddle nose (+2)",
        ),
        flag("hearing_loss", "Conductive or sensorineural hearing loss (+1)"),
        flag("c_anca_pr3", "c-ANCA or anti-PR3 positive (+5)"),
        flag("lung_nodules", "Pulmonary nodules, mass or cavitation on chest imaging (+2)"),
        flag(
            "granuloma",
            "Granuloma, extravascular granulomatous inflammation or giant cells on biopsy (+2)",
        ),
        flag(
            "sinus_imaging",
            "Nasal/paranasal sinus inflammation, consolidation or effusion, or mastoiditis on imaging (+1)",
        ),
        flag("pauci_immune_gn", "Pauci-immune glomerulonephritis on biopsy (+1)"),
        flag("p_anca_mpo", "p-ANCA or anti-MPO positive (−1)"),
        flag("eosinophils", "Blood eosinophils ≥ 1 × 10⁹/L (−4)"),
    ],
    source: "Robson et al., Ann Rheum Dis 2022 (ACR/EULAR); book Rheumatology Lecture 8, p. 147",
    compute: gpa,
};

pub const MPA_2022: CalcSpec = CalcSpec {
    id: "mpa_2022",
    name: "MPA 2022 ACR/EULAR criteria",
    area: Area::Rheum,
    fields: &[
        flag(
            "nasal",
            "Nasal bloody discharge, ulcers, crusting, congestion, blockage or septal perforation (−3)",
        ),
        flag("p_anca_mpo", "p-ANCA or anti-MPO positive (+6)"),
        flag("lung_fibrosis", "Fibrosis or interstitial lung disease on chest imaging (+3)"),
        flag("pauci_immune_gn", "Pauci-immune glomerulonephritis on biopsy (+3)"),
        flag("c_anca_pr3", "c-ANCA or anti-PR3 positive (−1)"),
        flag("eosinophils", "Blood eosinophils ≥ 1 × 10⁹/L (−4)"),
    ],
    source: "Suppiah et al., Ann Rheum Dis 2022 (ACR/EULAR); book Rheumatology Lecture 8, p. 147",
    compute: mpa,
};

fn score(i: &Inputs, items: &[Item]) -> Result<(f64, Vec<Part>), CalcError> {
    let mut parts = Vec::with_capacity(items.len());
    for (key, label, points) in items {
        parts.push(Part::new(*label, if i.get_flag(key)? { *points } else { 0.0 }));
    }
    Ok((parts.iter().map(|p| p.points).sum(), parts))
}

fn outcome(
    total: f64,
    parts: Vec<Part>,
    met: (&'static str, &'static str),
    not_met: &'static str,
) -> Outcome {
    let (band, label, meaning) = if total >= 5.0 {
        (met.0, "5 or more", met.1)
    } else {
        (not_met, "Below 5", "Not classified by these criteria.")
    };
    Outcome {
        score: Some(total),
        display: format!("{total} points"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(147),
        parts,
        notes: vec![
            "Apply once small- or medium-vessel vasculitis is diagnosed and its mimics are excluded."
                .to_owned(),
        ],
    }
}

fn gpa(i: &Inputs) -> Result<Outcome, CalcError> {
    let (total, parts) = score(i, GPA_ITEMS)?;
    Ok(outcome(
        total,
        parts,
        ("gpa_2022.met", "Classified as granulomatosis with polyangiitis."),
        "gpa_2022.not_met",
    ))
}

fn mpa(i: &Inputs) -> Result<Outcome, CalcError> {
    let (total, parts) = score(i, MPA_ITEMS)?;
    Ok(outcome(total, parts, ("mpa_2022.met", "Classified as microscopic polyangiitis."), "mpa_2022.not_met"))
}

const AAV_NEXT: &[&str] = &[
    "Stage severity: organ- or life-threatening (glomerulonephritis, alveolar haemorrhage, mononeuritis) or not.",
    "Urine sediment, creatinine and chest CT; biopsy the easiest affected organ.",
    "Induction: glucocorticoids plus rituximab or cyclophosphamide; plasma exchange can be added for severe renal disease. Maintenance: rituximab (or azathioprine).",
];

/// Next step and differential diagnosis for each result.
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "gpa_2022.met",
        Guide {
            next: AAV_NEXT,
            dd: &[
                "Microscopic polyangiitis (MPO, no granulomas or ENT disease)",
                "EGPA (asthma, eosinophilia)",
                "Anti-GBM disease (pulmonary-renal syndrome, anti-GBM antibody)",
                "Tuberculosis, fungal infection or lymphoma with lung nodules",
                "Cocaine/levamisole-induced midline destruction (ANCA can be positive)",
            ],
        },
    ),
    (
        "gpa_2022.not_met",
        Guide {
            next: &[
                "Check the other small-vessel vasculitis criteria (MPA, EGPA) and exclude mimics.",
                "Repeat ANCA by ELISA (PR3 and MPO) and seek tissue.",
            ],
            dd: &[
                "Microscopic polyangiitis",
                "EGPA",
                "Infection (TB, endocarditis) or malignancy",
                "Sarcoidosis",
            ],
        },
    ),
    (
        "mpa_2022.met",
        Guide {
            next: AAV_NEXT,
            dd: &[
                "GPA (PR3, ENT disease, granulomas)",
                "Anti-GBM disease",
                "Lupus nephritis (ANA, low complement, immune deposits)",
                "Infective endocarditis (can give ANCA and glomerulonephritis)",
                "Drug-induced ANCA vasculitis (hydralazine, propylthiouracil)",
            ],
        },
    ),
    (
        "mpa_2022.not_met",
        Guide {
            next: &[
                "Check the GPA and EGPA criteria and exclude mimics.",
                "Kidney biopsy for any active sediment: pauci-immune vs immune-complex GN.",
            ],
            dd: &[
                "GPA",
                "EGPA",
                "Immune-complex GN (lupus, IgA vasculitis, cryoglobulinaemia)",
                "Anti-GBM disease",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpa_cut_off_and_negatives() {
        let o = gpa(&Inputs::new().flag("c_anca_pr3", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(5.0), "gpa_2022.met"));
        let o = gpa(&Inputs::new().flag("c_anca_pr3", true).flag("p_anca_mpo", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(4.0), "gpa_2022.not_met"));
        let o = gpa(&Inputs::new().flag("nasal", true).flag("lung_nodules", true).flag("eosinophils", true))
            .unwrap();
        assert_eq!((o.score, o.band), (Some(1.0), "gpa_2022.not_met"));
    }

    #[test]
    fn mpa_cut_off_and_negatives() {
        let o = mpa(&Inputs::new().flag("p_anca_mpo", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(6.0), "mpa_2022.met"));
        let o = mpa(&Inputs::new().flag("p_anca_mpo", true).flag("nasal", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(3.0), "mpa_2022.not_met"));
        let o = mpa(&Inputs::new().flag("pauci_immune_gn", true).flag("lung_fibrosis", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(6.0), "mpa_2022.met"));
    }
}
