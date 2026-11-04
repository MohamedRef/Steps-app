//! Acid–base and osmolality: anion gap, delta ratio, expected compensation
//! and osmolal gap.
//!
//! Source: the Nephrology book (Chapter 8, Lecture 1, Evaluation of an
//! acid–base disorder). AG = Na − (Cl + HCO3) (p. 5), normal 12 ± 4, above
//! 16 high, below 8 low (p. 14); each 1 g/dL fall of albumin below 4.0
//! lowers the AG by 2.5 (p. 9). Delta ratio = (AG − 12) / (24 − HCO3) with
//! the bands < 0.4, 0.4–0.8, 0.8–2.0 and > 2.0 (pp. 12, 14). Expected
//! compensation as the p. 14 table writes it: pCO2 = 1.5 × HCO3 + 8 (± 2)
//! in metabolic acidosis and 0.7 × HCO3 + 20 (± 5) in metabolic alkalosis;
//! HCO3 = 24 ± (pCO2 − 40)/10 × 1 (acute) or × 4 (chronic) in respiratory
//! acidosis, × 2 or × 5 in respiratory alkalosis; above or below the
//! expected value means a second disorder (p. 13). Calculated osmolality
//! = 2 × Na + glucose/18 + BUN/2.8 (mg/dL); a gap above 10 mOsm is an
//! osmolal gap (p. 20).

use super::{Area, CalcError, CalcSpec, Inputs, Outcome, Part, choice, num, show};

const NA: super::Field = num("na_meq_l", "Sodium", "mEq/L", 90.0, 200.0, 1.0);
const CL: super::Field = num("cl_meq_l", "Chloride", "mEq/L", 50.0, 160.0, 1.0);
const HCO3: super::Field = num("hco3_meq_l", "Bicarbonate (HCO3)", "mEq/L", 1.0, 60.0, 0.1);

pub const ANION_GAP: CalcSpec = CalcSpec {
    id: "anion_gap",
    name: "Anion gap",
    area: Area::Renal,
    fields: &[
        NA,
        CL,
        HCO3,
        num("albumin_g_dl", "Albumin", "g/dL", 0.5, 6.0, 0.1)
            .optional()
            .hint("Corrects the gap for a low albumin."),
    ],
    source: "Book Nephrology Lecture 1, pp. 5, 9, 14",
    compute: anion_gap,
};

fn anion_gap(i: &Inputs) -> Result<Outcome, CalcError> {
    let (na, cl, hco3) = (i.get_num("na_meq_l")?, i.get_num("cl_meq_l")?, i.get_num("hco3_meq_l")?);
    let ag = na - (cl + hco3);
    let mut parts = vec![Part::new("Na", na), Part::new("− Cl", -cl), Part::new("− HCO3", -hco3)];
    let mut notes = Vec::new();
    let value = match i.opt_num("albumin_g_dl")? {
        Some(alb) if alb < 4.0 => {
            let add = 2.5 * (4.0 - alb);
            parts.push(Part::new("Albumin correction", add));
            notes.push(format!("Measured AG {}; corrected for albumin {} g/dL.", show(ag, 1), show(alb, 1)));
            ag + add
        }
        _ => ag,
    };
    let value = (value * 10.0).round() / 10.0;
    let (band, label, meaning) = if value > 16.0 {
        ("anion_gap.high", "High (> 16)", "High anion gap: look for added unmeasured anions (Box 2).")
    } else if value < 8.0 {
        ("anion_gap.low", "Low (< 8)", "Low anion gap: the commonest cause in hospital is a low albumin.")
    } else {
        ("anion_gap.normal", "Normal (12 ± 4)", "Normal anion gap.")
    };
    if value > 16.0 {
        notes.push("Check the delta ratio next.".to_owned());
    }
    Ok(Outcome {
        score: Some(value),
        display: format!("{} mEq/L", show(value, 1)),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(14),
        parts,
        notes,
    })
}

pub const DELTA_RATIO: CalcSpec = CalcSpec {
    id: "delta_ratio",
    name: "Delta ratio (gap : gap)",
    area: Area::Renal,
    fields: &[num("anion_gap_meq_l", "Anion gap", "mEq/L", 0.0, 60.0, 0.1), HCO3],
    source: "Book Nephrology Lecture 1, pp. 12, 14",
    compute: delta_ratio,
};

fn delta_ratio(i: &Inputs) -> Result<Outcome, CalcError> {
    let ag = i.get_num("anion_gap_meq_l")?;
    let hco3 = i.get_num("hco3_meq_l")?;
    if hco3 >= 24.0 {
        return Err(CalcError::Invalid {
            field: "hco3_meq_l",
            reason: "must be below 24 (a metabolic acidosis)",
        });
    }
    let ratio = ((ag - 12.0) / (24.0 - hco3) * 100.0).round() / 100.0;
    let (band, label, meaning) = match ratio {
        r if r < 0.4 => ("delta_ratio.normal_ag", "< 0.4", "Normal anion gap metabolic acidosis."),
        r if r < 0.8 => (
            "delta_ratio.mixed",
            "0.4–0.8",
            "Mixed: normal anion gap plus high anion gap metabolic acidosis.",
        ),
        r if r <= 2.0 => ("delta_ratio.pure_high", "0.8–2.0", "Pure high anion gap metabolic acidosis."),
        _ => (
            "delta_ratio.with_alkalosis",
            "> 2.0",
            "High anion gap metabolic acidosis plus metabolic alkalosis (or a pre-existing compensated respiratory acidosis).",
        ),
    };
    Ok(Outcome {
        score: Some(ratio),
        display: show(ratio, 2),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(12),
        parts: vec![Part::new("Δ AG = AG − 12", ag - 12.0), Part::new("Δ HCO3 = 24 − HCO3", 24.0 - hco3)],
        notes: Vec::new(),
    })
}

const DISORDERS: &[(&str, &str)] = &[
    ("metabolic_acidosis", "Metabolic acidosis"),
    ("metabolic_alkalosis", "Metabolic alkalosis"),
    ("respiratory_acidosis_acute", "Respiratory acidosis, acute"),
    ("respiratory_acidosis_chronic", "Respiratory acidosis, chronic"),
    ("respiratory_alkalosis_acute", "Respiratory alkalosis, acute"),
    ("respiratory_alkalosis_chronic", "Respiratory alkalosis, chronic"),
];

pub const COMPENSATION: CalcSpec = CalcSpec {
    id: "expected_compensation",
    name: "Expected compensation",
    area: Area::Renal,
    fields: &[
        choice("disorder", "Primary disorder", DISORDERS),
        HCO3,
        num("pco2_mm_hg", "pCO2", "mmHg", 5.0, 150.0, 1.0),
    ],
    source: "Book Nephrology Lecture 1, pp. 13–14",
    compute: compensation,
};

/// The book gives no range for the respiratory rules; ± 2 mEq/L is the
/// usual one.
const RESP_RANGE: f64 = 2.0;

fn compensation(i: &Inputs) -> Result<Outcome, CalcError> {
    let ids: Vec<&str> = DISORDERS.iter().map(|(id, _)| *id).collect();
    let disorder = i.get_choice("disorder", &ids)?;
    let hco3 = i.get_num("hco3_meq_l")?;
    let pco2 = i.get_num("pco2_mm_hg")?;
    let mut notes = Vec::new();
    // (expected, range, measured, what is measured, too-high band, too-low band)
    let (expected, range, measured, what, high, low) = match disorder {
        "metabolic_acidosis" => (1.5 * hco3 + 8.0, 2.0, pco2, "pCO2", RESP_ACIDOSIS, RESP_ALKALOSIS),
        "metabolic_alkalosis" => (0.7 * hco3 + 20.0, 5.0, pco2, "pCO2", RESP_ACIDOSIS, RESP_ALKALOSIS),
        resp => {
            let acidosis = resp.starts_with("respiratory_acidosis");
            if acidosis && pco2 <= 40.0 {
                return Err(CalcError::Invalid {
                    field: "pco2_mm_hg",
                    reason: "must be above 40 in respiratory acidosis",
                });
            }
            if !acidosis && pco2 >= 40.0 {
                return Err(CalcError::Invalid {
                    field: "pco2_mm_hg",
                    reason: "must be below 40 in respiratory alkalosis",
                });
            }
            let per_10 = match resp {
                "respiratory_acidosis_acute" => 1.0,
                "respiratory_acidosis_chronic" => {
                    notes.push("The p. 13 table says 3.5 per 10 mmHg; the p. 14 summary uses 4.".to_owned());
                    4.0
                }
                "respiratory_alkalosis_acute" => 2.0,
                _ => {
                    notes.push("The p. 13 table says 4–5 per 10 mmHg; the p. 14 summary uses 5.".to_owned());
                    5.0
                }
            };
            notes.push("The book gives no range here; ± 2 mEq/L is used.".to_owned());
            (24.0 + (pco2 - 40.0) / 10.0 * per_10, RESP_RANGE, hco3, "HCO3", MET_ALKALOSIS, MET_ACIDOSIS)
        }
    };
    let expected = (expected * 10.0).round() / 10.0;
    let unit = if what == "pCO2" { "mmHg" } else { "mEq/L" };
    let (band, label, meaning) = if measured > expected + range {
        high
    } else if measured < expected - range {
        low
    } else {
        APPROPRIATE
    };
    Ok(Outcome {
        score: Some(expected),
        display: format!(
            "Expected {what} {} ({}–{} {unit})",
            show(expected, 1),
            show(expected - range, 1),
            show(expected + range, 1)
        ),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(14),
        parts: vec![
            Part::new(format!("Expected {what}"), expected),
            Part::new(format!("Measured {what}"), measured),
        ],
        notes,
    })
}

type Band = (&'static str, &'static str, &'static str);

const APPROPRIATE: Band =
    ("expected_compensation.appropriate", "Appropriate", "Compensation is as expected: a simple disorder.");
const RESP_ACIDOSIS: Band = (
    "expected_compensation.added_resp_acidosis",
    "pCO2 above expected",
    "pCO2 failed to fall to the expected value: a complicating respiratory acidosis.",
);
const RESP_ALKALOSIS: Band = (
    "expected_compensation.added_resp_alkalosis",
    "pCO2 below expected",
    "pCO2 fell more than expected: a complicating respiratory alkalosis.",
);
const MET_ALKALOSIS: Band = (
    "expected_compensation.added_met_alkalosis",
    "HCO3 above expected",
    "HCO3 is above the expected value: a complicating metabolic alkalosis.",
);
const MET_ACIDOSIS: Band = (
    "expected_compensation.added_met_acidosis",
    "HCO3 below expected",
    "HCO3 is below the expected value: a complicating metabolic acidosis.",
);

pub const OSMOLAL_GAP: CalcSpec = CalcSpec {
    id: "osmolal_gap",
    name: "Osmolal gap",
    area: Area::Renal,
    fields: &[
        num("measured_osm", "Measured osmolality", "mOsm/kg", 200.0, 500.0, 1.0),
        NA,
        num("glucose_mg_dl", "Glucose", "mg/dL", 10.0, 2000.0, 1.0),
        num("bun_mg_dl", "BUN", "mg/dL", 1.0, 300.0, 0.1).hint("Urea ÷ 2.14."),
    ],
    source: "Book Nephrology Lecture 1, p. 20",
    compute: osmolal_gap,
};

fn osmolal_gap(i: &Inputs) -> Result<Outcome, CalcError> {
    let measured = i.get_num("measured_osm")?;
    let na = i.get_num("na_meq_l")?;
    let glucose = i.get_num("glucose_mg_dl")?;
    let bun = i.get_num("bun_mg_dl")?;
    let calculated = 2.0 * na + glucose / 18.0 + bun / 2.8;
    let gap = ((measured - calculated) * 10.0).round() / 10.0;
    let (band, label, meaning) = if gap > 10.0 {
        (
            "osmolal_gap.present",
            "Gap present (> 10)",
            "An osmolal gap: unmeasured osmoles such as methanol, ethylene glycol, ethanol or isopropyl alcohol.",
        )
    } else {
        ("osmolal_gap.normal", "No gap (≤ 10)", "No osmolal gap.")
    };
    Ok(Outcome {
        score: Some(gap),
        display: format!("{} mOsm/kg", show(gap, 1)),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(20),
        parts: vec![
            Part::new("2 × Na", 2.0 * na),
            Part::new("Glucose ÷ 18", (glucose / 18.0 * 10.0).round() / 10.0),
            Part::new("BUN ÷ 2.8", (bun / 2.8 * 10.0).round() / 10.0),
        ],
        notes: vec![format!("Calculated osmolality {} mOsm/kg.", show(calculated, 1))],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ag(na: f64, cl: f64, hco3: f64) -> Inputs {
        Inputs::new().num("na_meq_l", na).num("cl_meq_l", cl).num("hco3_meq_l", hco3)
    }

    #[test]
    fn anion_gap_bands() {
        let o = anion_gap(&ag(140.0, 104.0, 24.0)).unwrap();
        assert_eq!((o.score, o.band), (Some(12.0), "anion_gap.normal"));
        assert_eq!(anion_gap(&ag(140.0, 100.0, 10.0)).unwrap().band, "anion_gap.high");
        assert_eq!(anion_gap(&ag(140.0, 110.0, 24.0)).unwrap().band, "anion_gap.low");
        // 16 is still normal; only above 16 is high.
        assert_eq!(anion_gap(&ag(140.0, 100.0, 24.0)).unwrap().band, "anion_gap.normal");
    }

    #[test]
    fn albumin_correction() {
        // Albumin 2.0: + 5 (the book's p. 9 example).
        let o = anion_gap(&ag(140.0, 108.0, 22.0).num("albumin_g_dl", 2.0)).unwrap();
        assert_eq!((o.score, o.band), (Some(15.0), "anion_gap.normal"));
        let o = anion_gap(&ag(140.0, 104.0, 24.0).num("albumin_g_dl", 4.5)).unwrap();
        assert_eq!(o.score, Some(12.0));
    }

    #[test]
    fn delta_ratio_bands() {
        let run = |ag: f64, hco3: f64| {
            delta_ratio(&Inputs::new().num("anion_gap_meq_l", ag).num("hco3_meq_l", hco3)).unwrap()
        };
        assert_eq!(run(12.0, 14.0).band, "delta_ratio.normal_ag");
        assert_eq!(run(18.0, 12.0).band, "delta_ratio.mixed");
        let o = run(24.0, 12.0);
        assert_eq!((o.score, o.band), (Some(1.0), "delta_ratio.pure_high"));
        assert_eq!(run(32.0, 18.0).band, "delta_ratio.with_alkalosis");
        assert!(delta_ratio(&Inputs::new().num("anion_gap_meq_l", 20.0).num("hco3_meq_l", 24.0)).is_err());
    }

    fn comp(disorder: &str, hco3: f64, pco2: f64) -> Outcome {
        compensation(
            &Inputs::new().choice("disorder", disorder).num("hco3_meq_l", hco3).num("pco2_mm_hg", pco2),
        )
        .unwrap()
    }

    #[test]
    fn winters_formula() {
        // HCO3 12: expected pCO2 26 ± 2.
        let o = comp("metabolic_acidosis", 12.0, 27.0);
        assert_eq!((o.score, o.band), (Some(26.0), "expected_compensation.appropriate"));
        assert_eq!(comp("metabolic_acidosis", 12.0, 35.0).band, "expected_compensation.added_resp_acidosis");
        assert_eq!(comp("metabolic_acidosis", 12.0, 20.0).band, "expected_compensation.added_resp_alkalosis");
        // HCO3 40: expected pCO2 48 ± 5.
        assert_eq!(comp("metabolic_alkalosis", 40.0, 52.0).band, "expected_compensation.appropriate");
    }

    #[test]
    fn respiratory_rules() {
        // pCO2 60: acute HCO3 26, chronic 32.
        assert_eq!(comp("respiratory_acidosis_acute", 26.0, 60.0).score, Some(26.0));
        assert_eq!(
            comp("respiratory_acidosis_chronic", 32.0, 60.0).band,
            "expected_compensation.appropriate"
        );
        assert_eq!(
            comp("respiratory_acidosis_acute", 32.0, 60.0).band,
            "expected_compensation.added_met_alkalosis"
        );
        // pCO2 20: acute HCO3 20, chronic 14.
        assert_eq!(comp("respiratory_alkalosis_acute", 20.0, 20.0).score, Some(20.0));
        assert_eq!(comp("respiratory_alkalosis_chronic", 14.0, 20.0).score, Some(14.0));
        assert_eq!(
            comp("respiratory_alkalosis_acute", 15.0, 20.0).band,
            "expected_compensation.added_met_acidosis"
        );
        let wrong = Inputs::new()
            .choice("disorder", "respiratory_acidosis_acute")
            .num("hco3_meq_l", 24.0)
            .num("pco2_mm_hg", 30.0);
        assert!(compensation(&wrong).is_err());
    }

    #[test]
    fn osmolal_gap_cut_off() {
        // Calculated 2×140 + 90/18 + 14/2.8 = 290.
        let run = |m: f64| {
            osmolal_gap(
                &Inputs::new()
                    .num("measured_osm", m)
                    .num("na_meq_l", 140.0)
                    .num("glucose_mg_dl", 90.0)
                    .num("bun_mg_dl", 14.0),
            )
            .unwrap()
        };
        assert_eq!((run(300.0).score, run(300.0).band), (Some(10.0), "osmolal_gap.normal"));
        assert_eq!(run(320.0).band, "osmolal_gap.present");
    }
}
