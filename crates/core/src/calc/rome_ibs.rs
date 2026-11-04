//! Rome IV criteria for irritable bowel syndrome, with the subtype.
//!
//! Source: the book's Gastroenterology Lecture 3 (pp. 125–126). Recurrent
//! abdominal pain on average at least 1 day a week in the last 3 months, with
//! two or more of: related to defecation, a change in stool frequency, a
//! change in stool form; criteria fulfilled for the last 3 months with onset
//! at least 6 months before diagnosis. Subtype by stool form on days with an
//! abnormal bowel movement: IBS-C more than 25% Bristol 1–2 and under 25%
//! Bristol 6–7; IBS-D the reverse; IBS-M more than 25% of each; IBS-U
//! neither.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, flag};

pub const SPEC: CalcSpec = CalcSpec {
    id: "rome4_ibs",
    name: "Rome IV criteria for IBS",
    area: Area::Gi,
    fields: &[
        flag(
            "pain_weekly",
            "Recurrent abdominal pain, on average 1 day a week or more, in the last 3 months",
        ),
        flag("onset_6_months", "Symptoms began 6 months or more before diagnosis"),
        flag("defecation", "Pain related to defecation"),
        flag("frequency", "Pain with a change in stool frequency"),
        flag("form", "Pain with a change in stool form"),
        flag("hard_25", "Bristol 1–2 in more than 25% of abnormal bowel movements"),
        flag("loose_25", "Bristol 6–7 in more than 25% of abnormal bowel movements"),
    ],
    source: "Book Gastroenterology Lecture 3, pp. 125–126 (Rome IV, 2016)",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let pain = i.get_flag("pain_weekly")?;
    let onset = i.get_flag("onset_6_months")?;
    let features = [
        Part::met("Related to defecation", i.get_flag("defecation")?),
        Part::met("Change in stool frequency", i.get_flag("frequency")?),
        Part::met("Change in stool form", i.get_flag("form")?),
    ];
    let count = features.iter().filter(|p| p.points > 0.0).count();
    let mut parts = vec![
        Part::met("Pain ≥ 1 day a week in the last 3 months", pain),
        Part::met("Onset ≥ 6 months before diagnosis", onset),
    ];
    parts.extend(features);
    let met = pain && onset && count >= 2;
    let (hard, loose) = (i.get_flag("hard_25")?, i.get_flag("loose_25")?);
    let (band, label, meaning) = match (met, hard, loose) {
        (false, ..) => (
            "rome4_ibs.not_met",
            "Not met",
            "Rome IV criteria for IBS are not met: needs weekly pain for 3 months, onset 6 months \
             ago, and 2 of the 3 features.",
        ),
        (true, true, false) => ("rome4_ibs.c", "IBS-C", "IBS with predominant constipation."),
        (true, false, true) => ("rome4_ibs.d", "IBS-D", "IBS with predominant diarrhoea."),
        (true, true, true) => ("rome4_ibs.m", "IBS-M", "IBS with mixed bowel habits."),
        (true, false, false) => {
            ("rome4_ibs.u", "IBS-U", "IBS, unclassified: the bowel habit fits none of the other subtypes.")
        }
    };
    let mut notes = Vec::new();
    if met {
        notes.push("Subtype is reliable only off drugs that change bowel habit.".to_owned());
    }
    Ok(Outcome {
        score: Some(count as f64),
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(125),
        parts,
        notes,
    })
}

/// Next steps and differential diagnosis for each band (Rome IV, Lacy 2016;
/// BSG 2021 IBS guideline; book Lecture 3).
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "rome4_ibs.not_met",
        Guide {
            next: &[
                "Look for another functional bowel disorder (functional diarrhoea, constipation or bloating) or an organic cause.",
            ],
            dd: &[
                "Coeliac disease",
                "Inflammatory bowel disease",
                "Microscopic colitis",
                "Bile acid diarrhoea",
                "Colorectal cancer",
                "Thyroid disease",
                "Giardiasis",
            ],
        },
    ),
    (
        "rome4_ibs.c",
        Guide {
            next: &[
                "Look for red flags first: age over 50, rectal bleeding, weight loss, anaemia, night symptoms, family history of colorectal cancer or IBD.",
                "Basic tests: CBC, CRP, coeliac serology; faecal calprotectin if there is diarrhoea.",
                "Explain the diagnosis; regular exercise; a dietitian-led low-FODMAP diet if needed; low-dose tricyclic for pain.",
                "Constipation: soluble fibre (ispaghula), then osmotic laxatives (PEG); linaclotide if these fail.",
            ],
            dd: &[
                "Functional constipation",
                "Hypothyroidism",
                "Hypercalcaemia",
                "Colorectal cancer",
                "Drugs: opioids, anticholinergics, calcium channel blockers, iron",
                "Dyssynergic defecation",
            ],
        },
    ),
    (
        "rome4_ibs.d",
        Guide {
            next: &[
                "Look for red flags first: age over 50, rectal bleeding, weight loss, anaemia, night symptoms, family history of colorectal cancer or IBD.",
                "Basic tests: CBC, CRP, coeliac serology; faecal calprotectin if there is diarrhoea.",
                "Explain the diagnosis; regular exercise; a dietitian-led low-FODMAP diet if needed; low-dose tricyclic for pain.",
                "Diarrhoea: loperamide; 5-HT3 antagonists (ondansetron, ramosetron) or rifaximin if needed.",
            ],
            dd: &[
                "Coeliac disease",
                "Microscopic colitis",
                "Inflammatory bowel disease",
                "Bile acid diarrhoea",
                "Lactose intolerance",
                "Giardiasis",
                "Hyperthyroidism",
            ],
        },
    ),
    (
        "rome4_ibs.m",
        Guide {
            next: &[
                "Look for red flags first: age over 50, rectal bleeding, weight loss, anaemia, night symptoms, family history of colorectal cancer or IBD.",
                "Basic tests: CBC, CRP, coeliac serology; faecal calprotectin if there is diarrhoea.",
                "Explain the diagnosis; regular exercise; a dietitian-led low-FODMAP diet if needed; low-dose tricyclic for pain.",
                "Treat the main symptom of the moment; antispasmodics for pain.",
            ],
            dd: &[
                "Coeliac disease",
                "Inflammatory bowel disease",
                "Microscopic colitis",
                "Colorectal cancer",
                "Endometriosis in women",
            ],
        },
    ),
    (
        "rome4_ibs.u",
        Guide {
            next: &[
                "Look for red flags first: age over 50, rectal bleeding, weight loss, anaemia, night symptoms, family history of colorectal cancer or IBD.",
                "Basic tests: CBC, CRP, coeliac serology; faecal calprotectin if there is diarrhoea.",
                "Explain the diagnosis; regular exercise; a dietitian-led low-FODMAP diet if needed; low-dose tricyclic for pain.",
                "Antispasmodics for pain.",
            ],
            dd: &[
                "Coeliac disease",
                "Inflammatory bowel disease",
                "Colorectal cancer",
                "Endometriosis or ovarian cancer in women",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn ibs() -> Inputs {
        Inputs::new()
            .flag("pain_weekly", true)
            .flag("onset_6_months", true)
            .flag("defecation", true)
            .flag("form", true)
    }

    #[test]
    fn two_of_three_features_with_the_time_rules() {
        assert_eq!(compute(&ibs()).unwrap().band, "rome4_ibs.u");
        assert_eq!(compute(&ibs().flag("onset_6_months", false)).unwrap().band, "rome4_ibs.not_met");
        assert_eq!(compute(&ibs().flag("form", false)).unwrap().band, "rome4_ibs.not_met");
        assert_eq!(compute(&Inputs::new()).unwrap().band, "rome4_ibs.not_met");
    }

    #[test]
    fn subtypes_by_stool_form() {
        assert_eq!(compute(&ibs().flag("hard_25", true)).unwrap().band, "rome4_ibs.c");
        assert_eq!(compute(&ibs().flag("loose_25", true)).unwrap().band, "rome4_ibs.d");
        assert_eq!(compute(&ibs().flag("hard_25", true).flag("loose_25", true)).unwrap().band, "rome4_ibs.m");
    }
}
