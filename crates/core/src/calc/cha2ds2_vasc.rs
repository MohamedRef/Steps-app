//! CHA₂DS₂-VASc score for atrial fibrillation.
//!
//! The book uses it to grade thrombotic risk when anticoagulants are held for
//! endoscopy (Lecture 4, table p. 87, ASGE/ACG 2022): AF with a score of 1–4
//! is low risk, ≥ 7 high risk. The book does not print the score itself;
//! points from Lip et al., Chest 2010, and the stroke-prevention lines from
//! the ESC 2020 AF guideline (the ESC 2024 guideline drops the sex point:
//! CHA₂DS₂-VA).

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice, flag, num};

const SEX: &[(&str, &str)] = &[("male", "Male"), ("female", "Female")];

pub const SPEC: CalcSpec = CalcSpec {
    id: "cha2ds2_vasc",
    name: "CHA₂DS₂-VASc",
    area: Area::Liver,
    fields: &[
        num("age_years", "Age", "years", 18.0, 110.0, 1.0),
        choice("sex", "Sex", SEX),
        flag("heart_failure", "Heart failure"),
        flag("hypertension", "Hypertension"),
        flag("diabetes", "Diabetes"),
        flag("stroke_tia", "Previous stroke, TIA or thromboembolism"),
        flag("vascular", "Vascular disease (MI, PAD, aortic plaque)"),
    ],
    source: "Lip 2010, ESC 2020; book Lecture 4, p. 87 (thrombotic risk for endoscopy, ASGE/ACG 2022)",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let age = i.get_num("age_years")?;
    let female = i.get_choice("sex", &["male", "female"])? == "female";
    let pts = |b: bool, p: f64| if b { p } else { 0.0 };
    let age_pts = if age >= 75.0 {
        2.0
    } else if age >= 65.0 {
        1.0
    } else {
        0.0
    };
    let parts = vec![
        Part::new("Heart failure", pts(i.get_flag("heart_failure")?, 1.0)),
        Part::new("Hypertension", pts(i.get_flag("hypertension")?, 1.0)),
        Part::new(if age >= 75.0 { "Age ≥ 75" } else { "Age 65–74" }, age_pts),
        Part::new("Diabetes", pts(i.get_flag("diabetes")?, 1.0)),
        Part::new("Stroke, TIA or thromboembolism", pts(i.get_flag("stroke_tia")?, 2.0)),
        Part::new("Vascular disease", pts(i.get_flag("vascular")?, 1.0)),
        Part::new("Female sex", pts(female, 1.0)),
    ];
    let score: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = if score <= 4.0 {
        (
            "cha2ds2_vasc.low",
            "Low thrombotic risk for endoscopy (≤ 4)",
            "AF with CHA₂DS₂-VASc 1–4 is low thrombotic risk when anticoagulation is held for endoscopy \
             (ASGE, book p. 87): no bridging.",
        )
    } else if score <= 6.0 {
        (
            "cha2ds2_vasc.mid",
            "Between the book's rows (5–6)",
            "The book's table lists 1–4 as low and ≥ 7 as high thrombotic risk; 5–6 is decided case by case.",
        )
    } else {
        (
            "cha2ds2_vasc.high",
            "High thrombotic risk for endoscopy (≥ 7)",
            "AF with CHA₂DS₂-VASc ≥ 7 is high thrombotic risk when anticoagulation is held for endoscopy \
             (ASGE/ACG 2022, book p. 87): consider heparin bridging.",
        )
    };
    // ESC 2020: anticoagulate from 2 (men) or 3 (women); consider at 1 or 2.
    let base = score - pts(female, 1.0);
    let stroke = if base >= 2.0 {
        "Stroke prevention (ESC 2020): oral anticoagulation recommended."
    } else if base >= 1.0 {
        "Stroke prevention (ESC 2020): consider oral anticoagulation."
    } else {
        "Stroke prevention (ESC 2020): no anticoagulation needed."
    };
    Ok(Outcome {
        score: Some(score),
        display: format!("{score}"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(87),
        parts,
        notes: vec![stroke.to_owned()],
    })
}

pub const GUIDES: &[(&str, Guide)] = &[
    (
        "cha2ds2_vasc.low",
        Guide {
            next: &[
                "High-risk endoscopy: stop warfarin 5 days before (INR < 1.5 on the day), no heparin bridging; \
                 restart the same evening (p. 89).",
                "On a DOAC: last dose 3 days before (5 days for dabigatran with CrCl 30–50); restart 2–3 days \
                 after (p. 89).",
            ],
            dd: &[],
        },
    ),
    (
        "cha2ds2_vasc.mid",
        Guide { next: &["Weigh bleeding against clot risk case by case; involve cardiology."], dd: &[] },
    ),
    (
        "cha2ds2_vasc.high",
        Guide {
            next: &[
                "High-risk endoscopy: stop warfarin 5 days before and start LMWH 2 days after stopping; omit LMWH \
                 on the day, restart warfarin that evening and LMWH until the INR is adequate (p. 89).",
                "On a DOAC: last dose 3 days before, no bridging; restart 2–3 days after (p. 89).",
            ],
            dd: &[],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn base(age: f64, sex: &str) -> Inputs {
        Inputs::new().num("age_years", age).choice("sex", sex)
    }

    #[test]
    fn points() {
        let o = compute(&base(60.0, "male")).unwrap();
        assert_eq!((o.score, o.band), (Some(0.0), "cha2ds2_vasc.low"));
        assert!(o.notes[0].contains("no anticoagulation"));
        // A 70-year-old woman with hypertension: 1 + 1 + 1 = 3; sex alone
        // does not count towards anticoagulation (2 without it).
        let o = compute(&base(70.0, "female").flag("hypertension", true)).unwrap();
        assert_eq!(o.score, Some(3.0));
        assert!(o.notes[0].contains("recommended"));
        assert!(
            compute(&base(66.0, "female")).unwrap().notes[0].contains("consider"),
            "female 65–74 alone = 2, consider"
        );
        // 80, woman, HF, HTN, DM, stroke, vascular: 2 + 1 + 1 + 1 + 1 + 2 + 1 = 9.
        let o = compute(
            &base(80.0, "female")
                .flag("heart_failure", true)
                .flag("hypertension", true)
                .flag("diabetes", true)
                .flag("stroke_tia", true)
                .flag("vascular", true),
        )
        .unwrap();
        assert_eq!((o.score, o.band), (Some(9.0), "cha2ds2_vasc.high"));
        let o = compute(&base(76.0, "male").flag("stroke_tia", true).flag("diabetes", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(5.0), "cha2ds2_vasc.mid"));
    }
}
