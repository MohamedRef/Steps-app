//! Glasgow-Blatchford score for upper GI bleeding, before endoscopy.
//!
//! Points as the book's Table 2 lists them (Gastroenterology, Lecture 11,
//! p. 309): BUN 18.2 to < 22.4 mg/dL 2, 22.4 to < 28 3, 28 to < 70 4, 70 or
//! more 6; haemoglobin 12 to < 13 g/dL (men) or 10 to < 12 (women) 1, 10 to
//! < 12 (men) 3, under 10 6; systolic BP 100–109 1, 90–99 2, under 90 3;
//! heart rate 100 or more 1; melena 1; syncope 2; hepatic disease 2;
//! cardiac failure 2. The book reads a score of 0–1 as very low risk, fit
//! for outpatient care and endoscopy.

use super::{Area, CalcError, CalcSpec, Inputs, Outcome, Part, choice, flag, num};

const SEX: &[(&str, &str)] = &[("male", "Man"), ("female", "Woman")];

pub const SPEC: CalcSpec = CalcSpec {
    id: "glasgow_blatchford",
    name: "Glasgow-Blatchford score",
    area: Area::Gi,
    fields: &[
        num("bun_mg_dl", "BUN", "mg/dL", 1.0, 300.0, 0.1).hint("Urea ÷ 2.14."),
        choice("sex", "Sex", SEX),
        num("hb_g_dl", "Haemoglobin", "g/dL", 2.0, 25.0, 0.1),
        num("sbp_mmhg", "Systolic blood pressure", "mmHg", 40.0, 260.0, 1.0),
        num("hr_bpm", "Heart rate", "beats/min", 20.0, 250.0, 1.0),
        flag("melena", "Melena"),
        flag("syncope", "Syncope"),
        flag("hepatic_disease", "Hepatic disease"),
        flag("cardiac_failure", "Cardiac failure"),
    ],
    source: "Blatchford 2000; book Gastroenterology Lecture 11, p. 309",
    compute,
};

fn bun_points(bun: f64) -> f64 {
    match bun {
        b if b >= 70.0 => 6.0,
        b if b >= 28.0 => 4.0,
        b if b >= 22.4 => 3.0,
        b if b >= 18.2 => 2.0,
        _ => 0.0,
    }
}

fn hb_points(hb: f64, man: bool) -> f64 {
    match (hb, man) {
        (h, _) if h < 10.0 => 6.0,
        (h, true) if h < 12.0 => 3.0,
        (h, true) if h < 13.0 => 1.0,
        (h, false) if h < 12.0 => 1.0,
        _ => 0.0,
    }
}

fn sbp_points(sbp: f64) -> f64 {
    match sbp {
        s if s < 90.0 => 3.0,
        s if s < 100.0 => 2.0,
        s if s < 110.0 => 1.0,
        _ => 0.0,
    }
}

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let man = i.get_choice("sex", &["male", "female"])? == "male";
    let flag = |key: &'static str, label: &str, points: f64| -> Result<Part, CalcError> {
        Ok(Part::new(label, if i.get_flag(key)? { points } else { 0.0 }))
    };
    let parts = vec![
        Part::new("BUN", bun_points(i.get_num("bun_mg_dl")?)),
        Part::new("Haemoglobin", hb_points(i.get_num("hb_g_dl")?, man)),
        Part::new("Systolic BP", sbp_points(i.get_num("sbp_mmhg")?)),
        Part::new("Heart rate ≥ 100", if i.get_num("hr_bpm")? >= 100.0 { 1.0 } else { 0.0 }),
        flag("melena", "Melena", 1.0)?,
        flag("syncope", "Syncope", 2.0)?,
        flag("hepatic_disease", "Hepatic disease", 2.0)?,
        flag("cardiac_failure", "Cardiac failure", 2.0)?,
    ];
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = if total <= 1.0 {
        (
            "glasgow_blatchford.very_low",
            "0–1",
            "Very low risk of rebleeding, 30-day death or needing hospital treatment: outpatient care and endoscopy.",
        )
    } else {
        (
            "glasgow_blatchford.not_low",
            "2 or more",
            "Above the very-low-risk cut-off: assess and manage in hospital.",
        )
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{total}"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(309),
        parts,
        notes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calm() -> Inputs {
        Inputs::new()
            .num("bun_mg_dl", 15.0)
            .choice("sex", "male")
            .num("hb_g_dl", 14.0)
            .num("sbp_mmhg", 120.0)
            .num("hr_bpm", 80.0)
    }

    #[test]
    fn a_calm_patient_scores_zero() {
        let o = compute(&calm()).unwrap();
        assert_eq!(o.score, Some(0.0));
        assert_eq!(o.band, "glasgow_blatchford.very_low");
        let o = compute(&calm().flag("melena", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(1.0), "glasgow_blatchford.very_low"));
    }

    #[test]
    fn the_table_edges() {
        assert_eq!([18.1, 18.2, 22.4, 28.0, 70.0].map(bun_points), [0.0, 2.0, 3.0, 4.0, 6.0]);
        assert_eq!(hb_points(12.5, true), 1.0);
        assert_eq!(hb_points(12.5, false), 0.0);
        assert_eq!(hb_points(11.0, true), 3.0);
        assert_eq!(hb_points(11.0, false), 1.0);
        assert_eq!(hb_points(9.9, false), 6.0);
        assert_eq!([110.0, 109.0, 99.0, 89.0].map(sbp_points), [0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn everything_at_once() {
        let i = Inputs::new()
            .num("bun_mg_dl", 80.0)
            .choice("sex", "female")
            .num("hb_g_dl", 8.0)
            .num("sbp_mmhg", 85.0)
            .num("hr_bpm", 120.0)
            .flag("melena", true)
            .flag("syncope", true)
            .flag("hepatic_disease", true)
            .flag("cardiac_failure", true);
        let o = compute(&i).unwrap();
        assert_eq!(o.score, Some(23.0));
        assert_eq!(o.band, "glasgow_blatchford.not_low");
    }
}
