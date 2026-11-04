//! ACR/EULAR 2010 classification criteria for rheumatoid arthritis.
//!
//! Source: the Rheumatology book (Chapter 3, Lecture 16, p. 246). Joints:
//! 1 medium-large 0, 2–10 medium-large 1, 1–3 small 2, 4–10 small 3, more
//! than 10 small 5. Serology (RF or ACPA): both negative 0, low positive
//! (above the ULN, up to 3 × ULN) 2, high positive (above 3 × ULN) 3.
//! Synovitis 6 weeks or longer 1. Abnormal CRP or ESR 1. Each domain scores
//! its highest level met (5 small + 4 large joints scores 3); 6 or more of
//! 10 is definite RA.

use super::{Area, CalcError, CalcSpec, Inputs, Outcome, Part, choice, flag, num};

const SEROLOGY: &[(&str, &str)] = &[
    ("negative", "RF and ACPA negative"),
    ("low", "Low positive (> ULN, ≤ 3 × ULN)"),
    ("high", "High positive (> 3 × ULN)"),
];

pub const SPEC: CalcSpec = CalcSpec {
    id: "acr_eular_2010_ra",
    name: "ACR/EULAR 2010 RA criteria",
    area: Area::Rheum,
    fields: &[
        num("large_joints", "Medium-large joints involved", "", 0.0, 40.0, 1.0)
            .hint("Shoulders, elbows, hips, knees, ankles."),
        num("small_joints", "Small joints involved", "", 0.0, 60.0, 1.0)
            .hint("MCPs, PIPs, 2nd–5th MTPs, thumb IPs, wrists."),
        choice("serology", "RF / ACPA", SEROLOGY),
        flag("synovitis_6_weeks", "Synovitis 6 weeks or longer"),
        flag("abnormal_crp_esr", "Abnormal CRP or ESR"),
    ],
    source: "Book Rheumatology Lecture 16, p. 246",
    compute,
};

fn joints(i: &Inputs, key: &'static str) -> Result<f64, CalcError> {
    let v = i.get_num(key)?;
    if v < 0.0 || v.fract() != 0.0 {
        return Err(CalcError::Invalid { field: key, reason: "must be a whole number, 0 or more" });
    }
    Ok(v)
}

fn joint_points(large: f64, small: f64) -> f64 {
    match (large, small) {
        (_, s) if s > 10.0 => 5.0,
        (_, s) if s >= 4.0 => 3.0,
        (_, s) if s >= 1.0 => 2.0,
        (l, _) if l >= 2.0 => 1.0,
        _ => 0.0,
    }
}

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let large = joints(i, "large_joints")?;
    let small = joints(i, "small_joints")?;
    let serology = match i.get_choice("serology", &["negative", "low", "high"])? {
        "high" => 3.0,
        "low" => 2.0,
        _ => 0.0,
    };
    let parts = vec![
        Part::new("Joints", joint_points(large, small)),
        Part::new("Serology", serology),
        Part::new("Synovitis ≥ 6 weeks", if i.get_flag("synovitis_6_weeks")? { 1.0 } else { 0.0 }),
        Part::new("Abnormal CRP or ESR", if i.get_flag("abnormal_crp_esr")? { 1.0 } else { 0.0 }),
    ];
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let mut notes = Vec::new();
    if large + small == 0.0 {
        notes.push("The criteria apply to a patient with at least one swollen joint.".to_owned());
    }
    if small == 0.0 && large > 10.0 {
        notes.push("The book lists no level above 2–10 medium-large joints, so this scores 1.".to_owned());
    }
    let (band, label, meaning) = if total >= 6.0 {
        ("acr_eular_2010_ra.definite", "6 or more", "Definite rheumatoid arthritis.")
    } else {
        ("acr_eular_2010_ra.not_met", "Below 6", "Criteria not met: not classifiable as RA yet.")
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{total} / 10"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(246),
        parts,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highest_joint_level_wins() {
        // The book's example: 5 small + 4 large joints scores 3.
        assert_eq!(joint_points(4.0, 5.0), 3.0);
        assert_eq!(
            [(1.0, 0.0), (2.0, 0.0), (10.0, 0.0), (0.0, 3.0), (0.0, 11.0)].map(|(l, s)| joint_points(l, s)),
            [0.0, 1.0, 1.0, 2.0, 5.0]
        );
    }

    #[test]
    fn definite_at_six() {
        let base = Inputs::new().num("large_joints", 0.0).num("small_joints", 5.0).choice("serology", "low");
        let o = compute(&base.clone().flag("synovitis_6_weeks", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(6.0), "acr_eular_2010_ra.definite"));
        let o = compute(&base).unwrap();
        assert_eq!((o.score, o.band), (Some(5.0), "acr_eular_2010_ra.not_met"));
        let all = Inputs::new()
            .num("large_joints", 2.0)
            .num("small_joints", 12.0)
            .choice("serology", "high")
            .flag("synovitis_6_weeks", true)
            .flag("abnormal_crp_esr", true);
        assert_eq!(compute(&all).unwrap().score, Some(10.0));
    }
}
