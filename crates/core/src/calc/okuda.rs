//! Okuda staging of hepatocellular carcinoma.
//!
//! The book names the Okuda system (Lecture 18, p. 361) and uses it (an
//! advanced Okuda stage is a risk factor for spontaneous rupture, p. 355) but
//! does not print it. Criteria from Okuda et al., Cancer 1985: tumour > 50% of
//! the liver, ascites, albumin < 3 g/dL, bilirubin > 3 mg/dL; none = stage I,
//! 1–2 = stage II, 3–4 = stage III.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, flag, num};

pub const SPEC: CalcSpec = CalcSpec {
    id: "okuda",
    name: "Okuda stage",
    area: Area::Tumour,
    fields: &[
        flag("tumour_over_half", "Tumour > 50% of the liver"),
        flag("ascites", "Ascites"),
        num("albumin_g_dl", "Albumin", "g/dL", 0.5, 7.0, 0.1),
        num("bilirubin_mg_dl", "Bilirubin", "mg/dL", 0.1, 60.0, 0.1),
    ],
    source: "Okuda 1985 (the book names it, Lecture 18, p. 361 and 355, without the criteria)",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let parts = vec![
        Part::met("Tumour > 50% of the liver", i.get_flag("tumour_over_half")?),
        Part::met("Ascites", i.get_flag("ascites")?),
        Part::met("Albumin < 3 g/dL", i.get_pos("albumin_g_dl")? < 3.0),
        Part::met("Bilirubin > 3 mg/dL", i.get_pos("bilirubin_mg_dl")? > 3.0),
    ];
    let score: f64 = parts.iter().map(|p| p.points).sum();
    let (band, label, meaning) = if score == 0.0 {
        ("okuda.1", "Stage I (no criteria)", "Least advanced: small tumour and compensated liver.")
    } else if score <= 2.0 {
        ("okuda.2", "Stage II (1–2 criteria)", "Moderately advanced.")
    } else {
        (
            "okuda.3",
            "Stage III (3–4 criteria)",
            "Most advanced: poor prognosis. An advanced Okuda stage is a risk factor for spontaneous rupture (p. 355).",
        )
    };
    Ok(Outcome {
        score: Some(score),
        display: label.split(' ').take(2).collect::<Vec<_>>().join(" "),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(361),
        parts,
        notes: Vec::new(),
    })
}

pub const GUIDES: &[(&str, Guide)] = &[
    (
        "okuda.1",
        Guide { next: &["Choose treatment by BCLC stage (the book's staging for therapy)."], dd: &[] },
    ),
    (
        "okuda.2",
        Guide {
            next: &["Choose treatment by BCLC stage; check Child-Pugh and performance status."],
            dd: &[],
        },
    ),
    (
        "okuda.3",
        Guide {
            next: &[
                "Usually BCLC D: best supportive care.",
                "Watch for spontaneous rupture: sudden pain and shock (p. 355).",
            ],
            dd: &[
                "Sudden abdominal pain with shock: HCC rupture, variceal bleed, SBP, portal vein thrombosis.",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages() {
        let base = Inputs::new().num("albumin_g_dl", 3.5).num("bilirubin_mg_dl", 1.0);
        let o = compute(&base).unwrap();
        assert_eq!((o.band, o.display.as_str()), ("okuda.1", "Stage I"));
        assert_eq!(compute(&base.clone().flag("ascites", true)).unwrap().band, "okuda.2");
        let o = compute(
            &Inputs::new()
                .flag("tumour_over_half", true)
                .flag("ascites", true)
                .num("albumin_g_dl", 2.8)
                .num("bilirubin_mg_dl", 2.0),
        )
        .unwrap();
        assert_eq!((o.score, o.band), (Some(3.0), "okuda.3"));
        // Exactly 3 is not "below 3" or "above 3".
        let o = compute(&Inputs::new().num("albumin_g_dl", 3.0).num("bilirubin_mg_dl", 3.0)).unwrap();
        assert_eq!(o.band, "okuda.1");
    }
}
