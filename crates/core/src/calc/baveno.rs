//! Baveno VI criteria: who with compensated advanced chronic liver disease
//! (cACLD) can skip screening endoscopy for varices.
//!
//! Source: the book, Lecture 2, p. 35 (EASL 2021): liver stiffness by
//! transient elastography < 20 kPa and platelets > 150 ×10⁹/L rule out
//! high-risk varices, in cACLD from untreated viral hepatitis, HIV-HCV,
//! alcohol, NAFLD, PBC and PSC. The same page: LSM > 20–25 kPa diagnoses
//! clinically significant portal hypertension (CSPH).

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, num};

pub const SPEC: CalcSpec = CalcSpec {
    id: "baveno_vi",
    name: "Baveno VI (skip variceal screening)",
    area: Area::Liver,
    fields: &[
        num("lsm_kpa", "Liver stiffness (TE)", "kPa", 1.0, 75.0, 0.1),
        num("platelets_10e9_l", "Platelets", "×10⁹/L", 1.0, 1500.0, 1.0),
    ],
    source: "Baveno VI 2015; book Lecture 2, p. 35 (EASL 2021)",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let lsm = i.get_pos("lsm_kpa")?;
    let plt = i.get_pos("platelets_10e9_l")?;
    let soft = lsm < 20.0;
    let plt_ok = plt > 150.0;
    let (band, label, meaning) = if soft && plt_ok {
        (
            "baveno_vi.spare",
            "Criteria met: skip endoscopy",
            "High-risk varices ruled out: no screening endoscopy now. Repeat TE and platelets yearly.",
        )
    } else {
        (
            "baveno_vi.screen",
            "Criteria not met: screen",
            "High-risk varices not ruled out: do a screening endoscopy.",
        )
    };
    let mut notes = Vec::new();
    if lsm > 20.0 {
        notes.push(
            "LSM > 20–25 kPa diagnoses clinically significant portal hypertension in cACLD (p. 35)."
                .to_owned(),
        );
    }
    Ok(Outcome {
        score: None,
        display: label.split(':').next().unwrap_or(label).to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(35),
        parts: vec![Part::met("Liver stiffness < 20 kPa", soft), Part::met("Platelets > 150 ×10⁹/L", plt_ok)],
        notes,
    })
}

pub const GUIDES: &[(&str, Guide)] = &[
    (
        "baveno_vi.spare",
        Guide {
            next: &["No screening endoscopy now; repeat elastography and platelets every year."],
            dd: &[],
        },
    ),
    (
        "baveno_vi.screen",
        Guide {
            next: &[
                "Screening endoscopy for varices.",
                "High-risk varices: non-selective beta-blocker (carvedilol) or band ligation.",
            ],
            dd: &[
                "Low platelets in cACLD: hypersplenism, alcohol marrow suppression, ITP (HCV), drugs, folate deficiency.",
                "High stiffness without fibrosis: hepatic congestion (heart failure), acute hepatitis, biliary \
                 obstruction, a recent meal, infiltration (amyloid, lymphoma).",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn run(lsm: f64, plt: f64) -> Outcome {
        compute(&Inputs::new().num("lsm_kpa", lsm).num("platelets_10e9_l", plt)).unwrap()
    }

    #[test]
    fn both_needed() {
        assert_eq!(run(15.0, 180.0).band, "baveno_vi.spare");
        assert_eq!(run(15.0, 150.0).band, "baveno_vi.screen");
        assert_eq!(run(20.0, 200.0).band, "baveno_vi.screen");
        let o = run(28.0, 120.0);
        assert_eq!(o.band, "baveno_vi.screen");
        assert_eq!(o.notes.len(), 1);
    }
}
