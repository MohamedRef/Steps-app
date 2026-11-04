//! Seattle / Baltimore criteria for sinusoidal obstruction syndrome (SOS,
//! veno-occlusive disease) after haematopoietic stem cell transplantation.
//!
//! Source: the book, Lecture 4, p. 81: consider SOS with two or more of
//! weight gain, ascites, tender hepatomegaly and jaundice within 20 days of
//! the transplant. The book merges the two sets: Seattle (McDonald 1984) is 2
//! of jaundice, painful hepatomegaly and weight gain or ascites within 20
//! days; Baltimore (Jones 1987) is bilirubin ≥ 2 mg/dL plus 2 of
//! hepatomegaly, ascites and weight gain ≥ 5% within 21 days.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, flag, num};

pub const SPEC: CalcSpec = CalcSpec {
    id: "sos_criteria",
    name: "Seattle / Baltimore criteria (SOS)",
    area: Area::Liver,
    fields: &[
        flag("weight_gain", "Weight gain"),
        flag("ascites", "Ascites"),
        flag("tender_hepatomegaly", "Tender hepatomegaly"),
        flag("jaundice", "Jaundice"),
        num("days_since_transplant", "Days since the transplant", "days", 0.0, 365.0, 1.0),
    ],
    source: "McDonald 1984, Jones 1987, as the book merges them, Lecture 4, p. 81",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let signs = [
        Part::met("Weight gain", i.get_flag("weight_gain")?),
        Part::met("Ascites", i.get_flag("ascites")?),
        Part::met("Tender hepatomegaly", i.get_flag("tender_hepatomegaly")?),
        Part::met("Jaundice", i.get_flag("jaundice")?),
    ];
    let days = i.get_num("days_since_transplant")?;
    let count: f64 = signs.iter().map(|p| p.points).sum();
    let early = days <= 20.0;
    let met = count >= 2.0 && early;
    let (band, label, meaning) = if met {
        (
            "sos_criteria.met",
            "Criteria met",
            "Consider SOS, after excluding mimics: liver GvHD, infection, sepsis, other drug injury, heart \
             disease, other causes of ascites and renal failure. Transjugular biopsy if unclear.",
        )
    } else {
        (
            "sos_criteria.not_met",
            "Criteria not met",
            "Clinical criteria not met. If SOS is still suspected or another diagnosis must be excluded, \
             transjugular liver biopsy.",
        )
    };
    let mut parts = signs.to_vec();
    parts.push(Part::met("Within 20 days of the transplant", early));
    Ok(Outcome {
        score: Some(count),
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(81),
        parts,
        notes: Vec::new(),
    })
}

pub const GUIDES: &[(&str, Guide)] = &[
    (
        "sos_criteria.met",
        Guide {
            next: &[
                "Doppler ultrasound: hepatomegaly, ascites, slow or reversed portal flow.",
                "Fluid and salt balance, diuretics; defibrotide for severe SOS.",
                "Transjugular biopsy (with pressure gradient) if the diagnosis is unclear.",
            ],
            dd: &[
                "Liver GvHD, sepsis-associated cholestasis, drug-induced liver injury (azoles, TPN), viral hepatitis \
                 (CMV, HBV reactivation), Budd-Chiari, heart failure or fluid overload, renal failure.",
            ],
        },
    ),
    (
        "sos_criteria.not_met",
        Guide {
            next: &[
                "Keep watching weight, liver size and bilirubin daily in the first 3 weeks after the transplant.",
            ],
            dd: &["Jaundice alone after BMT: GvHD, sepsis, drugs, haemolysis, viral hepatitis."],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_signs_within_20_days() {
        let two = Inputs::new().flag("ascites", true).flag("jaundice", true);
        assert_eq!(
            compute(&two.clone().num("days_since_transplant", 12.0)).unwrap().band,
            "sos_criteria.met"
        );
        assert_eq!(compute(&two.num("days_since_transplant", 30.0)).unwrap().band, "sos_criteria.not_met");
        let one = Inputs::new().flag("weight_gain", true).num("days_since_transplant", 5.0);
        assert_eq!(compute(&one).unwrap().band, "sos_criteria.not_met");
    }
}
