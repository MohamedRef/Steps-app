//! Rai and Binet staging of chronic lymphocytic leukaemia.
//!
//! Source: the Hematology book (Chapter 4, Lecture 8, Table 1, p. 129).
//! Rai (1975): every stage has lymphocytosis; 0 nothing else, I with
//! lymphadenopathy, II with hepatomegaly or splenomegaly, III with Hb
//! under 11 g/dL, IV with platelets under 100 × 10³/µL. Binet (1981): A
//! fewer than 3 lymphoid areas, B 3 or more, both with Hb 10 g/dL or more
//! and platelets 100 or more; C Hb under 10 or platelets under 100,
//! whatever the areas. The five Binet areas (cervical, axillary and
//! inguinal nodes, spleen, liver) are the original definition (Binet 1981);
//! the book's table only says "< 3 lymphatic groups".

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, flag, num};

pub const RAI: CalcSpec = CalcSpec {
    id: "rai_cll",
    name: "Rai stage (CLL)",
    area: Area::Blood,
    fields: &[
        flag("lymphadenopathy", "Lymphadenopathy"),
        flag("organomegaly", "Hepatomegaly or splenomegaly"),
        num("hb_g_dl", "Haemoglobin", "g/dL", 2.0, 25.0, 0.1),
        num("platelets_10e9_l", "Platelets", "× 10³/µL", 1.0, 2000.0, 1.0),
    ],
    source: "Rai 1975, modified Rai 1987, iwCLL 2018; book Hematology Lecture 8, Table 1, p. 129",
    compute: rai,
};

pub const BINET: CalcSpec = CalcSpec {
    id: "binet_cll",
    name: "Binet stage (CLL)",
    area: Area::Blood,
    fields: &[
        num("areas", "Enlarged lymphoid areas", "of 5", 0.0, 5.0, 1.0)
            .hint("Cervical, axillary and inguinal nodes (each side counts once), spleen, liver."),
        num("hb_g_dl", "Haemoglobin", "g/dL", 2.0, 25.0, 0.1),
        num("platelets_10e9_l", "Platelets", "× 10³/µL", 1.0, 2000.0, 1.0),
    ],
    source: "Binet 1981, iwCLL 2018; book Hematology Lecture 8, Table 1, p. 129",
    compute: binet,
};

fn rai(i: &Inputs) -> Result<Outcome, CalcError> {
    let nodes = i.get_flag("lymphadenopathy")?;
    let organs = i.get_flag("organomegaly")?;
    let hb = i.get_pos("hb_g_dl")?;
    let plt = i.get_pos("platelets_10e9_l")?;
    let parts = vec![
        Part::met("Lymphadenopathy", nodes),
        Part::met("Hepatomegaly or splenomegaly", organs),
        Part::met("Hb < 11 g/dL", hb < 11.0),
        Part::met("Platelets < 100 × 10³/µL", plt < 100.0),
    ];
    let (stage, band, label, meaning) = if plt < 100.0 {
        (
            4.0,
            "rai_cll.4",
            "Stage IV",
            "Lymphocytosis with thrombocytopenia (platelets < 100 × 10³/µL): high risk.",
        )
    } else if hb < 11.0 {
        (3.0, "rai_cll.3", "Stage III", "Lymphocytosis with anaemia (Hb < 11 g/dL): high risk.")
    } else if organs {
        (2.0, "rai_cll.2", "Stage II", "Lymphocytosis with hepatomegaly or splenomegaly: intermediate risk.")
    } else if nodes {
        (1.0, "rai_cll.1", "Stage I", "Lymphocytosis with lymphadenopathy: intermediate risk.")
    } else {
        (0.0, "rai_cll.0", "Stage 0", "Lymphocytosis only: low risk.")
    };
    Ok(Outcome {
        score: Some(stage),
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(129),
        parts,
        notes: vec![
            "Late stage (Rai or Binet) is a poor prognostic factor (book p. 129). The low / intermediate / high risk groups are the modified Rai (1987), not in the book's table."
                .to_owned(),
        ],
    })
}

fn binet(i: &Inputs) -> Result<Outcome, CalcError> {
    let areas = i.get_num("areas")?;
    if !(0.0..=5.0).contains(&areas) || areas.fract() != 0.0 {
        return Err(CalcError::Invalid { field: "areas", reason: "must be a whole number from 0 to 5" });
    }
    let hb = i.get_pos("hb_g_dl")?;
    let plt = i.get_pos("platelets_10e9_l")?;
    let parts = vec![
        Part::met("3 or more lymphoid areas", areas >= 3.0),
        Part::met("Hb < 10 g/dL", hb < 10.0),
        Part::met("Platelets < 100 × 10³/µL", plt < 100.0),
    ];
    let (band, label, meaning) = if hb < 10.0 || plt < 100.0 {
        (
            "binet_cll.c",
            "Stage C",
            "Anaemia (Hb < 10 g/dL) or thrombocytopenia (platelets < 100 × 10³/µL), whatever the number of areas.",
        )
    } else if areas >= 3.0 {
        ("binet_cll.b", "Stage B", "3 or more lymphoid areas, Hb ≥ 10 g/dL and platelets ≥ 100 × 10³/µL.")
    } else {
        ("binet_cll.a", "Stage A", "Fewer than 3 lymphoid areas, Hb ≥ 10 g/dL and platelets ≥ 100 × 10³/µL.")
    };
    Ok(Outcome {
        score: Some(areas),
        display: label.to_owned(),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(129),
        parts,
        notes: vec![
            "The book's table prints stage C platelets as \"≤ 100\" and A/B as \"≥ 100\"; exactly 100 is read as A/B here (Binet 1981: < 100)."
                .to_owned(),
        ],
    })
}

/// Next step and differential per stage (iwCLL 2018, Hallek; book p. 129 for the DD).
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "rai_cll.0",
        Guide {
            next: &[
                "Watch and wait: CBC and examination every 3–6 months.",
                "Treat only active disease (iwCLL 2018): B symptoms, progressive marrow failure, bulky or growing nodes or spleen, lymphocyte doubling under 6 months.",
                "Vaccinate (pneumococcal, influenza); no live vaccines.",
            ],
            dd: &[
                "Reactive lymphocytosis: pertussis, infectious mononucleosis, TB, toxoplasmosis, autoimmune disease (usually under 15 × 10³/µL and not sustained).",
                "Monoclonal B lymphocytosis: clonal B cells under 5 × 10⁹/L with no nodes, organomegaly or cytopenias.",
                "Prolymphocytic leukaemia: massive spleen, more than 55% prolymphocytes.",
                "Mantle cell lymphoma in the blood: CD5+ but CD23−, cyclin D1 / t(11;14).",
                "Hairy cell leukaemia, marginal zone lymphoma: flow cytometry tells them apart.",
            ],
        },
    ),
    (
        "rai_cll.1",
        Guide {
            next: &[
                "Watch and wait: CBC and examination every 3–6 months.",
                "Treat only active disease (iwCLL 2018): B symptoms, progressive marrow failure, bulky or growing nodes or spleen, lymphocyte doubling under 6 months.",
                "Vaccinate (pneumococcal, influenza); no live vaccines.",
            ],
            dd: &[
                "Reactive lymphocytosis: pertussis, infectious mononucleosis, TB, toxoplasmosis, autoimmune disease (usually under 15 × 10³/µL and not sustained).",
                "Monoclonal B lymphocytosis: clonal B cells under 5 × 10⁹/L with no nodes, organomegaly or cytopenias.",
                "Prolymphocytic leukaemia: massive spleen, more than 55% prolymphocytes.",
                "Mantle cell lymphoma in the blood: CD5+ but CD23−, cyclin D1 / t(11;14).",
                "Hairy cell leukaemia, marginal zone lymphoma: flow cytometry tells them apart.",
            ],
        },
    ),
    (
        "rai_cll.2",
        Guide {
            next: &[
                "Watch and wait: CBC and examination every 3–6 months.",
                "Treat only active disease (iwCLL 2018): B symptoms, progressive marrow failure, bulky or growing nodes or spleen, lymphocyte doubling under 6 months.",
                "Vaccinate (pneumococcal, influenza); no live vaccines.",
            ],
            dd: &[
                "Reactive lymphocytosis: pertussis, infectious mononucleosis, TB, toxoplasmosis, autoimmune disease (usually under 15 × 10³/µL and not sustained).",
                "Monoclonal B lymphocytosis: clonal B cells under 5 × 10⁹/L with no nodes, organomegaly or cytopenias.",
                "Prolymphocytic leukaemia: massive spleen, more than 55% prolymphocytes.",
                "Mantle cell lymphoma in the blood: CD5+ but CD23−, cyclin D1 / t(11;14).",
                "Hairy cell leukaemia, marginal zone lymphoma: flow cytometry tells them apart.",
            ],
        },
    ),
    (
        "binet_cll.a",
        Guide {
            next: &[
                "Watch and wait: CBC and examination every 3–6 months.",
                "Treat only active disease (iwCLL 2018): B symptoms, progressive marrow failure, bulky or growing nodes or spleen, lymphocyte doubling under 6 months.",
                "Vaccinate (pneumococcal, influenza); no live vaccines.",
            ],
            dd: &[
                "Reactive lymphocytosis: pertussis, infectious mononucleosis, TB, toxoplasmosis, autoimmune disease (usually under 15 × 10³/µL and not sustained).",
                "Monoclonal B lymphocytosis: clonal B cells under 5 × 10⁹/L with no nodes, organomegaly or cytopenias.",
                "Prolymphocytic leukaemia: massive spleen, more than 55% prolymphocytes.",
                "Mantle cell lymphoma in the blood: CD5+ but CD23−, cyclin D1 / t(11;14).",
                "Hairy cell leukaemia, marginal zone lymphoma: flow cytometry tells them apart.",
            ],
        },
    ),
    (
        "binet_cll.b",
        Guide {
            next: &[
                "Watch and wait: CBC and examination every 3–6 months.",
                "Treat only active disease (iwCLL 2018): B symptoms, progressive marrow failure, bulky or growing nodes or spleen, lymphocyte doubling under 6 months.",
                "Vaccinate (pneumococcal, influenza); no live vaccines.",
            ],
            dd: &[
                "Reactive lymphocytosis: pertussis, infectious mononucleosis, TB, toxoplasmosis, autoimmune disease (usually under 15 × 10³/µL and not sustained).",
                "Monoclonal B lymphocytosis: clonal B cells under 5 × 10⁹/L with no nodes, organomegaly or cytopenias.",
                "Prolymphocytic leukaemia: massive spleen, more than 55% prolymphocytes.",
                "Mantle cell lymphoma in the blood: CD5+ but CD23−, cyclin D1 / t(11;14).",
                "Hairy cell leukaemia, marginal zone lymphoma: flow cytometry tells them apart.",
            ],
        },
    ),
    (
        "rai_cll.3",
        Guide {
            next: &[
                "Marrow failure is an indication to treat (iwCLL 2018).",
                "First check DAT and reticulocytes: autoimmune cytopenia is treated on its own and doesn't upstage.",
                "Before treatment: FISH for del(17p), TP53 mutation and IGHV status choose the regimen.",
            ],
            dd: &[
                "For the low Hb or platelets: marrow infiltration (true advanced stage) or autoimmune haemolysis (DAT+, high reticulocytes) or ITP, which do not upstage (iwCLL 2018).",
                "Also hypersplenism and treatment-related cytopenias.",
                "Reactive lymphocytosis: pertussis, infectious mononucleosis, TB, toxoplasmosis, autoimmune disease (usually under 15 × 10³/µL and not sustained).",
                "Monoclonal B lymphocytosis: clonal B cells under 5 × 10⁹/L with no nodes, organomegaly or cytopenias.",
                "Prolymphocytic leukaemia: massive spleen, more than 55% prolymphocytes.",
                "Mantle cell lymphoma in the blood: CD5+ but CD23−, cyclin D1 / t(11;14).",
                "Hairy cell leukaemia, marginal zone lymphoma: flow cytometry tells them apart.",
            ],
        },
    ),
    (
        "rai_cll.4",
        Guide {
            next: &[
                "Marrow failure is an indication to treat (iwCLL 2018).",
                "First check DAT and reticulocytes: autoimmune cytopenia is treated on its own and doesn't upstage.",
                "Before treatment: FISH for del(17p), TP53 mutation and IGHV status choose the regimen.",
            ],
            dd: &[
                "For the low Hb or platelets: marrow infiltration (true advanced stage) or autoimmune haemolysis (DAT+, high reticulocytes) or ITP, which do not upstage (iwCLL 2018).",
                "Also hypersplenism and treatment-related cytopenias.",
                "Reactive lymphocytosis: pertussis, infectious mononucleosis, TB, toxoplasmosis, autoimmune disease (usually under 15 × 10³/µL and not sustained).",
                "Monoclonal B lymphocytosis: clonal B cells under 5 × 10⁹/L with no nodes, organomegaly or cytopenias.",
                "Prolymphocytic leukaemia: massive spleen, more than 55% prolymphocytes.",
                "Mantle cell lymphoma in the blood: CD5+ but CD23−, cyclin D1 / t(11;14).",
                "Hairy cell leukaemia, marginal zone lymphoma: flow cytometry tells them apart.",
            ],
        },
    ),
    (
        "binet_cll.c",
        Guide {
            next: &[
                "Marrow failure is an indication to treat (iwCLL 2018).",
                "First check DAT and reticulocytes: autoimmune cytopenia is treated on its own and doesn't upstage.",
                "Before treatment: FISH for del(17p), TP53 mutation and IGHV status choose the regimen.",
            ],
            dd: &[
                "For the low Hb or platelets: marrow infiltration (true advanced stage) or autoimmune haemolysis (DAT+, high reticulocytes) or ITP, which do not upstage (iwCLL 2018).",
                "Also hypersplenism and treatment-related cytopenias.",
                "Reactive lymphocytosis: pertussis, infectious mononucleosis, TB, toxoplasmosis, autoimmune disease (usually under 15 × 10³/µL and not sustained).",
                "Monoclonal B lymphocytosis: clonal B cells under 5 × 10⁹/L with no nodes, organomegaly or cytopenias.",
                "Prolymphocytic leukaemia: massive spleen, more than 55% prolymphocytes.",
                "Mantle cell lymphoma in the blood: CD5+ but CD23−, cyclin D1 / t(11;14).",
                "Hairy cell leukaemia, marginal zone lymphoma: flow cytometry tells them apart.",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn rai_band(nodes: bool, organs: bool, hb: f64, plt: f64) -> &'static str {
        rai(&Inputs::new()
            .flag("lymphadenopathy", nodes)
            .flag("organomegaly", organs)
            .num("hb_g_dl", hb)
            .num("platelets_10e9_l", plt))
        .unwrap()
        .band
    }

    fn binet_band(areas: f64, hb: f64, plt: f64) -> &'static str {
        binet(&Inputs::new().num("areas", areas).num("hb_g_dl", hb).num("platelets_10e9_l", plt))
            .unwrap()
            .band
    }

    #[test]
    fn rai_stages_follow_table_1() {
        assert_eq!(rai_band(false, false, 13.0, 200.0), "rai_cll.0");
        assert_eq!(rai_band(true, false, 13.0, 200.0), "rai_cll.1");
        assert_eq!(rai_band(true, true, 13.0, 200.0), "rai_cll.2");
        assert_eq!(rai_band(false, false, 10.9, 200.0), "rai_cll.3");
        // Platelets decide over anaemia; exactly 11 and 100 are not low.
        assert_eq!(rai_band(false, false, 9.0, 90.0), "rai_cll.4");
        assert_eq!(rai_band(false, false, 11.0, 100.0), "rai_cll.0");
    }

    #[test]
    fn binet_stages_follow_table_1() {
        assert_eq!(binet_band(2.0, 12.0, 150.0), "binet_cll.a");
        assert_eq!(binet_band(3.0, 12.0, 150.0), "binet_cll.b");
        assert_eq!(binet_band(1.0, 9.9, 150.0), "binet_cll.c");
        assert_eq!(binet_band(5.0, 12.0, 99.0), "binet_cll.c");
        assert_eq!(binet_band(0.0, 10.0, 100.0), "binet_cll.a");
        assert!(
            binet(&Inputs::new().num("areas", 6.0).num("hb_g_dl", 12.0).num("platelets_10e9_l", 150.0))
                .is_err()
        );
    }
}
