//! IPSS for primary myelofibrosis (at diagnosis).
//!
//! Source: the Hematology book (Chapter 4, Lecture 9, p. 151): 1 point each
//! for age over 65, WBC over 25 × 10⁹/L, haemoglobin under 100 g/L
//! (10 g/dL), peripheral blasts 1% or more, and constitutional symptoms.
//! Low 0, intermediate-1 1, intermediate-2 2, high 3 or more; median
//! survival 135, 95, 48 and 27 months. The book prints the blast cut-off
//! as "< 1 / > 1"; the original IPSS (Cervantes 2009) counts ≥ 1%.

use super::{Area, CalcError, CalcSpec, Inputs, Outcome, Part, flag, num};

pub const SPEC: CalcSpec = CalcSpec {
    id: "ipss_mf",
    name: "IPSS (primary myelofibrosis)",
    area: Area::Blood,
    fields: &[
        num("age_years", "Age", "years", 18.0, 110.0, 1.0),
        num("wbc_10e9_l", "WBC", "× 10⁹/L", 0.1, 500.0, 0.1),
        num("hb_g_dl", "Haemoglobin", "g/dL", 2.0, 25.0, 0.1),
        num("blasts_pct", "Peripheral blasts", "%", 0.0, 100.0, 0.1),
        flag("constitutional", "Constitutional symptoms"),
    ],
    source: "Cervantes 2009; book Hematology Lecture 9, p. 151",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let parts = vec![
        Part::met("Age > 65", i.get_num("age_years")? > 65.0),
        Part::met("WBC > 25 × 10⁹/L", i.get_num("wbc_10e9_l")? > 25.0),
        Part::met("Hb < 10 g/dL", i.get_num("hb_g_dl")? < 10.0),
        Part::met("Blasts ≥ 1%", i.get_num("blasts_pct")? >= 1.0),
        Part::met("Constitutional symptoms", i.get_flag("constitutional")?),
    ];
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = match total as u8 {
        0 => ("ipss_mf.low", "Low (0)", "Low risk: median survival 135 months."),
        1 => ("ipss_mf.int1", "Intermediate-1 (1)", "Intermediate-1 risk: median survival 95 months."),
        2 => ("ipss_mf.int2", "Intermediate-2 (2)", "Intermediate-2 risk: median survival 48 months."),
        _ => ("ipss_mf.high", "High (3 or more)", "High risk: median survival 27 months."),
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{total}"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(151),
        parts,
        notes: vec![
            "The book prints the blast cut-off as \"< 1 / > 1\"; the original IPSS counts 1% or more."
                .to_owned(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(age: f64, wbc: f64, hb: f64, blasts: f64, symptoms: bool) -> Outcome {
        compute(
            &Inputs::new()
                .num("age_years", age)
                .num("wbc_10e9_l", wbc)
                .num("hb_g_dl", hb)
                .num("blasts_pct", blasts)
                .flag("constitutional", symptoms),
        )
        .unwrap()
    }

    #[test]
    fn risk_groups() {
        assert_eq!(run(60.0, 10.0, 12.0, 0.0, false).band, "ipss_mf.low");
        assert_eq!(run(70.0, 10.0, 12.0, 0.0, false).band, "ipss_mf.int1");
        assert_eq!(run(70.0, 30.0, 12.0, 0.0, false).band, "ipss_mf.int2");
        let o = run(70.0, 30.0, 9.0, 1.0, true);
        assert_eq!((o.score, o.band), (Some(5.0), "ipss_mf.high"));
        // Exactly 65 does not count; 1% blasts does.
        assert_eq!(run(65.0, 10.0, 12.0, 1.0, false).score, Some(1.0));
    }
}
