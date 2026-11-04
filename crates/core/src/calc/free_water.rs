//! Free water deficit and replacement volume in hypernatraemia.
//!
//! Source: the Nephrology book (Chapter 8, Lecture 6). Hypernatraemia is
//! Na > 146 mmol/L (p. 133). p. 134: "calculate water deficit", then the
//! replacement volume from the sodium in the fluid used, with 154 mEq/L as
//! the reference; replace it over 48–72 hours. p. 135: in chronic
//! hypernatraemia correct at no more than 0.5 mmol/L/h (no more than
//! 10 mmol/L/day), half the deficit in the first 12–24 hours.
//!
//! The book doesn't print the deficit formula; the usual one is used:
//! TBW = weight × 0.6 (men), 0.5 (women), 0.5 (older men), 0.45 (older
//! women); deficit = TBW × (Na / 140 − 1). The book prints the replacement
//! volume as "TBW deficit × 1+ [1-(Na in fluid / 154)]", a misprint of the
//! standard deficit × 1 / [1 − (Na in fluid / 154)]: half-normal saline is
//! half free water, so it takes twice the deficit.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice, flag, num, show};

const SEX: &[(&str, &str)] = &[("male", "Man"), ("female", "Woman")];

/// (id, label); the sodium in each is in [`fluid_na`].
const FLUIDS: &[(&str, &str)] = &[
    ("water", "Water by mouth / NG, or 5% dextrose (0 mEq/L)"),
    ("saline_0_2", "0.2% saline (34 mEq/L)"),
    ("saline_0_45", "0.45% (half-normal) saline (77 mEq/L)"),
];

fn fluid_na(id: &str) -> f64 {
    match id {
        "saline_0_2" => 34.0,
        "saline_0_45" => 77.0,
        _ => 0.0,
    }
}

pub const SPEC: CalcSpec = CalcSpec {
    id: "free_water_deficit",
    name: "Free water deficit (hypernatraemia)",
    area: Area::Renal,
    fields: &[
        num("na_meq_l", "Serum sodium", "mEq/L", 100.0, 220.0, 1.0),
        num("weight_kg", "Weight", "kg", 2.0, 300.0, 0.1),
        choice("sex", "Sex", SEX),
        flag("elderly", "Elderly (65 or over)"),
        choice("fluid", "Replacement fluid", FLUIDS),
    ],
    source: "Book Nephrology Lecture 6, pp. 133–135 (replacement volume and pace); deficit = \
             TBW × (Na/140 − 1) and the TBW fractions from Adrogué & Madias, NEJM 2000 (the book \
             doesn't print them)",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let na = i.get_num("na_meq_l")?;
    let weight = i.get_pos("weight_kg")?;
    let woman = i.get_choice("sex", &["male", "female"])? == "female";
    let elderly = i.get_flag("elderly")?;
    let fluid = i.get_choice("fluid", &["water", "saline_0_2", "saline_0_45"])?;
    if na <= 140.0 {
        return Err(CalcError::Invalid { field: "na_meq_l", reason: "must be above 140 (no water deficit)" });
    }
    let factor = match (woman, elderly) {
        (false, false) => 0.6,
        (true, false) | (false, true) => 0.5,
        (true, true) => 0.45,
    };
    let tbw = weight * factor;
    let deficit = tbw * (na / 140.0 - 1.0);
    let fna = fluid_na(fluid);
    let volume = deficit / (1.0 - fna / 154.0);
    let volume = (volume * 10.0).round() / 10.0;
    let deficit_r = (deficit * 10.0).round() / 10.0;
    let mut notes = Vec::new();
    if fna > 0.0 {
        notes.push(
            "The book prints the volume as \"deficit × 1+ [1-(Na/154)]\"; the standard \
             deficit ÷ (1 − Na/154) is used."
                .to_owned(),
        );
    }
    Ok(Outcome {
        score: Some(volume),
        display: format!("{} L to give (water deficit {} L)", show(volume, 1), show(deficit_r, 1)),
        band: "free_water_deficit.volume",
        label: "Replacement volume",
        meaning: format!("Give about {} L over 48–72 hours (p. 134), plus ongoing losses.", show(volume, 1)),
        page: Some(134),
        parts: vec![
            Part::new(format!("Total body water = weight × {factor}"), (tbw * 10.0).round() / 10.0),
            Part::new("Deficit = TBW × (Na/140 − 1)", deficit_r),
            Part::new("÷ (1 − fluid Na/154)", ((1.0 - fna / 154.0) * 100.0).round() / 100.0),
        ],
        notes,
    })
}

pub const GUIDES: &[(&str, Guide)] = &[(
    "free_water_deficit.volume",
    Guide {
        next: &[
            "Chronic (> 24 h): no faster than 0.5 mmol/L/h and no more than 10 mmol/L a day; half the deficit in the first 12–24 h, the rest over the next 24–48 h (p. 135)",
            "Acute (< 24 h): can be corrected at 1 mmol/L/h (p. 134)",
            "Hypovolaemic: isotonic saline first until the circulation is restored (p. 134)",
            "Add ongoing losses; check Na every 2 hours at first, then every 4 (p. 135)",
            "Urine osmolality: > 600 mOsm/kg means losses outside the kidney or too little water; < 300 means diabetes insipidus (desmopressin: central rises, nephrogenic doesn't) (Table 2, p. 133)",
        ],
        dd: &[
            "No access to water: elderly, immobile, intubated",
            "GI or skin losses: diarrhoea, vomiting, fever, sweating",
            "Loop diuretics; osmotic diuresis (glucose, urea, recovering ATN)",
            "Central DI: head trauma, surgery, tumour, infiltration",
            "Nephrogenic DI: lithium, demeclocycline, hypercalcaemia, hypokalaemia",
            "Salt excess: hypertonic saline, sodium bicarbonate, TPN or concentrated tube feeds",
        ],
    },
)];

#[cfg(test)]
mod tests {
    use super::*;

    fn run(na: f64, kg: f64, sex: &str, elderly: bool, fluid: &str) -> Outcome {
        compute(
            &Inputs::new()
                .num("na_meq_l", na)
                .num("weight_kg", kg)
                .choice("sex", sex)
                .flag("elderly", elderly)
                .choice("fluid", fluid),
        )
        .unwrap()
    }

    #[test]
    fn deficit_and_volume() {
        // 70 kg man, Na 160: TBW 42 L × (160/140 − 1) = 6.0 L.
        let o = run(160.0, 70.0, "male", false, "water");
        assert_eq!(o.score, Some(6.0));
        // Half-normal saline is half free water: twice the volume.
        assert_eq!(run(160.0, 70.0, "male", false, "saline_0_45").score, Some(12.0));
        // Older woman: × 0.45 → 31.5 L × 1/7 = 4.5 L.
        assert_eq!(run(160.0, 70.0, "female", true, "water").score, Some(4.5));
        // 0.2% saline: 6 / (1 − 34/154) = 7.7 L.
        assert_eq!(run(160.0, 70.0, "male", false, "saline_0_2").score, Some(7.7));
    }

    #[test]
    fn no_deficit_at_140() {
        assert!(
            compute(
                &Inputs::new()
                    .num("na_meq_l", 140.0)
                    .num("weight_kg", 70.0)
                    .choice("sex", "male")
                    .choice("fluid", "water")
            )
            .is_err()
        );
    }
}
