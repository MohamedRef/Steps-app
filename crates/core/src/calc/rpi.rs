//! Reticulocyte production index (RPI) and absolute reticulocyte count.
//!
//! Source: the Hematology book (Chapter 4, Lecture 2, p. 14): "Two methods
//! are employed to determine whether BM's response to the anaemia is
//! appropriate": the absolute reticulocyte count (reticulocytes % × RBC
//! count) and the RPI, with the book's figure splitting anaemia into
//! underproduction (RI < 2%) and increased destruction or loss (RI > 2%).
//! The book names the RPI but does not print its formula; this uses the
//! usual one: RPI = reticulocytes % × (Hct / 45) / maturation time, with a
//! maturation time of 1 day for Hct 40% or more, 1.5 for 35–39, 2 for
//! 25–34, 2.5 for 15–24 and 3 below 15.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, num, show};

pub const SPEC: CalcSpec = CalcSpec {
    id: "rpi",
    name: "Reticulocyte production index (RPI)",
    area: Area::Blood,
    fields: &[
        num("retic_pct", "Reticulocytes", "%", 0.0, 60.0, 0.1),
        num("hct_pct", "Haematocrit", "%", 5.0, 70.0, 0.1),
        num("rbc_10e12_l", "RBC count", "× 10⁶/µL", 0.5, 9.0, 0.01)
            .optional()
            .hint("Only for the absolute reticulocyte count."),
    ],
    source: "Hillman & Finch, Red Cell Manual (formula and maturation times, not printed in the book); book Hematology Lecture 2, p. 14 (RI < 2% / > 2%)",
    compute,
};

/// Days a reticulocyte stays in the blood at this haematocrit.
fn maturation(hct: f64) -> f64 {
    match hct {
        h if h >= 40.0 => 1.0,
        h if h >= 35.0 => 1.5,
        h if h >= 25.0 => 2.0,
        h if h >= 15.0 => 2.5,
        _ => 3.0,
    }
}

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let retic = i.get_num("retic_pct")?;
    if retic < 0.0 {
        return Err(CalcError::Invalid { field: "retic_pct", reason: "must be 0 or more" });
    }
    let hct = i.get_pos("hct_pct")?;
    let days = maturation(hct);
    let corrected = retic * hct / 45.0;
    let rpi = corrected / days;
    let mut notes = vec![
        "The book names the RPI and its 2% cut-off but not the formula; this is Hillman and Finch's (normal Hct 45%, maturation time by Hct)."
            .to_owned(),
    ];
    if let Some(rbc) = i.opt_num("rbc_10e12_l")? {
        // 1% of 1 × 10¹²/L is 10 × 10⁹/L.
        let arc = retic * rbc * 10.0;
        notes.push(format!("Absolute reticulocyte count: {} × 10⁹/L.", show(arc, 0)));
    }
    let (band, label, meaning) = if rpi < 2.0 {
        ("rpi.low", "Below 2", "Inadequate marrow response: underproduction anaemia.")
    } else {
        ("rpi.adequate", "2 or more", "Adequate marrow response: increased destruction or loss.")
    };
    Ok(Outcome {
        score: Some(rpi),
        display: show(rpi, 1),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(14),
        parts: vec![
            Part::new("Corrected reticulocytes (%)", (corrected * 100.0).round() / 100.0),
            Part::new("Maturation time (days)", days),
        ],
        notes,
    })
}

/// Next step and differential per band (Hillman & Finch; standard anaemia work-up).
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "rpi.low",
        Guide {
            next: &[
                "Sort the anaemia by the MCV: low, normal or high.",
                "Iron studies (ferritin, transferrin saturation), B12 and folate.",
                "Renal function, TSH and a blood film.",
                "Bone marrow biopsy if two or more cell lines are low or the film shows blasts.",
            ],
            dd: &[
                "Microcytic: iron deficiency, thalassaemia trait, anaemia of chronic disease, sideroblastic anaemia.",
                "Normocytic: anaemia of CKD (low erythropoietin), chronic disease, aplastic anaemia, marrow infiltration, early iron deficiency.",
                "Macrocytic: B12 or folate deficiency, MDS, liver disease, alcohol, hypothyroidism, drugs (methotrexate, hydroxyurea).",
            ],
        },
    ),
    (
        "rpi.adequate",
        Guide {
            next: &[
                "LDH, indirect bilirubin, haptoglobin and a blood film.",
                "Direct Coombs test if it looks like haemolysis.",
                "No haemolysis: look for bleeding (GI tract, menses).",
            ],
            dd: &[
                "Immune haemolysis: autoimmune (warm or cold), transfusion reaction, drugs.",
                "Inherited: hereditary spherocytosis, G6PD deficiency, sickle cell disease, thalassaemia.",
                "MAHA: TTP, HUS, DIC; also malaria and PNH.",
                "Recent acute blood loss.",
                "Recovery after iron, B12 or folate replacement.",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn run(retic: f64, hct: f64) -> Outcome {
        compute(&Inputs::new().num("retic_pct", retic).num("hct_pct", hct)).unwrap()
    }

    #[test]
    fn production_index() {
        // 6% reticulocytes at Hct 25%: 6 × 25/45 = 3.33, ÷ 2 = 1.7.
        let o = run(6.0, 25.0);
        assert_eq!((o.display.as_str(), o.band), ("1.7", "rpi.low"));
        // 9% at Hct 30%: 9 × 30/45 = 6, ÷ 2 = 3.
        assert_eq!(run(9.0, 30.0).band, "rpi.adequate");
        // Maturation time steps at 40, 35, 25 and 15.
        assert_eq!(maturation(40.0), 1.0);
        assert_eq!(maturation(39.9), 1.5);
        assert_eq!(maturation(24.0), 2.5);
        assert_eq!(maturation(10.0), 3.0);
    }

    #[test]
    fn absolute_count_when_rbc_given() {
        let o = compute(&Inputs::new().num("retic_pct", 2.0).num("hct_pct", 30.0).num("rbc_10e12_l", 3.5))
            .unwrap();
        assert!(o.notes.iter().any(|n| n.contains("70 × 10⁹/L")), "{:?}", o.notes);
    }
}
