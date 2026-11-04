//! Ulcerative colitis activity: mild, moderate or severe.
//!
//! Source: the book's Table 4, "Mayo score for assessment of UC activity"
//! (Gastroenterology Lecture 6, p. 222). The table is the Truelove and Witts
//! grading (1955), not the 0–12 Mayo score with endoscopy: mild is 4 or fewer
//! stools a day, no more than small amounts of blood and no systemic upset;
//! severe is 6 or more a day with visible blood plus at least one systemic
//! feature (temperature > 37.8 °C, pulse ≥ 90, anaemia, ESR > 30 mm/h);
//! moderate is in between.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice, flag, num};

const BLOOD: &[(&str, &str)] = &[
    ("small", "None or small amounts"),
    ("between", "Between small and visible"),
    ("visible", "Visible blood"),
];

pub const SPEC: CalcSpec = CalcSpec {
    id: "uc_activity",
    name: "UC activity (Truelove and Witts)",
    area: Area::Gi,
    fields: &[
        num("stools_per_day", "Bowel movements a day", "", 0.0, 40.0, 1.0),
        choice("blood", "Blood in stools", BLOOD),
        flag("fever", "Temperature above 37.8 °C"),
        flag("pulse_90", "Pulse 90/min or more"),
        flag("anaemia", "Anaemia"),
        num("esr_mm_h", "ESR", "mm/h", 0.0, 150.0, 1.0).optional(),
    ],
    source: "Book Gastroenterology Lecture 6, Table 4 (p. 222), titled there \"Mayo score\"; \
             the grading is Truelove and Witts 1955",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let stools = i.get_num("stools_per_day")?;
    let blood = i.get_option(
        "blood",
        &[("small", &["none", "no", "mild"]), ("between", &["moderate"]), ("visible", &["severe", "yes"])],
    )?;
    let esr = i.opt_num("esr_mm_h")?.is_some_and(|v| v > 30.0);
    let systemic = [
        Part::met("Temperature > 37.8 °C", i.get_flag("fever")?),
        Part::met("Pulse ≥ 90/min", i.get_flag("pulse_90")?),
        Part::met("Anaemia", i.get_flag("anaemia")?),
        Part::met("ESR > 30 mm/h", esr),
    ];
    let upset = systemic.iter().filter(|p| p.points > 0.0).count();
    let mut parts = vec![Part::new("Bowel movements a day", stools)];
    parts.extend(systemic);
    let mut notes = Vec::new();
    let (band, label, meaning) = if stools >= 6.0 && upset > 0 {
        if blood != "visible" {
            notes.push("The book's severe column also has visible blood.".to_owned());
        }
        (
            "uc_activity.severe",
            "Severe",
            "Acute severe UC: admit for IV steroids, and plan rescue therapy if no response.",
        )
    } else if stools <= 4.0 && blood == "small" && upset == 0 {
        ("uc_activity.mild", "Mild", "Mild activity: treat as an outpatient.")
    } else {
        ("uc_activity.moderate", "Moderate", "Moderate activity: between mild and severe.")
    };
    if stools >= 6.0 && upset == 0 {
        notes.push("6 or more stools a day is severe only with a systemic feature.".to_owned());
    }
    Ok(Outcome {
        score: None,
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(222),
        parts,
        notes,
    })
}

/// Next steps and differential diagnosis for each band (ECCO 2022 UC
/// guideline; BSG 2019 IBD guideline, Travis criteria for day 3; book Table 5
/// on p. 223 for the differential).
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "uc_activity.mild",
        Guide {
            next: &[
                "Mesalazine (5-ASA): rectal for proctitis, oral plus rectal for left-sided or extensive colitis.",
                "Stool culture and C. difficile toxin with every flare.",
                "No response in 2–4 weeks: add oral prednisolone.",
            ],
            dd: &[
                "Infective colitis: Campylobacter, Salmonella, Shigella, E. coli O157, C. difficile, amoebiasis, CMV",
                "Crohn's colitis",
                "Ischaemic colitis",
                "Drug-induced colitis (NSAIDs)",
                "Radiation proctitis",
                "Diverticular colitis",
                "Microscopic colitis (watery diarrhoea, no blood)",
            ],
        },
    ),
    (
        "uc_activity.moderate",
        Guide {
            next: &[
                "High-dose oral mesalazine plus rectal 5-ASA.",
                "Not settling: oral prednisolone 40 mg/day, tapered over 6–8 weeks.",
                "Steroid-dependent or refractory: thiopurine, a biologic (anti-TNF, vedolizumab, ustekinumab) or a JAK inhibitor.",
                "Stool culture and C. difficile toxin.",
            ],
            dd: &[
                "Infective colitis: Campylobacter, Salmonella, Shigella, E. coli O157, C. difficile, amoebiasis, CMV",
                "Crohn's colitis",
                "Ischaemic colitis",
                "Drug-induced colitis (NSAIDs)",
                "Radiation proctitis",
                "Diverticular colitis",
                "Microscopic colitis (watery diarrhoea, no blood)",
            ],
        },
    ),
    (
        "uc_activity.severe",
        Guide {
            next: &[
                "Admit: IV hydrocortisone 100 mg 6-hourly or methylprednisolone 60 mg/day.",
                "Stool culture and C. difficile; flexible sigmoidoscopy with biopsies for CMV.",
                "Abdominal X-ray for dilatation (colon above 5.5 cm = toxic megacolon); VTE prophylaxis.",
                "Day 3: more than 8 stools, or 3–8 with CRP above 45 mg/L, predicts colectomy: rescue infliximab or ciclosporin.",
                "Involve the colorectal surgeon from the start.",
            ],
            dd: &[
                "C. difficile or CMV superinfection",
                "Infective colitis",
                "Toxic megacolon or perforation",
                "Ischaemic colitis",
                "Crohn's colitis",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_columns() {
        let i = Inputs::new().num("stools_per_day", 3.0).choice("blood", "small");
        assert_eq!(compute(&i).unwrap().band, "uc_activity.mild");
        let i = Inputs::new().num("stools_per_day", 5.0).choice("blood", "between");
        assert_eq!(compute(&i).unwrap().band, "uc_activity.moderate");
        let i = Inputs::new().num("stools_per_day", 8.0).choice("blood", "visible").num("esr_mm_h", 45.0);
        assert_eq!(compute(&i).unwrap().band, "uc_activity.severe");
    }

    #[test]
    fn six_stools_without_systemic_upset_is_not_severe() {
        let i = Inputs::new().num("stools_per_day", 7.0).choice("blood", "visible").num("esr_mm_h", 30.0);
        let o = compute(&i).unwrap();
        assert_eq!(o.band, "uc_activity.moderate");
        assert_eq!(o.notes.len(), 1);
        let o = compute(&i.flag("pulse_90", true)).unwrap();
        assert_eq!(o.band, "uc_activity.severe");
    }

    #[test]
    fn a_systemic_feature_lifts_mild_to_moderate() {
        let i = Inputs::new().num("stools_per_day", 2.0).choice("blood", "small").flag("anaemia", true);
        assert_eq!(compute(&i).unwrap().band, "uc_activity.moderate");
    }
}
