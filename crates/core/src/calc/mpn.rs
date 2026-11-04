//! WHO 2016 diagnostic criteria for the classic BCR-ABL1-negative
//! myeloproliferative neoplasms: polycythaemia vera, essential
//! thrombocythaemia and overt primary myelofibrosis (Arber 2016).
//!
//! Source: the Hematology book. PV: Lecture 10, p. 163 (revised 2016
//! criteria; the 2008 table is on the same page). ET: Lecture 9, p. 135.
//! Overt PMF: Lecture 9, pp. 145–146. Each needs its major criteria (all
//! of them, or the first ones plus the minor criterion) as the book prints.

use super::{Area, CalcError, CalcSpec, Guide, Inputs, Outcome, Part, choice, flag, num};

const SEX: &[(&str, &str)] = &[("male", "Male"), ("female", "Female")];

pub const PV: CalcSpec = CalcSpec {
    id: "who_pv",
    name: "WHO 2016 criteria: polycythaemia vera",
    area: Area::Blood,
    fields: &[
        choice("sex", "Sex", SEX),
        num("hb_g_dl", "Haemoglobin", "g/dL", 2.0, 30.0, 0.1),
        num("hct_pct", "Haematocrit", "%", 10.0, 80.0, 0.1).optional(),
        flag("rcm_increased", "Increased red cell mass"),
        flag("bm_panmyelosis", "Marrow: hypercellular with trilineage growth (panmyelosis)"),
        flag("jak2", "JAK2 V617F or JAK2 exon 12 mutation"),
        flag("low_epo", "Subnormal serum erythropoietin"),
    ],
    source: "WHO 2016 (Arber 2016); book Hematology Lecture 10, p. 163",
    compute: pv,
};

pub const ET: CalcSpec = CalcSpec {
    id: "who_et",
    name: "WHO 2016 criteria: essential thrombocythaemia",
    area: Area::Blood,
    fields: &[
        num("platelets_10e9_l", "Platelets", "× 10⁹/L", 1.0, 5000.0, 1.0),
        flag("bm_megakaryocytes", "Marrow: mainly enlarged, mature, hyperlobulated megakaryocytes"),
        flag(
            "not_other_myeloid",
            "Not meeting WHO criteria for CML, PV, PMF, MDS or another myeloid neoplasm",
        ),
        flag("driver_mutation", "JAK2, CALR or MPL mutation"),
        flag("clonal_or_not_reactive", "Another clonal marker, or no evidence of reactive thrombocytosis"),
    ],
    source: "WHO 2016 (Arber 2016); book Hematology Lecture 9, p. 135",
    compute: et,
};

pub const PMF: CalcSpec = CalcSpec {
    id: "who_pmf",
    name: "WHO 2016 criteria: overt primary myelofibrosis",
    area: Area::Blood,
    fields: &[
        flag(
            "bm_megakaryocytes_fibrosis",
            "Marrow: megakaryocytic proliferation and atypia with grade 2–3 fibrosis",
        ),
        flag(
            "not_other_myeloid",
            "Not meeting WHO criteria for ET, PV, CML, MDS or another myeloid neoplasm",
        ),
        flag("clonal", "JAK2, CALR or MPL mutation, or another clonal marker, or no reactive myelofibrosis")
            .hint("Other markers: ASXL1, EZH2, TET2, IDH1/IDH2, SRSF2, SF3B1."),
        flag("anaemia", "Anaemia not due to another condition"),
        flag("leukocytosis", "Leukocytosis > 11 × 10⁹/L"),
        flag("splenomegaly", "Palpable splenomegaly"),
        flag("ldh_high", "LDH above the upper limit of normal"),
        flag("leukoerythroblastosis", "Leukoerythroblastosis"),
    ],
    source: "WHO 2016 (Arber 2016); book Hematology Lecture 9, pp. 145–146",
    compute: pmf,
};

/// `bands` = (met, not met).
fn verdict(
    met: bool,
    bands: (&'static str, &'static str),
    name: &str,
    page: u32,
    parts: Vec<Part>,
    notes: Vec<String>,
) -> Outcome {
    let (band, label, meaning) = if met {
        (bands.0, "Criteria met", format!("Meets the WHO 2016 criteria for {name}."))
    } else {
        (bands.1, "Criteria not met", format!("Does not meet the WHO 2016 criteria for {name}."))
    };
    Outcome { score: None, display: label.to_owned(), band, label, meaning, page: Some(page), parts, notes }
}

fn pv(i: &Inputs) -> Result<Outcome, CalcError> {
    let male = i.get_option("sex", &[("male", &["m", "man"]), ("female", &["f", "woman"])])? == "male";
    let hb = i.get_pos("hb_g_dl")?;
    let hct = i.opt_num("hct_pct")?;
    let rcm = i.get_flag("rcm_increased")?;
    let (hb_cut, hct_cut) = if male { (16.5, 49.0) } else { (16.0, 48.0) };
    let major1 = hb > hb_cut || hct.is_some_and(|h| h > hct_cut) || rcm;
    let major2 = i.get_flag("bm_panmyelosis")?;
    let major3 = i.get_flag("jak2")?;
    let minor = i.get_flag("low_epo")?;
    let parts = vec![
        Part::met(
            format!("Major 1: Hb > {hb_cut} g/dL, Hct > {hct_cut}% or increased red cell mass"),
            major1,
        ),
        Part::met("Major 2: panmyelosis on marrow biopsy", major2),
        Part::met("Major 3: JAK2 mutation", major3),
        Part::met("Minor: subnormal erythropoietin", minor),
    ];
    let met = major1 && major2 && (major3 || minor);
    let notes = vec![
        "Needs all 3 major criteria, or the first 2 major criteria and the minor one. WHO 2016 adds that the marrow biopsy may be skipped with sustained Hb > 18.5 g/dL (men) or > 16.5 (women), JAK2 and low erythropoietin."
            .to_owned(),
    ];
    Ok(verdict(met, ("who_pv.met", "who_pv.not_met"), "polycythaemia vera", 163, parts, notes))
}

fn et(i: &Inputs) -> Result<Outcome, CalcError> {
    let plt = i.get_pos("platelets_10e9_l")?;
    let majors = [
        Part::met("Major 1: platelets ≥ 450 × 10⁹/L", plt >= 450.0),
        Part::met("Major 2: megakaryocytic marrow", i.get_flag("bm_megakaryocytes")?),
        Part::met("Major 3: not another myeloid neoplasm", i.get_flag("not_other_myeloid")?),
        Part::met("Major 4: JAK2, CALR or MPL mutation", i.get_flag("driver_mutation")?),
    ];
    let minor = Part::met("Minor: clonal marker or no reactive cause", i.get_flag("clonal_or_not_reactive")?);
    let yes = |p: &Part| p.met == Some(true);
    let first_three = majors[..3].iter().all(yes);
    let met = first_three && (yes(&majors[3]) || yes(&minor));
    let mut parts = majors.to_vec();
    parts.push(minor);
    let notes =
        vec!["Needs all 4 major criteria, or the first 3 major criteria and the minor one.".to_owned()];
    Ok(verdict(met, ("who_et.met", "who_et.not_met"), "essential thrombocythaemia", 135, parts, notes))
}

fn pmf(i: &Inputs) -> Result<Outcome, CalcError> {
    let majors = [
        Part::met(
            "Major 1: megakaryocytic atypia with grade 2–3 fibrosis",
            i.get_flag("bm_megakaryocytes_fibrosis")?,
        ),
        Part::met("Major 2: not another myeloid neoplasm", i.get_flag("not_other_myeloid")?),
        Part::met("Major 3: clonal marker or no reactive fibrosis", i.get_flag("clonal")?),
    ];
    let minors = [
        Part::met("Minor: anaemia", i.get_flag("anaemia")?),
        Part::met("Minor: WBC > 11 × 10⁹/L", i.get_flag("leukocytosis")?),
        Part::met("Minor: palpable splenomegaly", i.get_flag("splenomegaly")?),
        Part::met("Minor: LDH above normal", i.get_flag("ldh_high")?),
        Part::met("Minor: leukoerythroblastosis", i.get_flag("leukoerythroblastosis")?),
    ];
    let yes = |p: &Part| p.met == Some(true);
    let met = majors.iter().all(yes) && minors.iter().any(yes);
    let parts = majors.into_iter().chain(minors).collect();
    let notes = vec![
        "Needs all 3 major criteria and at least one minor criterion, confirmed in two consecutive determinations."
            .to_owned(),
    ];
    Ok(verdict(met, ("who_pmf.met", "who_pmf.not_met"), "overt primary myelofibrosis", 145, parts, notes))
}

/// Next step and differential per verdict (Arber 2016; ELN 2018 / Tefferi for management).
pub const GUIDES: &[(&str, Guide)] = &[
    (
        "who_pv.met",
        Guide {
            next: &[
                "Phlebotomy to a haematocrit under 45% and low-dose aspirin for everyone.",
                "Risk: age 60 or more or a previous thrombosis is high risk.",
                "High risk adds cytoreduction: hydroxyurea, or interferon when young or pregnant.",
                "Treat cardiovascular risk factors.",
            ],
            dd: &[
                "Secondary erythrocytosis from hypoxia: COPD, sleep apnoea, cyanotic heart disease, high altitude, smoking and CO.",
                "High oxygen-affinity haemoglobin, EPO receptor mutation (familial) (book p. 163).",
                "EPO-secreting tumours: renal cell and hepatocellular carcinoma, cerebellar haemangioblastoma, uterine fibroids.",
                "After renal transplant; testosterone or SGLT2 inhibitors.",
                "Relative polycythaemia: dehydration, diuretics.",
            ],
        },
    ),
    (
        "who_pv.not_met",
        Guide {
            next: &[
                "Erythropoietin level: high or normal points to a secondary cause.",
                "Oxygen saturation, sleep study, chest and renal or liver imaging.",
                "JAK2 exon 12 if V617F is negative but EPO is low.",
            ],
            dd: &[
                "Secondary erythrocytosis from hypoxia: COPD, sleep apnoea, cyanotic heart disease, high altitude, smoking and CO.",
                "High oxygen-affinity haemoglobin, EPO receptor mutation (familial) (book p. 163).",
                "EPO-secreting tumours: renal cell and hepatocellular carcinoma, cerebellar haemangioblastoma, uterine fibroids.",
                "After renal transplant; testosterone or SGLT2 inhibitors.",
                "Relative polycythaemia: dehydration, diuretics.",
            ],
        },
    ),
    (
        "who_et.met",
        Guide {
            next: &[
                "Thrombosis risk (revised IPSET: age over 60, previous thrombosis, JAK2).",
                "Aspirin for most; cytoreduction (hydroxyurea) when high risk.",
                "Platelets over 1500 × 10⁹/L: check for acquired von Willebrand before aspirin.",
            ],
            dd: &[
                "Reactive thrombocytosis: iron deficiency, infection or inflammation, cancer, bleeding, after splenectomy or surgery.",
                "CML: always test BCR-ABL1.",
                "Prefibrotic PMF, PV masked by iron deficiency.",
                "MDS with del(5q), MDS/MPN with ring sideroblasts and thrombocytosis.",
            ],
        },
    ),
    (
        "who_et.not_met",
        Guide {
            next: &[
                "Rule out a reactive cause first: ferritin, CRP, blood film.",
                "Then BCR-ABL1, JAK2, CALR and MPL, and a marrow biopsy.",
            ],
            dd: &[
                "Reactive thrombocytosis: iron deficiency, infection or inflammation, cancer, bleeding, after splenectomy or surgery.",
                "CML: always test BCR-ABL1.",
                "Prefibrotic PMF, PV masked by iron deficiency.",
                "MDS with del(5q), MDS/MPN with ring sideroblasts and thrombocytosis.",
            ],
        },
    ),
    (
        "who_pmf.met",
        Guide {
            next: &[
                "Prognosis: IPSS at diagnosis (in this list), DIPSS later.",
                "High risk in a fit patient: refer for allogeneic stem cell transplant.",
                "JAK inhibitor (ruxolitinib) for the spleen and symptoms; treat the anaemia.",
            ],
            dd: &[
                "Reactive marrow fibrosis: infection (TB), autoimmune disease, chronic inflammation (book p. 146).",
                "Hairy cell leukaemia and other lymphoid neoplasms, metastatic cancer, toxic myelopathy.",
                "Other myeloid neoplasms: CML, MDS with fibrosis, post-PV or post-ET myelofibrosis, acute megakaryoblastic leukaemia.",
                "Prefibrotic PMF when fibrosis is grade 0–1.",
            ],
        },
    ),
    (
        "who_pmf.not_met",
        Guide {
            next: &[
                "If the marrow is fibrotic, look for another cause before calling it PMF.",
                "With grade 0–1 fibrosis, check the WHO criteria for prefibrotic PMF.",
            ],
            dd: &[
                "Reactive marrow fibrosis: infection (TB), autoimmune disease, chronic inflammation (book p. 146).",
                "Hairy cell leukaemia and other lymphoid neoplasms, metastatic cancer, toxic myelopathy.",
                "Other myeloid neoplasms: CML, MDS with fibrosis, post-PV or post-ET myelofibrosis, acute megakaryoblastic leukaemia.",
                "Prefibrotic PMF when fibrosis is grade 0–1.",
            ],
        },
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pv_needs_two_majors_and_a_third_or_the_minor() {
        let base = || Inputs::new().choice("sex", "female").num("hb_g_dl", 16.2).flag("bm_panmyelosis", true);
        assert_eq!(pv(&base().flag("jak2", true)).unwrap().band, "who_pv.met");
        assert_eq!(pv(&base().flag("low_epo", true)).unwrap().band, "who_pv.met");
        assert_eq!(pv(&base()).unwrap().band, "who_pv.not_met");
        // 16.2 is over the women's cut-off but not the men's; Hct can stand in.
        let man = Inputs::new()
            .choice("sex", "male")
            .num("hb_g_dl", 16.2)
            .flag("bm_panmyelosis", true)
            .flag("jak2", true);
        assert_eq!(pv(&man).unwrap().band, "who_pv.not_met");
        assert_eq!(pv(&man.num("hct_pct", 50.0)).unwrap().band, "who_pv.met");
    }

    #[test]
    fn et_needs_three_majors_and_the_fourth_or_the_minor() {
        let base = || {
            Inputs::new()
                .num("platelets_10e9_l", 450.0)
                .flag("bm_megakaryocytes", true)
                .flag("not_other_myeloid", true)
        };
        assert_eq!(et(&base().flag("driver_mutation", true)).unwrap().band, "who_et.met");
        assert_eq!(et(&base().flag("clonal_or_not_reactive", true)).unwrap().band, "who_et.met");
        assert_eq!(et(&base()).unwrap().band, "who_et.not_met");
        let low = base().num("platelets_10e9_l", 449.0).flag("driver_mutation", true);
        assert_eq!(et(&low).unwrap().band, "who_et.not_met");
    }

    #[test]
    fn pmf_needs_all_majors_and_one_minor() {
        let base = || {
            Inputs::new()
                .flag("bm_megakaryocytes_fibrosis", true)
                .flag("not_other_myeloid", true)
                .flag("clonal", true)
        };
        assert_eq!(pmf(&base()).unwrap().band, "who_pmf.not_met");
        assert_eq!(pmf(&base().flag("splenomegaly", true)).unwrap().band, "who_pmf.met");
        let o = pmf(&base().flag("clonal", false).flag("anaemia", true)).unwrap();
        assert_eq!(o.band, "who_pmf.not_met");
    }
}
