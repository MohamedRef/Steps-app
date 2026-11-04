//! Diabetic emergencies: corrected sodium and serum osmolality in HHS.
//!
//! Source: the Diabetology book (Chapter 6, Lecture 15, p. 189). Corrected
//! Na = measured Na + 0.016 × (glucose mg/dL − 100), i.e. 1.6 mEq/L per
//! 100 mg/dL above 100 (glucose 500, Na 132 → 138.4). HHS criteria:
//! effective osmolality 2 × Na + glucose (mmol/L) above 300 mOsm/kg, or
//! total osmolality 2 × Na + glucose + urea (mmol/L) above 320. HHS has
//! resolved once osmolality is below 300 (p. 192).

use super::{Area, CalcError, CalcSpec, Inputs, Outcome, Part, num, show};

const NA: super::Field = num("na_meq_l", "Sodium", "mEq/L", 90.0, 200.0, 1.0);
const GLUCOSE: super::Field = num("glucose_mg_dl", "Glucose", "mg/dL", 10.0, 3000.0, 1.0);

pub const CORRECTED_NA: CalcSpec = CalcSpec {
    id: "corrected_sodium",
    name: "Corrected sodium (hyperglycaemia)",
    area: Area::Diabetes,
    fields: &[NA, GLUCOSE],
    source: "Book Diabetology Lecture 15, p. 189",
    compute: corrected_sodium,
};

fn corrected_sodium(i: &Inputs) -> Result<Outcome, CalcError> {
    let na = i.get_num("na_meq_l")?;
    let glucose = i.get_num("glucose_mg_dl")?;
    let add = 0.016 * (glucose - 100.0).max(0.0);
    let value = ((na + add) * 10.0).round() / 10.0;
    Ok(Outcome {
        score: Some(value),
        display: format!("{} mEq/L", show(value, 1)),
        band: "corrected_sodium.na",
        label: "Corrected sodium",
        meaning: "The sodium once glucose is back to 100 mg/dL. A high or normal sodium in \
                  hyperglycaemia means a profound free-water loss."
            .to_owned(),
        page: Some(189),
        parts: vec![
            Part::new("Measured Na", na),
            Part::new("+ 0.016 × (glucose − 100)", (add * 10.0).round() / 10.0),
        ],
        notes: Vec::new(),
    })
}

pub const OSMOLALITY: CalcSpec = CalcSpec {
    id: "effective_osmolality",
    name: "Serum osmolality (HHS)",
    area: Area::Diabetes,
    fields: &[
        NA,
        GLUCOSE,
        num("urea_mmol_l", "Urea", "mmol/L", 0.5, 100.0, 0.1)
            .optional()
            .hint("For the total osmolality. Urea mg/dL ÷ 6."),
    ],
    source: "Book Diabetology Lecture 15, pp. 189, 192",
    compute: osmolality,
};

fn osmolality(i: &Inputs) -> Result<Outcome, CalcError> {
    let na = i.get_num("na_meq_l")?;
    let glucose = i.get_num("glucose_mg_dl")? / 18.0;
    let effective = ((2.0 * na + glucose) * 10.0).round() / 10.0;
    let mut parts =
        vec![Part::new("2 × Na", 2.0 * na), Part::new("Glucose (mmol/L)", (glucose * 10.0).round() / 10.0)];
    let mut notes = Vec::new();
    let mut met = effective > 300.0;
    if let Some(urea) = i.opt_num("urea_mmol_l")? {
        let total = effective + urea;
        parts.push(Part::new("Urea (total only)", urea));
        notes.push(format!("Total osmolality {} mOsm/kg (HHS criterion: above 320).", show(total, 1)));
        met |= total > 320.0;
    }
    let (band, label, meaning) = if met {
        (
            "effective_osmolality.hhs",
            "Meets the HHS criterion",
            "Hyperosmolality as in HHS: effective osmolality above 300 or total above 320 mOsm/kg.",
        )
    } else {
        (
            "effective_osmolality.below",
            "Below the HHS cut-off",
            "Not hyperosmolar by the HHS criteria. Below 300 is also the resolution target.",
        )
    };
    Ok(Outcome {
        score: Some(effective),
        display: format!("{} mOsm/kg", show(effective, 1)),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(189),
        parts,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_books_sodium_example() {
        let o = corrected_sodium(&Inputs::new().num("na_meq_l", 132.0).num("glucose_mg_dl", 500.0)).unwrap();
        assert_eq!(o.score, Some(138.4));
        let o = corrected_sodium(&Inputs::new().num("na_meq_l", 140.0).num("glucose_mg_dl", 90.0)).unwrap();
        assert_eq!(o.score, Some(140.0));
    }

    #[test]
    fn hhs_cut_offs() {
        // 2 × 145 + 900/18 = 340.
        let i = Inputs::new().num("na_meq_l", 145.0).num("glucose_mg_dl", 900.0);
        let o = osmolality(&i).unwrap();
        assert_eq!((o.score, o.band), (Some(340.0), "effective_osmolality.hhs"));
        // 2 × 140 + 180/18 = 290; with urea 35 the total is 325.
        let i = Inputs::new().num("na_meq_l", 140.0).num("glucose_mg_dl", 180.0);
        assert_eq!(osmolality(&i).unwrap().band, "effective_osmolality.below");
        assert_eq!(osmolality(&i.num("urea_mmol_l", 35.0)).unwrap().band, "effective_osmolality.hhs");
    }
}
