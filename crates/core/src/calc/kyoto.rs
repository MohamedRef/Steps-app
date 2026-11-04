//! Kyoto classification score of gastritis.
//!
//! Source: the book's Table 2 (Gastroenterology Lecture 1, p. 16;
//! Toyoshima 2020). Atrophy 0 (C0–CI), 1 (CII–CIII), 2 (OI–OIII); intestinal
//! metaplasia 0, 1 (within the antrum), 2 (up to the corpus); enlarged folds
//! (≥ 5 mm) 1; nodularity 1; diffuse redness 0, 1 (mild), 2 (severe), graded by the
//! translucency of the collecting venules in the body. Total 0–8: 0 no
//! H. pylori, ≥ 2 current infection, ≥ 4 higher gastric cancer risk (pp. 16,
//! 21).

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice, flag};

const ATROPHY: &[(&str, &str)] =
    &[("none", "None: C0–CI"), ("mild", "Mild: CII–CIII"), ("severe", "Severe: OI–OIII")];
const IM: &[(&str, &str)] =
    &[("none", "None"), ("mild", "Mild: within the antrum"), ("severe", "Severe: up to the corpus")];
const REDNESS: &[(&str, &str)] = &[("none", "None"), ("mild", "Mild"), ("severe", "Severe")];

pub const SPEC: CalcSpec = CalcSpec {
    id: "kyoto_gastritis",
    name: "Kyoto classification score (gastritis)",
    area: Area::Gi,
    fields: &[
        choice("atrophy", "Mucosal atrophy (Kimura-Takemoto)", ATROPHY),
        choice("intestinal_metaplasia", "Intestinal metaplasia", IM),
        flag("enlarged_folds", "Enlarged folds (5 mm or wider)"),
        flag("nodularity", "Nodularity (small nodules in the antrum)"),
        choice("diffuse_redness", "Diffuse redness", REDNESS)
            .hint("The book grades it by the translucency of the collecting venules in the body."),
    ],
    source: "Toyoshima 2020; book Gastroenterology Lecture 1, Table 2 (p. 16) and p. 21",
    compute,
};

fn grade(i: &Inputs, key: &'static str) -> Result<f64, CalcError> {
    let g = i.get_option(key, &[("none", &["0", "c0", "ci", "c1"]), ("mild", &["1"]), ("severe", &["2"])])?;
    Ok(match g {
        "mild" => 1.0,
        "severe" => 2.0,
        _ => 0.0,
    })
}

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let one = |key: &'static str| -> Result<f64, CalcError> { Ok(if i.get_flag(key)? { 1.0 } else { 0.0 }) };
    let parts = vec![
        Part::new("Mucosal atrophy", grade(i, "atrophy")?),
        Part::new("Intestinal metaplasia", grade(i, "intestinal_metaplasia")?),
        Part::new("Enlarged folds", one("enlarged_folds")?),
        Part::new("Nodularity", one("nodularity")?),
        Part::new("Diffuse redness", grade(i, "diffuse_redness")?),
    ];
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = match total {
        t if t >= 4.0 => (
            "kyoto_gastritis.cancer_risk",
            "4 or more",
            "Current H. pylori infection, with a higher risk of gastric cancer.",
        ),
        t if t >= 2.0 => ("kyoto_gastritis.current", "2–3", "Current H. pylori infection."),
        t if t >= 1.0 => (
            "kyoto_gastritis.indeterminate",
            "1",
            "Between the book's cut-offs: neither 0 (no infection) nor 2 or more (current infection).",
        ),
        _ => ("kyoto_gastritis.none", "0", "No H. pylori infection."),
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{total}"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(16),
        parts,
        notes: Vec::new(),
    })
}

/// Next steps and differential diagnosis for each band (Toyoshima 2020;
/// Maastricht VI/Florence 2022 for testing and eradication; MAPS II 2019 for
/// surveillance).
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "kyoto_gastritis.none",
        Guide {
            next: &[
                "Endoscopy shows no sign of H. pylori.",
                "Still suspected: urea breath test or stool antigen, off PPI for 2 weeks and antibiotics for 4.",
            ],
            dd: &["Stomach never infected", "Functional dyspepsia, if symptoms go on", "Reflux disease"],
        },
    ),
    (
        "kyoto_gastritis.indeterminate",
        Guide {
            next: &[
                "Settle it with a test: biopsy (rapid urease or histology), urea breath test or stool antigen.",
            ],
            dd: &[
                "Past infection, already eradicated (atrophy or metaplasia can stay)",
                "Current infection with few findings",
                "Autoimmune gastritis (atrophy of the corpus)",
            ],
        },
    ),
    (
        "kyoto_gastritis.current",
        Guide {
            next: &[
                "Confirm with biopsy (rapid urease test or histology).",
                "Eradicate, then confirm eradication by urea breath test or stool antigen 4 or more weeks after treatment.",
            ],
            dd: &[
                "Current H. pylori gastritis",
                "Past infection with atrophy left behind",
                "Autoimmune gastritis",
            ],
        },
    ),
    (
        "kyoto_gastritis.cancer_risk",
        Guide {
            next: &[
                "Eradicate and confirm eradication.",
                "Biopsies by the updated Sydney protocol to stage atrophy and metaplasia (OLGA/OLGIM).",
                "Extensive atrophy or metaplasia: endoscopic surveillance about every 3 years.",
            ],
            dd: &[
                "Atrophic gastritis with intestinal metaplasia",
                "Early gastric cancer: look hard for flat or depressed lesions",
                "Autoimmune gastritis",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn clean() -> Inputs {
        Inputs::new()
            .choice("atrophy", "none")
            .choice("intestinal_metaplasia", "none")
            .choice("diffuse_redness", "none")
    }

    #[test]
    fn the_books_cut_offs() {
        assert_eq!(compute(&clean()).unwrap().band, "kyoto_gastritis.none");
        let o = compute(&clean().flag("nodularity", true)).unwrap();
        assert_eq!((o.score, o.band), (Some(1.0), "kyoto_gastritis.indeterminate"));
        let o = compute(&clean().choice("diffuse_redness", "severe")).unwrap();
        assert_eq!((o.score, o.band), (Some(2.0), "kyoto_gastritis.current"));
        let o = compute(
            &clean()
                .choice("atrophy", "severe")
                .choice("intestinal_metaplasia", "mild")
                .flag("enlarged_folds", true),
        )
        .unwrap();
        assert_eq!((o.score, o.band), (Some(4.0), "kyoto_gastritis.cancer_risk"));
    }

    #[test]
    fn everything_scores_eight() {
        let i = Inputs::new()
            .choice("atrophy", "severe")
            .choice("intestinal_metaplasia", "severe")
            .flag("enlarged_folds", true)
            .flag("nodularity", true)
            .choice("diffuse_redness", "severe");
        assert_eq!(compute(&i).unwrap().score, Some(8.0));
    }
}
