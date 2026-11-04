//! PAGE-B (Platelets, Age, GEnder, hepatitis B): HCC risk in Caucasian
//! patients with chronic HBV on antivirals, used to decide who without
//! cirrhosis needs HCC surveillance.
//!
//! Source: the book, Lecture 18, p. 351–352: age 16–29 = 0, 30–39 = 2,
//! 40–49 = 4, 50–59 = 6, 60–69 = 8, ≥ 70 = 10; male = 6; bands ≤ 9 low,
//! 10–17 intermediate, ≥ 18 high. The book prints the platelet points as 1
//! (100–199) and 2 (< 100). The original score (Papatheodoridis et al.,
//! J Hepatol 2016) gives 6 and 9, and the book's own cut-offs only work with
//! those (with 1 and 2 points the maximum is 18), so this uses 6 and 9 and
//! says so in a note.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice, num};

const SEX: &[(&str, &str)] = &[("male", "Male"), ("female", "Female")];

pub const SPEC: CalcSpec = CalcSpec {
    id: "page_b",
    name: "PAGE-B",
    area: Area::Tumour,
    fields: &[
        num("age_years", "Age", "years", 16.0, 110.0, 1.0),
        choice("sex", "Sex", SEX),
        num("platelets_10e9_l", "Platelets", "×10⁹/L", 1.0, 1500.0, 1.0),
    ],
    source: "Papatheodoridis 2016; book Lecture 18, p. 351–352 (platelet points from the paper, see note)",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let age = i.get_num("age_years")?;
    let male = i.get_choice("sex", &["male", "female"])? == "male";
    let plt = i.get_pos("platelets_10e9_l")?;
    let age_pts = match age {
        a if a < 30.0 => 0.0,
        a if a < 40.0 => 2.0,
        a if a < 50.0 => 4.0,
        a if a < 60.0 => 6.0,
        a if a < 70.0 => 8.0,
        _ => 10.0,
    };
    let sex_pts = if male { 6.0 } else { 0.0 };
    let (plt_pts, book_plt_pts) = if plt >= 200.0 {
        (0.0, 0.0)
    } else if plt >= 100.0 {
        (6.0, 1.0)
    } else {
        (9.0, 2.0)
    };
    let score = age_pts + sex_pts + plt_pts;
    let (band, label, meaning) = if score <= 9.0 {
        (
            "page_b.low",
            "Low risk (≤ 9)",
            "Almost 0% HCC at 5 years: surveillance not needed without cirrhosis.",
        )
    } else if score <= 17.0 {
        ("page_b.mid", "Intermediate risk (10–17)", "About 3% HCC at 5 years: screen for HCC.")
    } else {
        ("page_b.high", "High risk (≥ 18)", "About 17% HCC at 5 years: screen for HCC.")
    };
    let mut notes = vec![format!(
        "The book prints 1 and 2 points for platelets 100–199 and < 100. The original PAGE-B gives 6 and 9, which \
         the book's cut-offs need; with the book's points this patient has {}.",
        age_pts + sex_pts + book_plt_pts
    )];
    if age < 16.0 {
        notes.push("PAGE-B was built for adults from 16 years.".to_owned());
    }
    notes.push("Not yet validated in Asian patients (book p. 352).".to_owned());
    Ok(Outcome {
        score: Some(score),
        display: format!("{score}"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(352),
        parts: vec![
            Part::new("Age", age_pts),
            Part::new("Male sex", sex_pts),
            Part::new("Platelets", plt_pts),
        ],
        notes,
    })
}

pub const GUIDES: &[(&str, Guide)] = &[
    (
        "page_b.low",
        Guide {
            next: &[
                "No HCC surveillance needed while there is no cirrhosis; keep HBV treatment going.",
                "Recalculate if platelets fall or cirrhosis appears; any cirrhosis needs 6-monthly ultrasound.",
            ],
            dd: &[
                "A falling platelet count can be silent cirrhosis or portal hypertension: check elastography.",
            ],
        },
    ),
    (
        "page_b.mid",
        Guide {
            next: &["HCC surveillance: ultrasound every 6 months, with or without AFP (p. 351)."],
            dd: &[
                "A new liver lesion: HCC, regenerative or dysplastic nodule, haemangioma, focal nodular hyperplasia, \
                 metastasis, cholangiocarcinoma.",
            ],
        },
    ),
    (
        "page_b.high",
        Guide {
            next: &[
                "HCC surveillance: ultrasound every 6 months, with or without AFP (p. 351).",
                "A nodule ≥ 1 cm: multiphase CT or MRI.",
            ],
            dd: &[
                "A new liver lesion: HCC, regenerative or dysplastic nodule, haemangioma, focal nodular hyperplasia, \
                 metastasis, cholangiocarcinoma.",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn run(age: f64, sex: &str, plt: f64) -> Outcome {
        compute(&Inputs::new().num("age_years", age).choice("sex", sex).num("platelets_10e9_l", plt)).unwrap()
    }

    #[test]
    fn bands() {
        // 45-year-old woman, platelets 250: 4.
        let o = run(45.0, "female", 250.0);
        assert_eq!(o.score, Some(4.0));
        assert_eq!(o.band, "page_b.low");
        // 52-year-old man, platelets 150: 6 + 6 + 6 = 18.
        let o = run(52.0, "male", 150.0);
        assert_eq!(o.score, Some(18.0));
        assert_eq!(o.band, "page_b.high");
        // 35-year-old man, platelets 220: 2 + 6 = 8; at 40: 10.
        assert_eq!(run(35.0, "male", 220.0).band, "page_b.low");
        assert_eq!(run(40.0, "male", 220.0).band, "page_b.mid");
        // 72-year-old woman, platelets 90: 10 + 9 = 19.
        assert_eq!(run(72.0, "female", 90.0).score, Some(19.0));
    }

    #[test]
    fn note_gives_the_book_points_too() {
        let o = run(52.0, "male", 150.0);
        assert!(o.notes[0].contains("this patient has 13"), "{:?}", o.notes);
    }
}
