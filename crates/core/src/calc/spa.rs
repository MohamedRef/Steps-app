//! Classification criteria for the spondyloarthritides: CASPAR (psoriatic
//! arthritis), ASAS axial SpA, ASAS peripheral SpA and the modified New York
//! criteria for ankylosing spondylitis.
//!
//! Sources: the Rheumatology book, Lecture 17 (pp. 287–288) and Lecture 20
//! (p. 334). Where the book leaves a line out, the published criteria fill it
//! and the module says so: the ASAS axial entry line (back pain 3 months or
//! longer, onset before 45; Rudwaleit 2009), and the ASAS peripheral rule that
//! the arthritis, enthesitis or dactylitis that brings the patient in does not
//! count again (Rudwaleit 2011).

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice, flag, num};

// CASPAR (Taylor 2006), as the book prints it on p. 334.

const SKIN: &[(&str, &str)] = &[
    ("current", "Psoriasis now (2)"),
    ("past", "Past history of psoriasis (1)"),
    ("family", "Family history only, patient not affected (1)"),
    ("none", "None of these (0)"),
];

pub const CASPAR: CalcSpec = CalcSpec {
    id: "caspar",
    name: "CASPAR (psoriatic arthritis)",
    area: Area::Rheum,
    fields: &[
        flag("inflammatory_disease", "Inflammatory articular disease (joint, spine or entheses)")
            .hint("The entry criterion: CASPAR is applied only to it."),
        choice("psoriasis", "Psoriasis", SKIN),
        flag("nail_lesions", "Nail lesions: onycholysis, pitting, hyperkeratosis"),
        flag("dactylitis", "Dactylitis, present or past (documented by a rheumatologist)"),
        flag("rf_negative", "Rheumatoid factor negative (any method except latex)"),
        flag("juxta_articular_bone", "Juxta-articular new bone formation on X-ray (not osteophytes)"),
    ],
    source: "Taylor et al., Arthritis Rheum 2006 (CASPAR); book Rheumatology Lecture 20, p. 334",
    compute: caspar,
};

fn caspar(i: &Inputs) -> Result<Outcome, CalcError> {
    let skin = match i.get_choice("psoriasis", &["current", "past", "family", "none"])? {
        "current" => 2.0,
        "past" | "family" => 1.0,
        _ => 0.0,
    };
    let point = |key: &'static str| i.get_flag(key).map(|b| if b { 1.0 } else { 0.0 });
    let parts = vec![
        Part::new("Psoriasis (now 2, past or family 1)", skin),
        Part::new("Nail dystrophy", point("nail_lesions")?),
        Part::new("Dactylitis", point("dactylitis")?),
        Part::new("RF negative", point("rf_negative")?),
        Part::new("Juxta-articular new bone", point("juxta_articular_bone")?),
    ];
    let total: f64 = parts.iter().map(|p| p.points).sum();
    let entry = i.get_flag("inflammatory_disease")?;
    let mut notes = vec!["Specificity 98.7%, sensitivity 91.4% (p. 334).".to_owned()];
    if !entry {
        notes.push(
            "CASPAR applies only to inflammatory articular disease (joint, spine or entheses); tick it to classify."
                .to_owned(),
        );
    }
    let (band, label, meaning) = if entry && total >= 3.0 {
        ("caspar.met", "3 or more", "Classified as psoriatic arthritis.")
    } else {
        ("caspar.not_met", "Not met", "Not classified as psoriatic arthritis.")
    };
    Ok(Outcome {
        score: Some(total),
        display: format!("{total} points"),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(334),
        parts,
        notes,
    })
}

// ASAS axial SpA (Rudwaleit 2009), book pp. 287–288.

/// The SpA features other than HLA-B27, with their labels.
const AXIAL_FEATURES: &[(&str, &str)] = &[
    ("ibp", "Inflammatory back pain"),
    ("arthritis", "Arthritis"),
    ("enthesitis", "Enthesitis (heel)"),
    ("uveitis", "Uveitis"),
    ("dactylitis", "Dactylitis"),
    ("psoriasis", "Psoriasis"),
    ("ibd", "Crohn's disease or ulcerative colitis"),
    ("nsaid_response", "Good response to NSAIDs"),
    ("family_history", "Family history of SpA"),
    ("elevated_crp", "Elevated CRP"),
];

pub const ASAS_AXIAL: CalcSpec = CalcSpec {
    id: "asas_axial_spa",
    name: "ASAS axial SpA criteria",
    area: Area::Rheum,
    fields: &[
        flag("back_pain_3_months_onset_45", "Back pain 3 months or longer, starting before age 45")
            .hint("The entry line (ASAS 2009); the book's p. 295 says onset before 40–45."),
        flag("sacroiliitis_imaging", "Sacroiliitis on imaging")
            .hint("Active inflammation on MRI, or X-ray grade 2 bilateral or grade 3–4 unilateral."),
        flag("hla_b27", "HLA-B27 positive"),
        flag("ibp", "Inflammatory back pain").hint(
            "Onset before 45, insidious, over 3 months, night pain, better with exercise, not with rest.",
        ),
        flag("arthritis", "Arthritis"),
        flag("enthesitis", "Enthesitis (heel)"),
        flag("uveitis", "Uveitis"),
        flag("dactylitis", "Dactylitis"),
        flag("psoriasis", "Psoriasis"),
        flag("ibd", "Crohn's disease or ulcerative colitis"),
        flag("nsaid_response", "Good response to NSAIDs"),
        flag("family_history", "Family history of SpA"),
        flag("elevated_crp", "Elevated CRP"),
    ],
    source: "Rudwaleit et al., Ann Rheum Dis 2009 (ASAS); book Rheumatology Lecture 17, pp. 287–288 (entry line from the paper)",
    compute: asas_axial,
};

fn count(i: &Inputs, keys: &[(&'static str, &str)]) -> Result<(usize, Vec<Part>), CalcError> {
    let mut n = 0;
    let mut parts = Vec::with_capacity(keys.len());
    for (key, label) in keys {
        let met = i.get_flag(key)?;
        n += usize::from(met);
        parts.push(Part::met(*label, met));
    }
    Ok((n, parts))
}

fn asas_axial(i: &Inputs) -> Result<Outcome, CalcError> {
    let entry = i.get_flag("back_pain_3_months_onset_45")?;
    let imaging = i.get_flag("sacroiliitis_imaging")?;
    let b27 = i.get_flag("hla_b27")?;
    let (features, feature_parts) = count(i, AXIAL_FEATURES)?;
    // In the imaging arm HLA-B27 is one of the SpA features.
    let imaging_arm = imaging && features + usize::from(b27) >= 1;
    let clinical_arm = b27 && features >= 2;
    let mut parts = vec![Part::met("Sacroiliitis on imaging", imaging), Part::met("HLA-B27", b27)];
    parts.extend(feature_parts);
    let mut notes = Vec::new();
    if !entry {
        notes.push(
            "The criteria apply to back pain of 3 months or more that started before age 45.".to_owned(),
        );
    }
    if entry && imaging_arm && clinical_arm {
        notes.push("Both arms are met.".to_owned());
    }
    let (band, label, meaning) = match (entry, imaging_arm, clinical_arm) {
        (true, true, _) => (
            "asas_axial_spa.imaging",
            "Imaging arm met",
            "Axial SpA: sacroiliitis on imaging plus 1 or more SpA feature.",
        ),
        (true, false, true) => (
            "asas_axial_spa.clinical",
            "Clinical arm met",
            "Axial SpA: HLA-B27 plus 2 or more other SpA features.",
        ),
        _ => ("asas_axial_spa.not_met", "Not met", "Not classified as axial SpA."),
    };
    Ok(Outcome {
        score: None,
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(288),
        parts,
        notes,
    })
}

// ASAS peripheral SpA (Rudwaleit 2011), book p. 288.

const PERIPHERAL_ENTRY: &[(&str, &str)] =
    &[("arthritis", "Arthritis"), ("enthesitis", "Enthesitis"), ("dactylitis", "Dactylitis")];

const PERIPHERAL_A: &[(&str, &str)] = &[
    ("psoriasis", "Psoriasis"),
    ("ibd", "Inflammatory bowel disease"),
    ("preceding_infection", "Preceding infection"),
    ("hla_b27", "HLA-B27"),
    ("uveitis", "Uveitis"),
    ("sacroiliitis_imaging", "Sacroiliitis on imaging"),
];

const PERIPHERAL_B: &[(&str, &str)] =
    &[("ibp_past", "Inflammatory back pain in the past"), ("family_history", "Family history of SpA")];

pub const ASAS_PERIPHERAL: CalcSpec = CalcSpec {
    id: "asas_peripheral_spa",
    name: "ASAS peripheral SpA criteria",
    area: Area::Rheum,
    fields: &[
        flag("arthritis", "Arthritis"),
        flag("enthesitis", "Enthesitis"),
        flag("dactylitis", "Dactylitis"),
        flag("psoriasis", "Psoriasis"),
        flag("ibd", "Inflammatory bowel disease"),
        flag("preceding_infection", "Preceding infection")
            .hint("Urethritis, cervicitis or diarrhoea before the arthritis."),
        flag("hla_b27", "HLA-B27 positive"),
        flag("uveitis", "Uveitis"),
        flag("sacroiliitis_imaging", "Sacroiliitis on imaging (X-ray or MRI)"),
        flag("ibp_past", "Inflammatory back pain in the past"),
        flag("family_history", "Family history of SpA"),
    ],
    source: "Rudwaleit et al., Ann Rheum Dis 2011 (ASAS); book Rheumatology Lecture 17, p. 288",
    compute: asas_peripheral,
};

fn asas_peripheral(i: &Inputs) -> Result<Outcome, CalcError> {
    let (peripheral, mut parts) = count(i, PERIPHERAL_ENTRY)?;
    let (a, a_parts) = count(i, PERIPHERAL_A)?;
    let (b_more, b_parts) = count(i, PERIPHERAL_B)?;
    parts.extend(a_parts);
    parts.extend(b_parts);
    // The feature that brings the patient in does not count again.
    let b = peripheral.saturating_sub(1) + b_more;
    let entry = peripheral >= 1;
    let mut notes = vec!["For peripheral symptoms only; with back pain use the axial criteria.".to_owned()];
    if !entry {
        notes.push("Entry needs arthritis, enthesitis or dactylitis.".to_owned());
    }
    let (band, label, meaning) = match (entry, a >= 1, b >= 2) {
        (true, true, _) => (
            "asas_peripheral_spa.met",
            "Met",
            "Peripheral SpA: arthritis, enthesitis or dactylitis plus 1 or more of the first list.",
        ),
        (true, false, true) => (
            "asas_peripheral_spa.met",
            "Met",
            "Peripheral SpA: arthritis, enthesitis or dactylitis plus 2 or more of the remaining features.",
        ),
        _ => ("asas_peripheral_spa.not_met", "Not met", "Not classified as peripheral SpA."),
    };
    Ok(Outcome {
        score: None,
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(288),
        parts,
        notes,
    })
}

// Modified New York criteria 1984, book p. 287 (sacroiliitis grades p. 286).

pub const MODIFIED_NEW_YORK: CalcSpec = CalcSpec {
    id: "modified_new_york",
    name: "Modified New York criteria (AS)",
    area: Area::Rheum,
    fields: &[
        flag(
            "inflammatory_back_pain",
            "Low back pain and stiffness 3 months or more, better with exercise, not rest",
        ),
        flag("lumbar_limitation", "Limited lumbar motion in sagittal and frontal planes"),
        flag("chest_expansion", "Limited chest expansion (for age and sex)"),
        num("si_grade_right", "Sacroiliitis grade, right", "", 0.0, 4.0, 1.0)
            .hint("X-ray: 0 normal, 1 suspicious, 2 minimal, 3 moderate, 4 ankylosis."),
        num("si_grade_left", "Sacroiliitis grade, left", "", 0.0, 4.0, 1.0),
    ],
    source: "van der Linden et al., Arthritis Rheum 1984; book Rheumatology Lecture 17, pp. 286–288",
    compute: modified_new_york,
};

fn grade(i: &Inputs, key: &'static str) -> Result<f64, CalcError> {
    let v = i.get_num(key)?;
    if !(0.0..=4.0).contains(&v) || v.fract() != 0.0 {
        return Err(CalcError::Invalid { field: key, reason: "must be a grade from 0 to 4" });
    }
    Ok(v)
}

fn modified_new_york(i: &Inputs) -> Result<Outcome, CalcError> {
    let (right, left) = (grade(i, "si_grade_right")?, grade(i, "si_grade_left")?);
    let radiological = right.min(left) >= 2.0 || right.max(left) >= 3.0;
    let clinical = [
        Part::met("Inflammatory low back pain ≥ 3 months", i.get_flag("inflammatory_back_pain")?),
        Part::met("Limited lumbar motion", i.get_flag("lumbar_limitation")?),
        Part::met("Limited chest expansion", i.get_flag("chest_expansion")?),
    ];
    let clinical_met = clinical.iter().any(|p| p.points > 0.0);
    let mut parts = vec![Part::met("Sacroiliitis: grade ≥ 2 bilateral or 3–4 unilateral", radiological)];
    parts.extend(clinical);
    let mut notes = Vec::new();
    if !radiological && right.min(left) >= 1.0 {
        notes.push(
            "p. 287 prints \"grade 1–2 bilaterally\"; p. 288 and the 1984 criteria need grade 2 or more on both sides."
                .to_owned(),
        );
    }
    let (band, label, meaning) = if radiological && clinical_met {
        (
            "modified_new_york.met",
            "Definite AS",
            "Definite ankylosing spondylitis: the radiological criterion plus 1 or more clinical criterion.",
        )
    } else if radiological {
        (
            "modified_new_york.not_met",
            "Not met",
            "Radiological criterion only: needs at least one clinical criterion.",
        )
    } else {
        (
            "modified_new_york.not_met",
            "Not met",
            "No radiographic sacroiliitis: consider ASAS axial SpA (non-radiographic).",
        )
    };
    Ok(Outcome {
        score: None,
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(287),
        parts,
        notes,
    })
}

/// Next step and differential diagnosis for each result.
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "caspar.met",
        Guide {
            next: &[
                "Assess the domains: peripheral arthritis, axial disease, enthesitis, dactylitis, skin and nails.",
                "X-ray hands and feet (erosions with new bone, pencil-in-cup); screen for metabolic syndrome.",
                "Treat by domain: NSAIDs and local steroid, then csDMARD (methotrexate) for peripheral arthritis; biologic (TNF, IL-17 or IL-23) if active despite it or with axial disease.",
            ],
            dd: &[
                "Rheumatoid arthritis with coincident psoriasis (symmetrical, RF/ACPA high, no DIP or new bone)",
                "Gout (DIP and toe joints, urate crystals on aspiration)",
                "Erosive hand osteoarthritis (DIP, Heberden nodes, no dactylitis)",
                "Reactive arthritis and IBD-associated arthritis",
            ],
        },
    ),
    (
        "caspar.not_met",
        Guide {
            next: &[
                "Re-examine skin (scalp, umbilicus, natal cleft), nails and family history: hidden psoriasis is common.",
                "Check RF and ACPA, uric acid and X-rays; re-apply CASPAR as features evolve.",
            ],
            dd: &[
                "Rheumatoid arthritis",
                "Other spondyloarthritis (reactive, IBD-associated, axial SpA)",
                "Gout or CPPD",
                "Osteoarthritis",
            ],
        },
    ),
    (
        "asas_axial_spa.imaging",
        Guide {
            next: &[
                "Radiographic sacroiliitis (modified New York) = ankylosing spondylitis; MRI only = non-radiographic axial SpA.",
                "Measure activity (ASDAS or BASDAI) and CRP; look for uveitis, IBD and psoriasis.",
                "First line: exercise and physiotherapy plus full-dose NSAIDs; TNF or IL-17 inhibitor if still active after 2 NSAIDs over 4 weeks.",
            ],
            dd: &[
                "Mechanical back pain and degenerative disc disease",
                "Diffuse idiopathic skeletal hyperostosis (DISH: older, flowing anterior ossification, normal SI joints)",
                "Osteitis condensans ilii (iliac sclerosis after pregnancy, no erosions)",
                "Septic or tuberculous sacroiliitis (unilateral, fever, high CRP)",
                "Brucellosis sacroiliitis in endemic areas",
            ],
        },
    ),
    (
        "asas_axial_spa.clinical",
        Guide {
            next: &[
                "Get an MRI of the sacroiliac joints (STIR) if not done: it may move the patient into the imaging arm.",
                "Measure activity (ASDAS or BASDAI) and CRP before treatment.",
                "Exercise plus NSAIDs first; biologic only with objective inflammation (high CRP or MRI) and persistent activity.",
            ],
            dd: &[
                "Mechanical back pain in an HLA-B27 carrier (B27 is in about 8% of healthy people)",
                "Fibromyalgia (widespread pain, normal CRP and MRI)",
                "DISH",
                "Osteitis condensans ilii",
            ],
        },
    ),
    (
        "asas_axial_spa.not_met",
        Guide {
            next: &[
                "If back pain is inflammatory, check HLA-B27, CRP and MRI of the SI joints, then re-apply.",
                "Look for red flags (fever, weight loss, night pain at rest, neurological signs) that point elsewhere.",
            ],
            dd: &[
                "Mechanical or degenerative back pain",
                "Disc prolapse or spinal stenosis",
                "Infection (TB, brucella) or malignancy (myeloma, metastases)",
                "Fibromyalgia",
            ],
        },
    ),
    (
        "asas_peripheral_spa.met",
        Guide {
            next: &[
                "Name the subtype: psoriatic, reactive (recent urethritis or diarrhoea), IBD-associated, or undifferentiated.",
                "Ask for the trigger infection (chlamydia NAAT, stool culture), examine the eyes, skin and gut.",
                "NSAIDs and local steroid injection; sulfasalazine or methotrexate for persistent peripheral arthritis; biologic if refractory.",
            ],
            dd: &[
                "Septic arthritis (aspirate a single hot joint first)",
                "Gout or CPPD (crystals on aspiration)",
                "Rheumatoid arthritis (symmetric small joints, RF/ACPA)",
                "Disseminated gonococcal infection (tenosynovitis, pustules)",
            ],
        },
    ),
    (
        "asas_peripheral_spa.not_met",
        Guide {
            next: &[
                "Aspirate an effusion (cell count, crystals, Gram stain, culture).",
                "Check HLA-B27, CRP, RF/ACPA and imaging of the SI joints; re-apply as features appear.",
            ],
            dd: &[
                "Septic arthritis",
                "Crystal arthritis",
                "Rheumatoid arthritis",
                "Viral arthritis (parvovirus B19, hepatitis B/C, chikungunya)",
            ],
        },
    ),
    (
        "modified_new_york.met",
        Guide {
            next: &[
                "Ankylosing spondylitis (radiographic axial SpA): measure activity (ASDAS/BASDAI), function (BASFI) and spinal mobility (BASMI).",
                "Screen for uveitis, aortic regurgitation, IBD, psoriasis and osteoporosis (spine fractures after minor trauma).",
                "Exercise and NSAIDs; TNF or IL-17 inhibitor if active after 2 NSAIDs; no role for systemic steroids or csDMARDs in axial disease.",
            ],
            dd: &[
                "DISH (right-sided flowing ossification, SI joints spared)",
                "Osteitis condensans ilii",
                "Degenerative SI joint disease in the elderly",
                "Infective sacroiliitis (TB, brucella)",
            ],
        },
    ),
    (
        "modified_new_york.not_met",
        Guide {
            next: &[
                "No radiographic sacroiliitis does not exclude axial SpA: apply the ASAS axial criteria and get an MRI.",
                "Radiographic sacroiliitis with no clinical criterion: re-examine mobility and chest expansion.",
            ],
            dd: &["Non-radiographic axial SpA", "Mechanical back pain", "DISH", "Osteitis condensans ilii"],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caspar_needs_three_points_and_the_entry() {
        let i = Inputs::new()
            .flag("inflammatory_disease", true)
            .choice("psoriasis", "current")
            .flag("rf_negative", true);
        let o = caspar(&i).unwrap();
        assert_eq!((o.score, o.band), (Some(3.0), "caspar.met"));
        let o = caspar(&i.clone().flag("inflammatory_disease", false)).unwrap();
        assert_eq!(o.band, "caspar.not_met");
        let past = Inputs::new()
            .flag("inflammatory_disease", true)
            .choice("psoriasis", "family")
            .flag("nail_lesions", true);
        assert_eq!(caspar(&past).unwrap().band, "caspar.not_met");
        let all = Inputs::new()
            .flag("inflammatory_disease", true)
            .choice("psoriasis", "current")
            .flag("nail_lesions", true)
            .flag("dactylitis", true)
            .flag("rf_negative", true)
            .flag("juxta_articular_bone", true);
        assert_eq!(caspar(&all).unwrap().score, Some(6.0));
    }

    #[test]
    fn asas_axial_arms() {
        let entry = Inputs::new().flag("back_pain_3_months_onset_45", true);
        // Imaging + HLA-B27 alone: B27 counts as the one feature.
        let o = asas_axial(&entry.clone().flag("sacroiliitis_imaging", true).flag("hla_b27", true)).unwrap();
        assert_eq!(o.band, "asas_axial_spa.imaging");
        // HLA-B27 + IBP + uveitis: the clinical arm.
        let o =
            asas_axial(&entry.clone().flag("hla_b27", true).flag("ibp", true).flag("uveitis", true)).unwrap();
        assert_eq!(o.band, "asas_axial_spa.clinical");
        // HLA-B27 + one feature is not enough.
        let o = asas_axial(&entry.clone().flag("hla_b27", true).flag("ibp", true)).unwrap();
        assert_eq!(o.band, "asas_axial_spa.not_met");
        // Without the entry line nothing is classified.
        let o = asas_axial(&Inputs::new().flag("sacroiliitis_imaging", true).flag("ibp", true)).unwrap();
        assert_eq!(o.band, "asas_axial_spa.not_met");
    }

    #[test]
    fn asas_peripheral_arms() {
        let o = asas_peripheral(&Inputs::new().flag("arthritis", true).flag("uveitis", true)).unwrap();
        assert_eq!(o.band, "asas_peripheral_spa.met");
        // Arthritis + enthesitis + family history: two of the remaining.
        let o = asas_peripheral(
            &Inputs::new().flag("arthritis", true).flag("enthesitis", true).flag("family_history", true),
        )
        .unwrap();
        assert_eq!(o.band, "asas_peripheral_spa.met");
        // Arthritis + family history: only one of the remaining.
        let o = asas_peripheral(&Inputs::new().flag("arthritis", true).flag("family_history", true)).unwrap();
        assert_eq!(o.band, "asas_peripheral_spa.not_met");
        // No entry feature.
        let o = asas_peripheral(&Inputs::new().flag("hla_b27", true).flag("uveitis", true)).unwrap();
        assert_eq!(o.band, "asas_peripheral_spa.not_met");
    }

    #[test]
    fn modified_new_york_grades() {
        let with = |r: f64, l: f64| {
            Inputs::new().num("si_grade_right", r).num("si_grade_left", l).flag("chest_expansion", true)
        };
        assert_eq!(modified_new_york(&with(2.0, 2.0)).unwrap().band, "modified_new_york.met");
        assert_eq!(modified_new_york(&with(3.0, 0.0)).unwrap().band, "modified_new_york.met");
        assert_eq!(modified_new_york(&with(2.0, 1.0)).unwrap().band, "modified_new_york.not_met");
        let o = modified_new_york(&with(1.0, 1.0)).unwrap();
        assert_eq!(o.band, "modified_new_york.not_met");
        assert_eq!(o.notes.len(), 1);
        let no_clinical = Inputs::new().num("si_grade_right", 4.0).num("si_grade_left", 4.0);
        assert_eq!(modified_new_york(&no_clinical).unwrap().band, "modified_new_york.not_met");
        assert!(modified_new_york(&with(2.5, 2.0)).is_err());
    }
}
