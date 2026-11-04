//! Revised Atlanta classification (2012) of acute pancreatitis severity.
//!
//! Source: the book's table, Lecture 15, p. 285. Mild: no organ failure and
//! no local or systemic complications. Moderately severe: organ failure that
//! resolves within 48 h, and/or local or systemic complications without
//! persistent organ failure. Severe: organ failure persisting > 48 h (single
//! or multiple). The book's organ-failure list is on the same page.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice, flag};

const ORGAN_FAILURE: &[(&str, &str)] =
    &[("none", "None"), ("transient", "Resolves within 48 h"), ("persistent", "Persists over 48 h")];

pub const SPEC: CalcSpec = CalcSpec {
    id: "atlanta_2012",
    name: "Revised Atlanta severity (2012)",
    area: Area::Pancreas,
    fields: &[
        choice("organ_failure", "Organ failure", ORGAN_FAILURE).hint(
            "Shock (SBP < 90 or MAP < 70), PaO2 < 60 or ventilation, creatinine > 2 mg/dL after fluids or \
             dialysis, GI bleeding > 500 mL/24 h, pulse > 90.",
        ),
        flag("complications", "Local or systemic complications"),
    ],
    source: "Banks 2013 (Atlanta 2012); book Lecture 15, table p. 285",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let of = i.get_option(
        "organ_failure",
        &[
            ("none", &["no"]),
            ("transient", &["resolves", "lt48", "<48h", "<_48_h"]),
            ("persistent", &["persists", "gt48", ">48h", ">_48_h"]),
        ],
    )?;
    let comp = i.get_flag("complications")?;
    let (band, label, meaning) = match (of, comp) {
        ("persistent", _) => (
            "atlanta_2012.severe",
            "Severe",
            "Persistent organ failure (> 48 h), single or multiple. Usually necrotizing pancreatitis.",
        ),
        ("transient", _) | (_, true) => (
            "atlanta_2012.moderately_severe",
            "Moderately severe",
            "Transient organ failure and/or local or systemic complications without persistent organ failure. \
             Usually necrotizing pancreatitis.",
        ),
        _ => (
            "atlanta_2012.mild",
            "Mild",
            "No organ failure and no local or systemic complications. Usually interstitial pancreatitis.",
        ),
    };
    Ok(Outcome {
        score: None,
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(285),
        parts: vec![
            Part::met("Organ failure", of != "none"),
            Part::met("Organ failure > 48 h", of == "persistent"),
            Part::met("Local or systemic complications", comp),
        ],
        notes: Vec::new(),
    })
}

const DD: &str = "Other causes of severe epigastric pain with a raised amylase: perforated peptic ulcer, mesenteric \
                  ischaemia, dissecting aortic aneurysm, myocardial infarction, biliary colic or cholecystitis, \
                  volvulus (p. 283).";

pub const GUIDES: &[(&str, Guide)] = &[
    (
        "atlanta_2012.mild",
        Guide {
            next: &[
                "Fluids and analgesia; oral low-fat solid diet as soon as there is no nausea or pain.",
                "Gallstones: cholecystectomy in the same admission.",
            ],
            dd: &[DD],
        },
    ),
    (
        "atlanta_2012.moderately_severe",
        Guide {
            next: &[
                "Contrast CT if not improving after 48–72 h, to look for local complications.",
                "Enteral (nasogastric) feeding if oral is not tolerated.",
            ],
            dd: &[DD],
        },
    ),
    (
        "atlanta_2012.severe",
        Guide {
            next: &[
                "ICU: goal-directed fluids, organ support, enteral feeding.",
                "No routine antibiotic prophylaxis; treat proven or suspected infection.",
            ],
            dd: &[DD],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn run(of: &str, comp: bool) -> &'static str {
        compute(&Inputs::new().choice("organ_failure", of).flag("complications", comp)).unwrap().band
    }

    #[test]
    fn grades() {
        assert_eq!(run("none", false), "atlanta_2012.mild");
        assert_eq!(run("none", true), "atlanta_2012.moderately_severe");
        assert_eq!(run("transient", false), "atlanta_2012.moderately_severe");
        assert_eq!(run("persistent", false), "atlanta_2012.severe");
        assert_eq!(run("persistent", true), "atlanta_2012.severe");
    }
}
