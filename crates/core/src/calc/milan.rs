//! Milan criteria for liver transplantation in hepatocellular carcinoma.
//!
//! Source: the book, Lecture 18, p. 368: a single lesion ≤ 5 cm, or 2–3
//! lesions all ≤ 3 cm, without gross vascular invasion, without regional
//! nodal or distant metastasis, and performance status ≤ 1 (Mazzaferro et al.,
//! NEJM 1996, has no performance-status criterion; the book adds it).

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, flag, num};

pub const SPEC: CalcSpec = CalcSpec {
    id: "milan",
    name: "Milan criteria",
    area: Area::Tumour,
    fields: &[
        num("nodules", "Number of lesions", "", 1.0, 20.0, 1.0),
        num("largest_cm", "Largest lesion", "cm", 0.1, 30.0, 0.1),
        flag("vascular_invasion", "Gross vascular invasion"),
        flag("metastasis", "Nodal or distant metastasis"),
        num("performance_status", "ECOG performance status", "", 0.0, 4.0, 1.0),
    ],
    source: "Mazzaferro 1996; book Lecture 18, p. 368 (adds PS ≤ 1) and p. 373",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let nodules = i.get_num("nodules")?;
    let largest = i.get_pos("largest_cm")?;
    let invasion = i.get_flag("vascular_invasion")?;
    let spread = i.get_flag("metastasis")?;
    let ps = i.get_num("performance_status")?;
    if nodules < 1.0 {
        return Err(CalcError::Invalid { field: "nodules", reason: "must be at least 1" });
    }
    let size = (nodules <= 1.0 && largest <= 5.0) || (nodules <= 3.0 && largest <= 3.0);
    let parts = vec![
        Part::met("1 lesion ≤ 5 cm, or 2–3 lesions all ≤ 3 cm", size),
        Part::met("No gross vascular invasion", !invasion),
        Part::met("No nodal or distant metastasis", !spread),
        Part::met("Performance status ≤ 1", ps <= 1.0),
    ];
    let within = parts.iter().all(|p| p.met == Some(true));
    let (band, label, meaning) = if within {
        (
            "milan.within",
            "Within Milan criteria",
            "Liver transplantation is an option (with a high MELD score, or Child-Pugh C with PS ≤ 1, p. 373). \
             Bridge with TACE, TARE or RFA if the wait is over 3–6 months.",
        )
    } else {
        (
            "milan.outside",
            "Outside Milan criteria",
            "Not a standard transplant candidate. Downstaging to within Milan with AFP < 1000 ng/mL (OPTN protocol) \
             can make the patient eligible.",
        )
    };
    Ok(Outcome {
        score: None,
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(368),
        parts,
        notes: Vec::new(),
    })
}

pub const GUIDES: &[(&str, Guide)] = &[
    (
        "milan.within",
        Guide {
            next: &[
                "Refer for liver transplantation (MELD exception points for HCC).",
                "Bridge with TACE, TARE or RFA if the wait is over 3–6 months.",
                "With preserved liver function and no portal hypertension, resection or ablation first (BCLC 0–A).",
            ],
            dd: &[
                "Make sure the lesions are HCC (arterial enhancement with washout); otherwise intrahepatic \
                 cholangiocarcinoma, mixed HCC-cholangiocarcinoma, dysplastic nodule.",
            ],
        },
    ),
    (
        "milan.outside",
        Guide {
            next: &[
                "Try downstaging (TACE, TARE, ablation) to within Milan with AFP < 1000 ng/mL (OPTN).",
                "Otherwise treat by BCLC stage.",
            ],
            dd: &[
                "Tumour thrombus vs bland portal vein thrombosis: bland thrombus does not exclude transplant.",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn run(n: f64, cm: f64, invasion: bool, ps: f64) -> &'static str {
        compute(
            &Inputs::new()
                .num("nodules", n)
                .num("largest_cm", cm)
                .flag("vascular_invasion", invasion)
                .num("performance_status", ps),
        )
        .unwrap()
        .band
    }

    #[test]
    fn size_and_count() {
        assert_eq!(run(1.0, 5.0, false, 0.0), "milan.within");
        assert_eq!(run(1.0, 5.5, false, 0.0), "milan.outside");
        assert_eq!(run(3.0, 3.0, false, 1.0), "milan.within");
        assert_eq!(run(2.0, 3.5, false, 0.0), "milan.outside");
        assert_eq!(run(4.0, 2.0, false, 0.0), "milan.outside");
    }

    #[test]
    fn invasion_and_performance() {
        assert_eq!(run(1.0, 3.0, true, 0.0), "milan.outside");
        assert_eq!(run(1.0, 3.0, false, 2.0), "milan.outside");
    }
}
