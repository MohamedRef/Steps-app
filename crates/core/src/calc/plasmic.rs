//! PLASMIC score: how likely a thrombotic microangiopathy is TTP
//! (ADAMTS13 activity ≤ 10%).
//!
//! Source: the Nephrology book (Chapter 8, Lecture 14). p. 479: schistocytes
//! must be present; one point each for platelets < 30,000/µL, haemolysis
//! (reticulocytes above 2.5%, undetectable haptoglobin or indirect
//! bilirubin above 2 mg/dL), MCV < 90 fL, INR < 1.5, creatinine < 2.0 mg/dL, no cancer, no
//! solid-organ or stem-cell transplant; above 5 high and below 5 low
//! probability. p. 480: sensitivity 99%, specificity 57%. p. 466 places it
//! in the TMA work-up. 5 is intermediate (Bendapudi et al., 2017).

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, flag, num};

pub const SPEC: CalcSpec = CalcSpec {
    id: "plasmic",
    name: "PLASMIC score (TTP)",
    area: Area::Renal,
    fields: &[
        num("platelets_10e9_l", "Platelets", "× 10⁹/L", 0.0, 1000.0, 1.0).hint("30 × 10⁹/L = 30,000/µL."),
        flag("hemolysis", "Haemolysis")
            .hint("Reticulocytes > 2.5%, undetectable haptoglobin or indirect bilirubin > 2 mg/dL."),
        num("mcv_fl", "MCV", "fL", 40.0, 150.0, 1.0),
        num("inr", "INR", "", 0.5, 15.0, 0.1),
        num("creatinine_mg_dl", "Serum creatinine", "mg/dL", 0.1, 30.0, 0.01),
        flag("active_cancer", "Active cancer"),
        flag("transplant", "Solid-organ or stem-cell transplant"),
    ],
    source: "Bendapudi et al., Lancet Haematol 2017 (derivation and validation); book Nephrology \
             Lecture 14, pp. 466, 479–480",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let plt = i.get_num("platelets_10e9_l")?;
    let hemolysis = i.get_flag("hemolysis")?;
    let mcv = i.get_num("mcv_fl")?;
    let inr = i.get_num("inr")?;
    let cr = i.get_num("creatinine_mg_dl")?;
    let cancer = i.get_flag("active_cancer")?;
    let transplant = i.get_flag("transplant")?;
    let parts = vec![
        Part::met("Platelets < 30 × 10⁹/L", plt < 30.0),
        Part::met("Haemolysis", hemolysis),
        Part::met("No active cancer", !cancer),
        Part::met("No solid-organ or stem-cell transplant", !transplant),
        Part::met("MCV < 90 fL", mcv < 90.0),
        Part::met("INR < 1.5", inr < 1.5),
        Part::met("Creatinine < 2.0 mg/dL", cr < 2.0),
    ];
    let score: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = match score as u8 {
        6.. => ("plasmic.high", "High (6–7)", "High probability of TTP (ADAMTS13 ≤ 10%)."),
        5 => ("plasmic.intermediate", "Intermediate (5)", "Intermediate probability of TTP."),
        _ => ("plasmic.low", "Low (0–4)", "Low probability of TTP."),
    };
    Ok(Outcome {
        score: Some(score),
        display: format!("{score} / 7"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(479),
        parts,
        notes: vec![
            "Only with schistocytes on the film (p. 479).".to_owned(),
            "Sensitivity 99%, specificity 57%: best at ruling TTP out (p. 480).".to_owned(),
        ],
    })
}

const TMA_DD: &[&str] = &[
    "TTP (ADAMTS13 < 10%)",
    "STEC-HUS: bloody diarrhoea first",
    "Complement-mediated (atypical) HUS",
    "DIC: raised INR, low fibrinogen",
    "Malignant hypertension; scleroderma renal crisis",
    "Antiphospholipid syndrome and lupus (p. 466)",
    "Drugs: calcineurin inhibitors, gemcitabine, quinine",
    "Pregnancy: HELLP, pre-eclampsia",
    "Cancer; solid-organ or stem-cell transplant",
];

pub const GUIDES: &[(&str, Guide)] = &[
    (
        "plasmic.high",
        Guide {
            next: &[
                "Send ADAMTS13 activity before any plasma",
                "Start plasma exchange and steroids without waiting for the result",
                "Haematology now; caplacizumab or rituximab per the ISTH 2020 guideline",
            ],
            dd: TMA_DD,
        },
    ),
    (
        "plasmic.intermediate",
        Guide {
            next: &[
                "Send ADAMTS13; decide on plasma exchange by the clinical picture while it comes back",
                "Haematology opinion",
            ],
            dd: TMA_DD,
        },
    ),
    (
        "plasmic.low",
        Guide {
            next: &[
                "Look for another cause of the TMA (list below); ADAMTS13 is still worth sending",
                "Stool for STEC; complement studies if no other cause",
            ],
            dd: TMA_DD,
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_ttp_scores_7() {
        let i = Inputs::new()
            .num("platelets_10e9_l", 12.0)
            .flag("hemolysis", true)
            .num("mcv_fl", 85.0)
            .num("inr", 1.1)
            .num("creatinine_mg_dl", 1.2);
        let o = compute(&i).unwrap();
        assert_eq!((o.score, o.band), (Some(7.0), "plasmic.high"));
    }

    #[test]
    fn bands() {
        let base = || {
            Inputs::new()
                .num("platelets_10e9_l", 12.0)
                .flag("hemolysis", true)
                .num("mcv_fl", 85.0)
                .num("inr", 1.1)
        };
        // Creatinine 3 → 6 points.
        assert_eq!(compute(&base().num("creatinine_mg_dl", 3.0)).unwrap().band, "plasmic.high");
        // And a transplant → 5.
        let o = compute(&base().num("creatinine_mg_dl", 3.0).flag("transplant", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(5.0), "plasmic.intermediate"));
        let o = compute(
            &base().num("creatinine_mg_dl", 3.0).flag("transplant", true).flag("active_cancer", true),
        )
        .unwrap();
        assert_eq!(o.band, "plasmic.low");
    }
}
