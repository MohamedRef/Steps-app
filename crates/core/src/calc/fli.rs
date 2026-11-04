//! Fatty liver index (FLI).
//!
//! The book lists the FLI for steatosis (Lecture 6, p. 101: "calculated from
//! serum TG, BMI, waist circumference & GGT") without the formula. Formula and
//! cut-offs from Bedogni et al., BMC Gastroenterology 2006:
//! y = 0.953 ln(TG mg/dL) + 0.139 BMI + 0.718 ln(GGT U/L) + 0.053 waist cm
//! − 15.745; FLI = 100 eʸ / (1 + eʸ). < 30 rules out, ≥ 60 rules in fatty
//! liver.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, num, show};

pub const SPEC: CalcSpec = CalcSpec {
    id: "fli",
    name: "Fatty liver index (FLI)",
    area: Area::Liver,
    fields: &[
        num("tg_mg_dl", "Triglycerides", "mg/dL", 10.0, 5000.0, 1.0),
        num("bmi", "BMI", "kg/m²", 10.0, 80.0, 0.1),
        num("waist_cm", "Waist circumference", "cm", 40.0, 250.0, 1.0),
        num("ggt_u_l", "GGT", "U/L", 1.0, 5000.0, 1.0),
    ],
    source: "Bedogni 2006 (formula and cut-offs; the book names the FLI only, Lecture 6, p. 101)",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let tg = i.get_pos("tg_mg_dl")?;
    let bmi = i.get_pos("bmi")?;
    let waist = i.get_pos("waist_cm")?;
    let ggt = i.get_pos("ggt_u_l")?;
    let parts = vec![
        Part::new("0.953 × ln TG", 0.953 * tg.ln()),
        Part::new("0.139 × BMI", 0.139 * bmi),
        Part::new("0.718 × ln GGT", 0.718 * ggt.ln()),
        Part::new("0.053 × waist", 0.053 * waist),
    ];
    let y: f64 = parts.iter().map(|p| p.points).sum::<f64>() - 15.745;
    let score = 100.0 / (1.0 + (-y).exp());
    let (band, label, meaning) = if score < 30.0 {
        ("fli.low", "Below 30", "Fatty liver ruled out.")
    } else if score < 60.0 {
        ("fli.mid", "30–59", "Indeterminate: confirm with ultrasound.")
    } else {
        ("fli.high", "60 or above", "Fatty liver likely.")
    };
    Ok(Outcome {
        score: Some(score),
        display: show(score, 0),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(101),
        parts,
        notes: Vec::new(),
    })
}

const DD: &str = "Causes of steatosis: MASLD (metabolic), alcohol, HCV genotype 3, drugs (amiodarone, methotrexate, \
                  steroids, tamoxifen), Wilson disease, hypothyroidism, starvation or parenteral nutrition.";

pub const GUIDES: &[(&str, Guide)] = &[
    ("fli.low", Guide { next: &["Fatty liver unlikely; look elsewhere if enzymes are raised."], dd: &[] }),
    ("fli.mid", Guide { next: &["Ultrasound to confirm or exclude steatosis."], dd: &[DD] }),
    (
        "fli.high",
        Guide {
            next: &[
                "Confirm with ultrasound, then FIB-4 for fibrosis (elastography if 1.3 or above).",
                "Weight loss and treat diabetes, lipids and blood pressure.",
            ],
            dd: &[DD],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn run(tg: f64, bmi: f64, waist: f64, ggt: f64) -> Outcome {
        compute(&Inputs::new().num("tg_mg_dl", tg).num("bmi", bmi).num("waist_cm", waist).num("ggt_u_l", ggt))
            .unwrap()
    }

    #[test]
    fn worked_examples() {
        // y = 0.953 ln 150 + 0.139 × 30 + 0.718 ln 40 + 0.053 × 100 − 15.745 = 1.149 → 75.9.
        let o = run(150.0, 30.0, 100.0, 40.0);
        assert!((o.score.unwrap() - 75.9).abs() < 0.1, "{:?}", o.score);
        assert_eq!(o.band, "fli.high");
        // A lean adult: TG 80, BMI 22, waist 78, GGT 18 → about 4.
        assert_eq!(run(80.0, 22.0, 78.0, 18.0).band, "fli.low");
        // TG 130, BMI 27, waist 92, GGT 30 → about 44.
        assert_eq!(run(130.0, 27.0, 92.0, 30.0).band, "fli.mid");
    }
}
