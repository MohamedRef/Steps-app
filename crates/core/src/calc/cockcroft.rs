//! Cockcroft-Gault creatinine clearance.
//!
//! Source: the Geriatrics book (Chapter 7). Lecture 2, Renal diseases in the
//! elderly, p. 30: CrCl = (140 − age) × weight (kg) / (72 × serum
//! creatinine mg/dL), × 0.85 for women. Lecture 3, p. 42, repeats it for
//! drug dosing and warns that serum creatinine is not reliable in the
//! elderly and the result is only an estimate. The book gives no bands.

use super::{Area, CalcError, CalcSpec, Inputs, Outcome, Part, choice, num, show};

const SEX: &[(&str, &str)] = &[("male", "Man"), ("female", "Woman")];

pub const SPEC: CalcSpec = CalcSpec {
    id: "cockcroft_gault",
    name: "Cockcroft-Gault creatinine clearance",
    area: Area::Elderly,
    fields: &[
        num("age_years", "Age", "years", 18.0, 120.0, 1.0),
        num("weight_kg", "Weight", "kg", 20.0, 300.0, 0.1),
        num("creatinine_mg_dl", "Serum creatinine", "mg/dL", 0.1, 20.0, 0.01),
        choice("sex", "Sex", SEX),
    ],
    source: "Book Geriatrics Lecture 2, p. 30; Lecture 3, p. 42",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let age = i.get_num("age_years")?;
    let weight = i.get_pos("weight_kg")?;
    let cr = i.get_pos("creatinine_mg_dl")?;
    let woman = i.get_choice("sex", &["male", "female"])? == "female";
    if age >= 140.0 {
        return Err(CalcError::Invalid { field: "age_years", reason: "must be below 140" });
    }
    let base = (140.0 - age) * weight / (72.0 * cr);
    let crcl = ((if woman { base * 0.85 } else { base }) * 10.0).round() / 10.0;
    let mut parts = vec![
        Part::new("140 − age", 140.0 - age),
        Part::new("× weight (kg)", weight),
        Part::new("÷ (72 × creatinine)", 72.0 * cr),
    ];
    if woman {
        parts.push(Part::new("× 0.85 (woman)", 0.85));
    }
    Ok(Outcome {
        score: Some(crcl),
        display: format!("{} mL/min", show(crcl, 1)),
        band: "cockcroft_gault.crcl",
        label: "Estimated creatinine clearance",
        meaning: "An estimate of GFR for drug dosing. In the elderly serum creatinine is not reliable \
                  (less muscle), so read it as an estimate."
            .to_owned(),
        page: Some(30),
        parts,
        notes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(sex: &str) -> Outcome {
        compute(
            &Inputs::new()
                .num("age_years", 80.0)
                .num("weight_kg", 60.0)
                .num("creatinine_mg_dl", 1.0)
                .choice("sex", sex),
        )
        .unwrap()
    }

    #[test]
    fn the_formula() {
        // (140 − 80) × 60 / 72 = 50; × 0.85 = 42.5.
        assert_eq!(run("male").score, Some(50.0));
        assert_eq!(run("female").score, Some(42.5));
        assert!(
            compute(
                &Inputs::new()
                    .num("age_years", 80.0)
                    .num("weight_kg", 60.0)
                    .num("creatinine_mg_dl", 0.0)
                    .choice("sex", "male")
            )
            .is_err()
        );
    }
}
