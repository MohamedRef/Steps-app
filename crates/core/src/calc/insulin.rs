//! Starting dose of premixed (biphasic) insulin twice daily.
//!
//! Source: the Diabetology book, Lecture 10, p. 128: for type 2 diabetes not
//! controlled on a once-daily regimen, start 0.3 U/kg/day, two thirds before
//! breakfast and one third before dinner, then titrate by FBG and PPG.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, num};

pub const PREMIXED: CalcSpec = CalcSpec {
    id: "premixed_insulin",
    name: "Premixed insulin starting dose",
    area: Area::Diabetes,
    fields: &[num("weight_kg", "Body weight", "kg", 20.0, 300.0, 0.5)],
    source: "Book Diabetology Lecture 10, p. 128",
    compute: premixed,
};

fn premixed(i: &Inputs) -> Result<Outcome, CalcError> {
    let weight = i.get_pos("weight_kg")?;
    let total = (0.3 * weight).round();
    let breakfast = (total * 2.0 / 3.0).round();
    let dinner = total - breakfast;
    Ok(Outcome {
        score: Some(total),
        display: format!("{total} U/day"),
        band: "premixed_insulin.dose",
        label: "Starting daily dose",
        meaning: format!(
            "{breakfast} U before breakfast and {dinner} U before dinner. Titrate by fasting and \
             post-prandial glucose."
        ),
        page: Some(128),
        parts: vec![
            Part::new("0.3 U/kg/day", total),
            Part::new("Before breakfast (2/3)", breakfast),
            Part::new("Before dinner (1/3)", dinner),
        ],
        notes: vec!["Rounded to whole units.".to_owned()],
    })
}

/// Next steps for the dose (book p. 128; ADA Standards of Care 2025 §9).
pub const GUIDES: &[(&str, Guide)] = &[(
    "premixed_insulin.dose",
    Guide {
        next: &[
            "Titrate every 3–7 days by about 10%: the evening dose by the next fasting glucose, the morning dose by the pre-dinner glucose.",
            "Three regular meals a day; don't skip one.",
            "Stop the sulfonylurea when premixed insulin starts.",
            "Night or pre-lunch hypoglycaemia: lower the dose before it.",
        ],
        dd: &[],
    },
)];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_thirds_and_one_third() {
        let o = premixed(&Inputs::new().num("weight_kg", 90.0)).unwrap();
        assert_eq!(o.score, Some(27.0));
        assert_eq!(o.parts[1].points, 18.0);
        assert_eq!(o.parts[2].points, 9.0);
        let o = premixed(&Inputs::new().num("weight_kg", 70.0)).unwrap();
        assert_eq!((o.parts[0].points, o.parts[1].points, o.parts[2].points), (21.0, 14.0, 7.0));
    }
}
