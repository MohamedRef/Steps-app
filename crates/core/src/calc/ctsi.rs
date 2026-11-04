//! CT severity index (Balthazar) of acute pancreatitis.
//!
//! Source: the book's table, Lecture 15, p. 282: CT grade A–E scores 0–4,
//! necrosis none / < 30% / 30–50% / > 50% scores 0 / 2 / 4 / 6; CTSI is
//! grade plus necrosis (0–10), with morbidity and mortality per band. Page
//! 283: a CTSI of 3 or more is severe and needs a follow-up CECT at 7–10 days.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice};

const GRADE: &[(&str, &str)] = &[
    ("a", "A: normal pancreas"),
    ("b", "B: pancreatic enlargement"),
    ("c", "C: inflammation of the pancreas and/or peripancreatic fat"),
    ("d", "D: single peripancreatic fluid collection"),
    ("e", "E: 2 or more collections and/or retroperitoneal air"),
];

const NECROSIS: &[(&str, &str)] = &[
    ("none", "None (uniform enhancement)"),
    ("lt30", "Under 30%"),
    ("30_50", "30–50%"),
    ("gt50", "Over 50%"),
];

pub const SPEC: CalcSpec = CalcSpec {
    id: "ctsi",
    name: "CT severity index (Balthazar)",
    area: Area::Pancreas,
    fields: &[
        choice("ct_grade", "Balthazar CT grade", GRADE),
        choice("necrosis", "Pancreatic necrosis", NECROSIS),
    ],
    source: "Balthazar 1990; book Lecture 15, table p. 282 and p. 283",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let grade = i.get_option(
        "ct_grade",
        &[
            ("a", &["grade_a"]),
            ("b", &["grade_b"]),
            ("c", &["grade_c"]),
            ("d", &["grade_d"]),
            ("e", &["grade_e"]),
        ],
    )?;
    let necrosis = i.get_option(
        "necrosis",
        &[
            ("none", &["0", "no"]),
            ("lt30", &["<30", "<_30"]),
            ("30_50", &["30–50"]),
            ("gt50", &[">50", ">_50"]),
        ],
    )?;
    let g = match grade {
        "a" => 0.0,
        "b" => 1.0,
        "c" => 2.0,
        "d" => 3.0,
        _ => 4.0,
    };
    let n = match necrosis {
        "none" => 0.0,
        "lt30" => 2.0,
        "30_50" => 4.0,
        _ => 6.0,
    };
    let score = g + n;
    let (band, label, meaning) = if score <= 1.0 {
        ("ctsi.0_1", "CTSI 0–1", "Morbidity 0%, mortality 0%.")
    } else if score <= 3.0 {
        ("ctsi.2_3", "CTSI 2–3", "Morbidity 8%, mortality 3%.")
    } else if score <= 6.0 {
        ("ctsi.4_6", "CTSI 4–6", "Morbidity 35%, mortality 6%.")
    } else {
        ("ctsi.7_10", "CTSI 7–10", "Morbidity 92%, mortality 17%.")
    };
    let mut notes = Vec::new();
    if score >= 3.0 {
        notes.push("CTSI ≥ 3 is severe: repeat contrast CT 7–10 days after the first (p. 283).".to_owned());
    }
    Ok(Outcome {
        score: Some(score),
        display: format!("{score}"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(282),
        parts: vec![Part::new(format!("Grade {}", grade.to_uppercase()), g), Part::new("Necrosis", n)],
        notes,
    })
}

const LOCAL: &str = "Local complications to look for: acute peripancreatic fluid collection, pseudocyst, acute necrotic \
                     collection, walled-off necrosis, infected necrosis (gas), pseudoaneurysm, splenic or portal vein \
                     thrombosis.";

pub const GUIDES: &[(&str, Guide)] = &[
    (
        "ctsi.0_1",
        Guide {
            next: &["Supportive care: fluids, analgesia, early oral feeding."],
            dd: &["A normal CT with typical pain: mild interstitial pancreatitis, or another cause of pain."],
        },
    ),
    (
        "ctsi.2_3",
        Guide {
            next: &["Supportive care; a CTSI of 3 or more: repeat contrast CT at 7–10 days (p. 283)."],
            dd: &[LOCAL],
        },
    ),
    (
        "ctsi.4_6",
        Guide {
            next: &[
                "Treat as severe: close monitoring, enteral (nasogastric) feeding.",
                "Repeat contrast CT at 7–10 days, sooner if worsening (p. 283).",
            ],
            dd: &[LOCAL],
        },
    ),
    (
        "ctsi.7_10",
        Guide {
            next: &[
                "ICU-level care; enteral feeding.",
                "No routine antibiotics; give them for suspected infected necrosis.",
                "Infected necrosis: step-up drainage, then minimally invasive necrosectomy.",
            ],
            dd: &[LOCAL],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn run(grade: &str, necrosis: &str) -> Outcome {
        compute(&Inputs::new().choice("ct_grade", grade).choice("necrosis", necrosis)).unwrap()
    }

    #[test]
    fn grade_plus_necrosis() {
        assert_eq!(run("a", "none").band, "ctsi.0_1");
        let o = run("c", "none");
        assert_eq!((o.score, o.band), (Some(2.0), "ctsi.2_3"));
        assert!(o.notes.is_empty());
        let o = run("d", "none");
        assert_eq!(o.score, Some(3.0));
        assert_eq!(o.notes.len(), 1);
        assert_eq!(run("d", "lt30").band, "ctsi.4_6");
        let o = run("e", "gt50");
        assert_eq!((o.score, o.band), (Some(10.0), "ctsi.7_10"));
        assert_eq!(run("E", "30-50").score, Some(8.0));
    }
}
