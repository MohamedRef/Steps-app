//! Acute kidney injury: KDIGO stage, FENa and FEurea.
//!
//! Source: the Nephrology book (Chapter 8, Lecture 5). KDIGO (p. 100,
//! Table 28-1): stage 1 creatinine 1.5–1.9 × baseline or a rise of
//! ≥ 0.3 mg/dL, urine < 0.5 mL/kg/h for 6–12 h; stage 2 creatinine
//! 2.0–2.9 ×, urine < 0.5 mL/kg/h for ≥ 12 h; stage 3 creatinine 3 ×, or
//! ≥ 4.0 mg/dL, or renal replacement therapy, urine < 0.3 mL/kg/h for ≥ 24 h
//! or anuria for ≥ 12 h. Prerenal azotemia vs ATN (p. 109): FENa below 1%
//! vs above 2%, FEurea below 35% vs above 50%. FE = (urine X × plasma Cr)
//! / (plasma X × urine Cr) × 100.

use super::{Area, CalcError, CalcSpec, Inputs, Outcome, Part, choice, flag, num, show};

const URINE_OUTPUT: &[(&str, &str)] = &[
    ("normal", "Not reduced"),
    ("lt05_6h", "< 0.5 mL/kg/h for 6–12 h"),
    ("lt05_12h", "< 0.5 mL/kg/h for ≥ 12 h"),
    ("lt03_24h_or_anuria", "< 0.3 mL/kg/h for ≥ 24 h, or anuria ≥ 12 h"),
];

pub const KDIGO: CalcSpec = CalcSpec {
    id: "kdigo_aki",
    name: "KDIGO AKI stage",
    area: Area::Renal,
    fields: &[
        num("baseline_cr_mg_dl", "Baseline creatinine", "mg/dL", 0.1, 20.0, 0.01),
        num("current_cr_mg_dl", "Current creatinine", "mg/dL", 0.1, 30.0, 0.01),
        choice("urine_output", "Urine output", URINE_OUTPUT),
        flag("rrt", "Renal replacement therapy started"),
    ],
    source: "KDIGO 2012; book Nephrology Lecture 5, p. 100",
    compute: kdigo,
};

fn kdigo(i: &Inputs) -> Result<Outcome, CalcError> {
    let base = i.get_pos("baseline_cr_mg_dl")?;
    let now = i.get_pos("current_cr_mg_dl")?;
    let uo = i.get_choice("urine_output", &["normal", "lt05_6h", "lt05_12h", "lt03_24h_or_anuria"])?;
    let rrt = i.get_flag("rrt")?;
    let ratio = now / base;
    let cr_stage = if ratio >= 3.0 || now >= 4.0 || rrt {
        3.0
    } else if ratio >= 2.0 {
        2.0
    } else if ratio >= 1.5 || now - base >= 0.3 {
        1.0
    } else {
        0.0
    };
    let uo_stage = match uo {
        "lt03_24h_or_anuria" => 3.0,
        "lt05_12h" => 2.0,
        "lt05_6h" => 1.0,
        _ => 0.0,
    };
    let stage = f64::max(cr_stage, uo_stage);
    let (band, label, meaning) = match stage as u8 {
        0 => (
            "kdigo_aki.none",
            "No AKI",
            "Neither the creatinine nor the urine output meets the AKI criteria.",
        ),
        1 => ("kdigo_aki.1", "Stage 1", "AKI stage 1."),
        2 => ("kdigo_aki.2", "Stage 2", "AKI stage 2."),
        _ => ("kdigo_aki.3", "Stage 3", "AKI stage 3."),
    };
    Ok(Outcome {
        score: Some(stage),
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(100),
        parts: vec![
            Part::new(format!("Creatinine × {} of baseline", show(ratio, 2)), cr_stage),
            Part::new("Urine output", uo_stage),
        ],
        notes: vec![
            "A rise of ≥ 0.3 mg/dL counts within 48 hours; a 1.5-fold rise within 7 days.".to_owned(),
            "The stage is the higher of the creatinine and urine-output stages.".to_owned(),
        ],
    })
}

pub const FENA: CalcSpec = CalcSpec {
    id: "fena",
    name: "FENa (fractional excretion of sodium)",
    area: Area::Renal,
    fields: &[
        num("urine_na_meq_l", "Urine sodium", "mEq/L", 1.0, 300.0, 1.0),
        num("plasma_na_meq_l", "Plasma sodium", "mEq/L", 90.0, 200.0, 1.0),
        num("urine_cr_mg_dl", "Urine creatinine", "mg/dL", 1.0, 1000.0, 0.1),
        num("plasma_cr_mg_dl", "Plasma creatinine", "mg/dL", 0.1, 30.0, 0.01),
    ],
    source: "Book Nephrology Lecture 5, p. 109",
    compute: fena,
};

pub const FEUREA: CalcSpec = CalcSpec {
    id: "feurea",
    name: "FEurea (fractional excretion of urea)",
    area: Area::Renal,
    fields: &[
        num("urine_urea_mg_dl", "Urine urea", "mg/dL", 1.0, 5000.0, 1.0)
            .hint("Urea or BUN, the same unit as the plasma value."),
        num("plasma_urea_mg_dl", "Plasma urea", "mg/dL", 1.0, 500.0, 0.1),
        num("urine_cr_mg_dl", "Urine creatinine", "mg/dL", 1.0, 1000.0, 0.1),
        num("plasma_cr_mg_dl", "Plasma creatinine", "mg/dL", 0.1, 30.0, 0.01),
    ],
    source: "Book Nephrology Lecture 5, p. 109",
    compute: feurea,
};

type Band = (&'static str, &'static str, &'static str);

fn fraction(
    i: &Inputs,
    (urine, plasma): (&'static str, &'static str),
    (low, high): (f64, f64),
    [pre, mid, atn]: [Band; 3],
) -> Result<Outcome, CalcError> {
    let ux = i.get_num(urine)?;
    let px = i.get_pos(plasma)?;
    let ucr = i.get_pos("urine_cr_mg_dl")?;
    let pcr = i.get_pos("plasma_cr_mg_dl")?;
    let fe = ((ux * pcr) / (px * ucr) * 100.0 * 100.0).round() / 100.0;
    let (band, label, meaning) = if fe < low {
        pre
    } else if fe > high {
        atn
    } else {
        mid
    };
    Ok(Outcome {
        score: Some(fe),
        display: format!("{}%", show(fe, 2)),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(109),
        parts: vec![Part::new("Urine ÷ plasma", ux / px), Part::new("Plasma Cr ÷ urine Cr", pcr / ucr)],
        notes: Vec::new(),
    })
}

fn fena(i: &Inputs) -> Result<Outcome, CalcError> {
    fraction(
        i,
        ("urine_na_meq_l", "plasma_na_meq_l"),
        (1.0, 2.0),
        [
            ("fena.prerenal", "< 1%", "Favours prerenal azotemia."),
            ("fena.indeterminate", "1–2%", "Between the cut-offs: read with the other urinary indices."),
            ("fena.atn", "> 2%", "Favours acute tubular necrosis."),
        ],
    )
}

fn feurea(i: &Inputs) -> Result<Outcome, CalcError> {
    fraction(
        i,
        ("urine_urea_mg_dl", "plasma_urea_mg_dl"),
        (35.0, 50.0),
        [
            ("feurea.prerenal", "< 35%", "Favours prerenal azotemia (useful after diuretics)."),
            ("feurea.indeterminate", "35–50%", "Between the cut-offs: read with the other urinary indices."),
            ("feurea.atn", "> 50%", "Favours acute tubular necrosis."),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage(base: f64, now: f64, uo: &str) -> Outcome {
        kdigo(
            &Inputs::new()
                .num("baseline_cr_mg_dl", base)
                .num("current_cr_mg_dl", now)
                .choice("urine_output", uo),
        )
        .unwrap()
    }

    #[test]
    fn kdigo_stages() {
        assert_eq!(stage(1.0, 1.2, "normal").band, "kdigo_aki.none");
        assert_eq!(stage(1.0, 1.3, "normal").band, "kdigo_aki.1"); // + 0.3
        assert_eq!(stage(1.0, 1.5, "normal").band, "kdigo_aki.1");
        assert_eq!(stage(1.0, 2.0, "normal").band, "kdigo_aki.2");
        assert_eq!(stage(1.0, 3.0, "normal").band, "kdigo_aki.3");
        assert_eq!(stage(2.5, 4.0, "normal").band, "kdigo_aki.3"); // ≥ 4.0
        assert_eq!(stage(1.0, 1.0, "lt05_12h").band, "kdigo_aki.2"); // urine wins
        let rrt = Inputs::new()
            .num("baseline_cr_mg_dl", 1.0)
            .num("current_cr_mg_dl", 1.1)
            .choice("urine_output", "normal")
            .flag("rrt", true);
        assert_eq!(kdigo(&rrt).unwrap().band, "kdigo_aki.3");
    }

    #[test]
    fn fractional_excretion() {
        // (10 × 2) / (140 × 100) × 100 = 0.14%.
        let i = Inputs::new()
            .num("urine_na_meq_l", 10.0)
            .num("plasma_na_meq_l", 140.0)
            .num("urine_cr_mg_dl", 100.0)
            .num("plasma_cr_mg_dl", 2.0);
        let o = fena(&i).unwrap();
        assert_eq!((o.score, o.band), (Some(0.14), "fena.prerenal"));
        let i = i.num("urine_na_meq_l", 80.0).num("urine_cr_mg_dl", 40.0);
        assert_eq!(fena(&i).unwrap().band, "fena.atn"); // 2.86%
        // (300 × 2) / (60 × 40) × 100 = 25%.
        let u = Inputs::new()
            .num("urine_urea_mg_dl", 300.0)
            .num("plasma_urea_mg_dl", 60.0)
            .num("urine_cr_mg_dl", 40.0)
            .num("plasma_cr_mg_dl", 2.0);
        assert_eq!(feurea(&u).unwrap().band, "feurea.prerenal");
    }
}
