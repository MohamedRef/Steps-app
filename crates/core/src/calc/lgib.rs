//! Lower GI bleeding: the shock index and the Oakland score.
//!
//! Source: the book's Gastroenterology Lecture 7. Shock index = heart rate ÷
//! systolic BP; above 1 the patient is unstable (p. 227). Stable bleeds are
//! then sorted with the Oakland score, Table 1 (p. 229, Oakland 2017): age
//! < 40 0, 40–69 1, ≥ 70 2; male 1; previous LGIB admission 1; blood on DRE
//! 1; heart rate < 70 0, 70–89 1, 90–109 2, ≥ 110 3; systolic BP < 90 5,
//! 90–119 4, 120–129 3, 130–159 2, ≥ 160 0; haemoglobin (g/L) < 70 22, 70–89
//! 17, 90–109 13, 110–129 8, 130–159 4, ≥ 160 0. The table's footnote: 8 or
//! less, with no other reason to admit, is fit for discharge and outpatient
//! investigation; p. 228 words it as "OS < 8 carries 95% probability of safe
//! discharge".

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice, flag, num, show};

const HR: super::Field = num("hr_bpm", "Heart rate", "beats/min", 20.0, 250.0, 1.0);
const SBP: super::Field = num("sbp_mmhg", "Systolic blood pressure", "mmHg", 40.0, 260.0, 1.0);

pub const SHOCK_INDEX: CalcSpec = CalcSpec {
    id: "shock_index",
    name: "Shock index",
    area: Area::Gi,
    fields: &[HR, SBP],
    source: "Book Gastroenterology Lecture 7, p. 227",
    compute: shock_index,
};

fn shock_index(i: &Inputs) -> Result<Outcome, CalcError> {
    let hr = i.get_pos("hr_bpm")?;
    let sbp = i.get_pos("sbp_mmhg")?;
    let si = ((hr / sbp) * 100.0).round() / 100.0;
    let (band, label, meaning) = if hr / sbp > 1.0 {
        (
            "shock_index.unstable",
            "Above 1: unstable",
            "Unstable lower GI bleed: resuscitate; CT angiography first if bleeding goes on, \
             and think of an upper GI source.",
        )
    } else {
        (
            "shock_index.stable",
            "1 or less: stable",
            "Stable bleed: sort it into major or minor with a risk score such as the Oakland score.",
        )
    };
    Ok(Outcome {
        score: Some(si),
        display: show(si, 2),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(227),
        parts: vec![Part::new("Heart rate", hr), Part::new("÷ systolic BP", sbp)],
        notes: Vec::new(),
    })
}

const SEX: &[(&str, &str)] = &[("male", "Man"), ("female", "Woman")];

pub const OAKLAND: CalcSpec = CalcSpec {
    id: "oakland",
    name: "Oakland score (lower GI bleeding)",
    area: Area::Gi,
    fields: &[
        num("age_years", "Age", "years", 16.0, 110.0, 1.0),
        choice("sex", "Sex", SEX),
        flag("previous_lgib", "Previous admission with lower GI bleeding"),
        flag("dre_blood", "Blood on digital rectal examination"),
        HR,
        SBP,
        num("hb_g_dl", "Haemoglobin", "g/dL", 2.0, 25.0, 0.1)
            .hint("The book's table is in g/L: 7 g/dL = 70 g/L."),
    ],
    source: "Oakland 2017; book Gastroenterology Lecture 7, Table 1 (pp. 228–229)",
    compute: oakland,
};

fn age_points(age: f64) -> f64 {
    match age {
        a if a >= 70.0 => 2.0,
        a if a >= 40.0 => 1.0,
        _ => 0.0,
    }
}

fn hr_points(hr: f64) -> f64 {
    match hr {
        h if h >= 110.0 => 3.0,
        h if h >= 90.0 => 2.0,
        h if h >= 70.0 => 1.0,
        _ => 0.0,
    }
}

fn sbp_points(sbp: f64) -> f64 {
    match sbp {
        s if s < 90.0 => 5.0,
        s if s < 120.0 => 4.0,
        s if s < 130.0 => 3.0,
        s if s < 160.0 => 2.0,
        _ => 0.0,
    }
}

/// Haemoglobin in g/L, as the book's table has it.
fn hb_points(hb_g_l: f64) -> f64 {
    match hb_g_l {
        h if h < 70.0 => 22.0,
        h if h < 90.0 => 17.0,
        h if h < 110.0 => 13.0,
        h if h < 130.0 => 8.0,
        h if h < 160.0 => 4.0,
        _ => 0.0,
    }
}

fn oakland(i: &Inputs) -> Result<Outcome, CalcError> {
    let man = i.get_choice("sex", &["male", "female"])? == "male";
    let hb_g_l = (i.get_num("hb_g_dl")? * 10.0 * 10.0).round() / 10.0;
    let parts = vec![
        Part::new("Age", age_points(i.get_num("age_years")?)),
        Part::new("Male", if man { 1.0 } else { 0.0 }),
        Part::new("Previous LGIB admission", if i.get_flag("previous_lgib")? { 1.0 } else { 0.0 }),
        Part::new("Blood on DRE", if i.get_flag("dre_blood")? { 1.0 } else { 0.0 }),
        Part::new("Heart rate", hr_points(i.get_num("hr_bpm")?)),
        Part::new("Systolic BP", sbp_points(i.get_num("sbp_mmhg")?)),
        Part::new("Haemoglobin", hb_points(hb_g_l)),
    ];
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = if total <= 8.0 {
        (
            "oakland.discharge",
            "8 or less",
            "Minor bleed: with no other reason to admit, fit for discharge and outpatient \
             investigation (about 95% chance of safe discharge).",
        )
    } else {
        ("oakland.admit", "Above 8", "Major bleed: admit for inpatient care and colonoscopy.")
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{total}"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(229),
        parts,
        notes: vec!["The table's footnote says 8 or less; the text on p. 228 says below 8.".to_owned()],
    })
}

/// Next steps and differential diagnosis for each band (ACG 2023 LGIB
/// guideline, Sengupta; BSG 2019, Oakland; book pp. 227–229).
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "shock_index.unstable",
        Guide {
            next: &[
                "Resuscitate: IV fluids, and red cells to keep Hb above 7 g/dL (higher with cardiovascular disease).",
                "Exclude an upper GI source: upper endoscopy if suspicion is high (BUN/creatinine above 30, ulcer history, NSAIDs).",
                "Ongoing bleeding: CT angiography first, then catheter embolisation if it shows extravasation.",
            ],
            dd: &[
                "Brisk upper GI bleed presenting as haematochezia",
                "Diverticular bleeding",
                "Angiodysplasia",
                "Other causes of a shock index above 1: sepsis, cardiogenic shock, pain, fever",
            ],
        },
    ),
    (
        "shock_index.stable",
        Guide {
            next: &[
                "Sort the bleed with the Oakland score: 8 or less can go home.",
                "Beta-blockers or a pacemaker can hold the heart rate down: judge the whole patient.",
                "If admitted: restrictive transfusion (Hb 7 g/dL) and a non-urgent colonoscopy.",
            ],
            dd: &[
                "Haemorrhoids or anal fissure",
                "Diverticular bleeding",
                "Colitis: inflammatory, infective, ischaemic",
                "Polyp or colorectal cancer",
            ],
        },
    ),
    (
        "oakland.discharge",
        Guide {
            next: &[
                "Discharge if there is no other reason to admit.",
                "Outpatient colonoscopy or flexible sigmoidoscopy.",
                "Come back with more bleeding, dizziness or fainting.",
            ],
            dd: &[
                "Haemorrhoids",
                "Anal fissure",
                "Diverticular bleeding that has stopped",
                "Polyp or colorectal cancer",
                "Proctitis or colitis",
            ],
        },
    ),
    (
        "oakland.admit",
        Guide {
            next: &[
                "Admit. Restrictive transfusion: Hb threshold 7 g/dL (8 with cardiovascular disease).",
                "Review anticoagulants and antiplatelets.",
                "Non-urgent inpatient colonoscopy: within 24 hours does not improve rebleeding or death.",
                "CT angiography first if bleeding goes on with haemodynamic instability.",
            ],
            dd: &[
                "Diverticular bleeding (the commonest)",
                "Angiodysplasia",
                "Ischaemic colitis",
                "Inflammatory bowel disease",
                "Colorectal cancer",
                "Post-polypectomy bleeding",
                "Upper GI source",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shock_index_above_one_is_unstable() {
        let o = shock_index(&Inputs::new().num("hr_bpm", 120.0).num("sbp_mmhg", 90.0)).unwrap();
        assert_eq!((o.score, o.band, o.display.as_str()), (Some(1.33), "shock_index.unstable", "1.33"));
        let o = shock_index(&Inputs::new().num("hr_bpm", 100.0).num("sbp_mmhg", 100.0)).unwrap();
        assert_eq!(o.band, "shock_index.stable");
    }

    #[test]
    fn the_oakland_table_edges() {
        assert_eq!([39.0, 40.0, 69.0, 70.0].map(age_points), [0.0, 1.0, 1.0, 2.0]);
        assert_eq!([69.0, 70.0, 89.0, 90.0, 109.0, 110.0].map(hr_points), [0.0, 1.0, 1.0, 2.0, 2.0, 3.0]);
        assert_eq!(
            [89.0, 90.0, 119.0, 120.0, 129.0, 130.0, 159.0, 160.0].map(sbp_points),
            [5.0, 4.0, 4.0, 3.0, 3.0, 2.0, 2.0, 0.0]
        );
        assert_eq!(
            [69.0, 70.0, 89.0, 90.0, 110.0, 130.0, 160.0].map(hb_points),
            [22.0, 17.0, 17.0, 13.0, 8.0, 4.0, 0.0]
        );
    }

    #[test]
    fn a_young_woman_with_a_normal_hb_goes_home() {
        let i = Inputs::new()
            .num("age_years", 30.0)
            .choice("sex", "female")
            .num("hr_bpm", 80.0)
            .num("sbp_mmhg", 165.0)
            .num("hb_g_dl", 13.5);
        // HR 1 + Hb 4 = 5.
        let o = oakland(&i).unwrap();
        assert_eq!((o.score, o.band), (Some(5.0), "oakland.discharge"));
        // 8 is still fit for discharge by the table.
        let o = oakland(&i.num("age_years", 45.0).choice("sex", "male").flag("dre_blood", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(8.0), "oakland.discharge"));
    }

    #[test]
    fn an_older_man_with_anaemia_is_admitted() {
        let i = Inputs::new()
            .num("age_years", 75.0)
            .choice("sex", "male")
            .flag("previous_lgib", true)
            .flag("dre_blood", true)
            .num("hr_bpm", 112.0)
            .num("sbp_mmhg", 95.0)
            .num("hb_g_dl", 8.0);
        // 2 + 1 + 1 + 1 + 3 + 4 + 17 = 29.
        let o = oakland(&i).unwrap();
        assert_eq!((o.score, o.band), (Some(29.0), "oakland.admit"));
    }
}
