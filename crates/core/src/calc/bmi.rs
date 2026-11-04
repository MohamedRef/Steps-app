//! Body mass index, with what the book's ADA chapter does at each level.
//!
//! BMI = weight (kg) ÷ height (m)². The book (Diabetology Lecture 2,
//! pp. 18–22) uses these cut-offs: ≥ 25 overweight, screen for diabetes with
//! one more risk factor (p. 18); ≥ 27 obesity drugs may be added in type 2
//! diabetes (p. 22); metabolic surgery may be considered at 30.0–34.9,
//! recommended at 35.0–39.9 without durable weight loss, and at ≥ 40 (p. 22).
//! The class names (underweight < 18.5, obesity class I–III) are the WHO's;
//! the book does not print them.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, num, show};

pub const SPEC: CalcSpec = CalcSpec {
    id: "bmi",
    name: "Body mass index (BMI)",
    area: Area::Diabetes,
    fields: &[
        num("weight_kg", "Weight", "kg", 20.0, 300.0, 0.1),
        num("height_cm", "Height", "cm", 100.0, 230.0, 0.5),
    ],
    source: "Book Diabetology Lecture 2, pp. 18–22 (ADA); class names WHO",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let weight = i.get_pos("weight_kg")?;
    let m = i.get_pos("height_cm")? / 100.0;
    let bmi = ((weight / (m * m)) * 10.0).round() / 10.0;
    let (band, label, meaning) = match bmi {
        b if b >= 40.0 => (
            "bmi.class3",
            "Obesity class III (40 or more)",
            "Metabolic surgery is a recommended option for type 2 diabetes at this BMI.",
        ),
        b if b >= 35.0 => (
            "bmi.class2",
            "Obesity class II (35–39.9)",
            "Metabolic surgery is recommended in type 2 diabetes if weight loss and comorbidities \
             don't improve durably without it.",
        ),
        b if b >= 30.0 => (
            "bmi.class1",
            "Obesity class I (30–34.9)",
            "Metabolic surgery may be considered in type 2 diabetes if non-surgical weight loss \
             isn't durable.",
        ),
        b if b >= 25.0 => (
            "bmi.overweight",
            "Overweight (25–29.9)",
            "Screen for prediabetes and type 2 diabetes if there is one more risk factor. \
             From 27, obesity drugs can be added in type 2 diabetes.",
        ),
        b if b >= 18.5 => ("bmi.normal", "Normal (18.5–24.9)", "Normal weight."),
        _ => ("bmi.under", "Underweight (below 18.5)", "Below the normal range."),
    };
    let mut notes = Vec::new();
    if bmi >= 27.0 {
        notes.push("27 or more: obesity pharmacotherapy can be added in type 2 diabetes (p. 22).".to_owned());
    }
    Ok(Outcome {
        score: Some(bmi),
        display: format!("{} kg/m²", show(bmi, 1)),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(22),
        parts: vec![
            Part::new("Weight (kg)", weight),
            Part::new("÷ height² (m²)", (m * m * 100.0).round() / 100.0),
        ],
        notes,
    })
}

/// Next steps and differential diagnosis for each band (ADA Standards of
/// Care 2025 §2 and §8; WHO 2000 classes; book pp. 18–22).
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "bmi.under",
        Guide {
            next: &["Find the cause; nutrition assessment."],
            dd: &[
                "Malnutrition",
                "Hyperthyroidism",
                "Uncontrolled diabetes (type 1 or LADA)",
                "Malabsorption: coeliac disease, IBD",
                "Cancer",
                "TB or HIV",
                "Eating disorder or depression",
            ],
        },
    ),
    (
        "bmi.normal",
        Guide {
            next: &[
                "Diabetes screening still starts at 35 years for everyone.",
                "Asian ancestry: screening starts at BMI 23, not 25.",
            ],
            dd: &[],
        },
    ),
    (
        "bmi.overweight",
        Guide {
            next: &[
                "Screen for prediabetes and diabetes if there is one more risk factor.",
                "Lifestyle: aim for 5–7% weight loss or more.",
                "27 or more with type 2 diabetes: consider a GLP-1 receptor agonist or tirzepatide.",
            ],
            dd: &[
                "Drugs: insulin, sulfonylureas, pioglitazone, steroids, antipsychotics",
                "Hypothyroidism",
                "Cushing's syndrome",
                "Fluid, not fat: oedema, ascites",
                "High muscle mass (BMI overestimates fat)",
            ],
        },
    ),
    (
        "bmi.class1",
        Guide {
            next: &[
                "Lifestyle plus obesity drugs (GLP-1 receptor agonist or tirzepatide).",
                "Type 2 diabetes not controlled despite this: metabolic surgery may be considered.",
                "Check for complications: hypertension, dyslipidaemia, MASLD, sleep apnoea, osteoarthritis.",
            ],
            dd: &[
                "Drugs: insulin, sulfonylureas, pioglitazone, steroids, antipsychotics",
                "Hypothyroidism",
                "Cushing's syndrome",
                "Fluid, not fat: oedema, ascites",
                "High muscle mass (BMI overestimates fat)",
            ],
        },
    ),
    (
        "bmi.class2",
        Guide {
            next: &[
                "Lifestyle plus obesity drugs.",
                "Type 2 diabetes without durable weight loss: metabolic surgery is recommended.",
                "Check for complications: hypertension, dyslipidaemia, MASLD, sleep apnoea, osteoarthritis.",
            ],
            dd: &[
                "Drugs: insulin, sulfonylureas, pioglitazone, steroids, antipsychotics",
                "Hypothyroidism",
                "Cushing's syndrome",
                "Fluid, not fat: oedema, ascites",
                "High muscle mass (BMI overestimates fat)",
            ],
        },
    ),
    (
        "bmi.class3",
        Guide {
            next: &[
                "Metabolic surgery is a recommended option in type 2 diabetes.",
                "Lifestyle plus obesity drugs.",
                "Check for complications: hypertension, dyslipidaemia, MASLD, sleep apnoea, osteoarthritis.",
            ],
            dd: &[
                "Drugs: insulin, sulfonylureas, pioglitazone, steroids, antipsychotics",
                "Hypothyroidism",
                "Cushing's syndrome",
                "Fluid, not fat: oedema, ascites",
                "High muscle mass (BMI overestimates fat)",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn bmi(w: f64, h: f64) -> Outcome {
        compute(&Inputs::new().num("weight_kg", w).num("height_cm", h)).unwrap()
    }

    #[test]
    fn the_formula_and_the_cut_offs() {
        let o = bmi(70.0, 175.0);
        assert_eq!((o.score, o.band), (Some(22.9), "bmi.normal"));
        assert_eq!(bmi(80.0, 175.0).band, "bmi.overweight");
        assert_eq!(bmi(95.0, 175.0).band, "bmi.class1");
        assert_eq!(bmi(110.0, 175.0).band, "bmi.class2");
        assert_eq!(bmi(125.0, 175.0).band, "bmi.class3");
        assert_eq!(bmi(50.0, 175.0).band, "bmi.under");
    }

    #[test]
    fn twenty_seven_adds_the_drug_note() {
        assert!(bmi(80.0, 175.0).notes.is_empty()); // 26.1
        assert_eq!(bmi(85.0, 175.0).notes.len(), 1); // 27.8
    }
}
