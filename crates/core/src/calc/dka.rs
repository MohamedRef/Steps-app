//! DKA: whether the criteria are met, and how severe.
//!
//! Source: the Diabetology book, Lecture 15, p. 188 (ADA 2024 consensus
//! table). Diagnosis: glucose ≥ 200 mg/dL or known diabetes; β-hydroxybutyrate
//! ≥ 3.0 mmol/L or urine ketones 2+; pH < 7.3 and/or bicarbonate < 18. Severity
//! is the worst of: pH > 7.25 to < 7.30 or HCO3 15–18 mild, pH 7.0–7.25 or
//! HCO3 10 to < 15 moderate, pH < 7.0 or HCO3 < 10 severe; β-hydroxybutyrate
//! above 6.0 mmol/L severe; mental status alert (mild), alert or drowsy
//! (moderate), stupor or coma (severe). Glucose below 200 with ketosis and
//! acidosis is euglycaemic DKA (SGLT2 inhibitors, pregnancy, fasting).

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice, flag, num, show};

const MENTAL: &[(&str, &str)] = &[("alert", "Alert"), ("drowsy", "Drowsy"), ("stupor", "Stupor or coma")];

pub const SPEC: CalcSpec = CalcSpec {
    id: "dka_severity",
    name: "DKA severity",
    area: Area::Diabetes,
    fields: &[
        num("ph", "Venous or arterial pH", "", 6.5, 7.8, 0.01),
        num("hco3_mmol_l", "Bicarbonate", "mmol/L", 0.0, 45.0, 0.1),
        num("bhb_mmol_l", "β-hydroxybutyrate", "mmol/L", 0.0, 20.0, 0.1)
            .optional()
            .hint("Leave empty if you have only a urine strip and tick the box below."),
        flag("urine_ketones", "Urine ketones 2+ or more"),
        num("glucose_mg_dl", "Glucose", "mg/dL", 10.0, 3000.0, 1.0).optional(),
        flag("known_diabetes", "Known diabetes"),
        choice("mental", "Mental status", MENTAL),
    ],
    source: "Book Diabetology Lecture 15, Table 2 (p. 188), from the ADA 2024 consensus",
    compute,
};

/// 0 none, 1 mild, 2 moderate, 3 severe.
fn ph_grade(ph: f64) -> u8 {
    match ph {
        p if p < 7.0 => 3,
        p if p <= 7.25 => 2,
        p if p < 7.3 => 1,
        _ => 0,
    }
}

fn hco3_grade(hco3: f64) -> u8 {
    match hco3 {
        h if h < 10.0 => 3,
        h if h < 15.0 => 2,
        h if h < 18.0 => 1,
        _ => 0,
    }
}

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let ph = i.get_num("ph")?;
    let hco3 = i.get_num("hco3_mmol_l")?;
    let bhb = i.opt_num("bhb_mmol_l")?;
    let glucose = i.opt_num("glucose_mg_dl")?;
    let mental =
        i.get_option("mental", &[("alert", &[]), ("drowsy", &[]), ("stupor", &["coma", "stupor_coma"])])?;
    let ketosis = bhb.is_some_and(|b| b >= 3.0) || i.get_flag("urine_ketones")?;
    let diabetes = glucose.is_some_and(|g| g >= 200.0) || i.get_flag("known_diabetes")?;
    let acidosis = ph < 7.3 || hco3 < 18.0;
    let mut parts = vec![
        Part::met("Glucose ≥ 200 mg/dL or known diabetes", diabetes),
        Part::met("Ketosis: β-OHB ≥ 3.0 mmol/L or urine ketones 2+", ketosis),
        Part::met("Acidosis: pH < 7.3 and/or HCO3 < 18", acidosis),
    ];
    let mut notes = Vec::new();
    if !(ketosis && acidosis && diabetes) {
        if ketosis && acidosis {
            notes.push("Without glucose ≥ 200 or known diabetes: think of another ketoacidosis.".to_owned());
        }
        return Ok(Outcome {
            score: None,
            display: "Not DKA".to_owned(),
            band: "dka_severity.not_met",
            label: "DKA criteria not met",
            meaning: "The DKA triad (diabetes or hyperglycaemia, ketosis, acidosis) is not complete."
                .to_owned(),
            page: Some(188),
            parts,
            notes,
        });
    }
    let grades = [
        ("pH", ph_grade(ph)),
        ("Bicarbonate", hco3_grade(hco3)),
        ("β-hydroxybutyrate > 6.0", if bhb.is_some_and(|b| b > 6.0) { 3 } else { 1 }),
        (
            "Mental status",
            match mental {
                "stupor" => 3,
                "drowsy" => 2,
                _ => 1,
            },
        ),
    ];
    let worst = grades.iter().map(|(_, g)| *g).max().unwrap_or(1).max(1);
    parts.extend(grades.iter().map(|(l, g)| Part::new(*l, f64::from(*g))));
    if glucose.is_some_and(|g| g < 200.0) {
        notes.push(format!(
            "Glucose {} mg/dL: euglycaemic DKA (SGLT2 inhibitors, pregnancy, fasting, insulin on the way in).",
            show(glucose.unwrap_or_default(), 0)
        ));
    }
    let (band, label, meaning) = match worst {
        3 => ("dka_severity.severe", "Severe DKA", "Severe DKA: intensive care unit."),
        2 => ("dka_severity.moderate", "Moderate DKA", "Moderate DKA: step-down or intermediate care unit."),
        _ => ("dka_severity.mild", "Mild DKA", "Mild DKA: regular nursing unit or observation."),
    };
    Ok(Outcome {
        score: Some(f64::from(worst)),
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(188),
        parts,
        notes,
    })
}

/// Next steps and differential diagnosis for each band (ADA/EASD/JBDS/AACE/
/// DTS 2024 consensus on hyperglycaemic crises, Umpierrez 2024).
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "dka_severity.not_met",
        Guide {
            next: &["Complete the triad: β-hydroxybutyrate and a venous blood gas with bicarbonate."],
            dd: &[
                "HHS: glucose 600 or more, effective osmolality above 300, little ketosis",
                "Starvation ketosis (bicarbonate rarely below 18)",
                "Alcoholic ketoacidosis (glucose often normal or low)",
                "Lactic acidosis",
                "Toxic alcohols or salicylate",
                "Uraemic acidosis",
            ],
        },
    ),
    (
        "dka_severity.mild",
        Guide {
            next: &[
                "IV fluids: 0.9% saline or a balanced crystalloid, about 1 L in the first hour.",
                "Potassium first: no insulin while K is below 3.5; add 20–30 mEq/L of K while it is 3.5–5.",
                "Dextrose once glucose is below 250 mg/dL; keep insulin until ketosis clears.",
                "Resolved when β-hydroxybutyrate is below 0.6 mmol/L and pH is 7.3 or more (or HCO3 18 or more).",
                "Insulin: subcutaneous rapid-acting analogue is acceptable in mild, uncomplicated DKA; otherwise IV 0.1 U/kg/h.",
            ],
            dd: &[
                "Triggers: infection, missed insulin, new type 1 diabetes, MI, stroke, pancreatitis",
                "Drugs: SGLT2 inhibitors (euglycaemic DKA), steroids, antipsychotics",
                "Mixed DKA and HHS (osmolality above 300)",
            ],
        },
    ),
    (
        "dka_severity.moderate",
        Guide {
            next: &[
                "IV fluids: 0.9% saline or a balanced crystalloid, about 1 L in the first hour.",
                "Potassium first: no insulin while K is below 3.5; add 20–30 mEq/L of K while it is 3.5–5.",
                "Dextrose once glucose is below 250 mg/dL; keep insulin until ketosis clears.",
                "Resolved when β-hydroxybutyrate is below 0.6 mmol/L and pH is 7.3 or more (or HCO3 18 or more).",
                "IV insulin 0.1 U/kg/h; glucose every hour, electrolytes every 2–4 hours.",
            ],
            dd: &[
                "Triggers: infection, missed insulin, new type 1 diabetes, MI, stroke, pancreatitis",
                "Drugs: SGLT2 inhibitors (euglycaemic DKA), steroids, antipsychotics",
                "Mixed DKA and HHS (osmolality above 300)",
            ],
        },
    ),
    (
        "dka_severity.severe",
        Guide {
            next: &[
                "Intensive care.",
                "IV fluids: 0.9% saline or a balanced crystalloid, about 1 L in the first hour.",
                "Potassium first: no insulin while K is below 3.5; add 20–30 mEq/L of K while it is 3.5–5.",
                "Dextrose once glucose is below 250 mg/dL; keep insulin until ketosis clears.",
                "Resolved when β-hydroxybutyrate is below 0.6 mmol/L and pH is 7.3 or more (or HCO3 18 or more).",
                "IV insulin 0.1 U/kg/h. Bicarbonate only if pH is below 7.0.",
                "Watch for cerebral oedema (young patients), hypokalaemia and hypoglycaemia.",
            ],
            dd: &[
                "Triggers: infection, missed insulin, new type 1 diabetes, MI, stroke, pancreatitis",
                "Drugs: SGLT2 inhibitors (euglycaemic DKA), steroids, antipsychotics",
                "Mixed DKA and HHS (osmolality above 300)",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn dka(ph: f64, hco3: f64) -> Inputs {
        Inputs::new()
            .num("ph", ph)
            .num("hco3_mmol_l", hco3)
            .num("bhb_mmol_l", 4.0)
            .num("glucose_mg_dl", 450.0)
            .choice("mental", "alert")
    }

    #[test]
    fn the_tables_columns() {
        assert_eq!(compute(&dka(7.28, 16.0)).unwrap().band, "dka_severity.mild");
        assert_eq!(compute(&dka(7.25, 16.0)).unwrap().band, "dka_severity.moderate");
        assert_eq!(compute(&dka(7.28, 12.0)).unwrap().band, "dka_severity.moderate");
        assert_eq!(compute(&dka(7.0, 10.0)).unwrap().band, "dka_severity.moderate");
        assert_eq!(compute(&dka(6.95, 12.0)).unwrap().band, "dka_severity.severe");
        assert_eq!(compute(&dka(7.2, 8.0)).unwrap().band, "dka_severity.severe");
    }

    #[test]
    fn ketones_and_mental_status_raise_the_grade() {
        assert_eq!(compute(&dka(7.28, 16.0).num("bhb_mmol_l", 6.5)).unwrap().band, "dka_severity.severe");
        assert_eq!(
            compute(&dka(7.28, 16.0).choice("mental", "drowsy")).unwrap().band,
            "dka_severity.moderate"
        );
        assert_eq!(compute(&dka(7.28, 16.0).choice("mental", "coma")).unwrap().band, "dka_severity.severe");
    }

    #[test]
    fn the_triad_must_be_complete() {
        assert_eq!(compute(&dka(7.35, 22.0)).unwrap().band, "dka_severity.not_met");
        let no_ketones = dka(7.2, 12.0).num("bhb_mmol_l", 1.0);
        assert_eq!(compute(&no_ketones).unwrap().band, "dka_severity.not_met");
        let strip = no_ketones.flag("urine_ketones", true);
        assert_eq!(compute(&strip).unwrap().band, "dka_severity.moderate");
    }

    #[test]
    fn euglycaemic_dka_is_noted() {
        let i = dka(7.2, 12.0).num("glucose_mg_dl", 160.0).flag("known_diabetes", true);
        let o = compute(&i).unwrap();
        assert_eq!(o.band, "dka_severity.moderate");
        assert!(o.notes[0].contains("euglycaemic"));
    }
}
