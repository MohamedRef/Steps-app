//! The Calculators screen (a tab of Cases), organised the way MDCalc and
//! Medscape list theirs: a search field and an area filter on top, the
//! pinned (★) and the last few opened first, then every calculator under its
//! area as a full-width row with its name and one Arabic line on what it is
//! for. On phones the list and the open calculator take turns; from 840dp
//! the open one sits beside the list, and that pane is never empty: it shows
//! the one he used last.

use dioxus::prelude::*;
use steps_core::calc::{self, Area, CalcSpec, Inputs};

use super::{AREAS, CSS, CalcForm, area_label};
use crate::i18n::calc as tx;

/// Pinned calculators: a synced setting, so the phone and the Mac agree.
const FAVOURITES_KEY: &str = "calc.favourites";
/// The last few opened, newest first, kept on this device.
const RECENT_KEY: &str = "calc.recent";
const RECENT_MAX: usize = 5;

/// Pinned ids in the order he pinned them. `None` until first read.
static FAVOURITES: GlobalSignal<Option<Vec<&'static str>>> = Signal::global(|| None);

/// Only ids this build knows, each once (a sync from a newer build, or a
/// calculator that was renamed, leaves the rest out).
fn known(ids: Option<Vec<String>>) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for id in ids.unwrap_or_default() {
        if let Some(spec) = calc::spec(&id)
            && !out.contains(&spec.id)
        {
            out.push(spec.id);
        }
    }
    out
}

fn read_favourites() -> Vec<&'static str> {
    known(crate::store::shared().setting::<Vec<String>>(FAVOURITES_KEY))
}

fn favourites() -> Vec<&'static str> {
    if FAVOURITES.peek().is_none() {
        *FAVOURITES.write() = Some(read_favourites());
    }
    FAVOURITES.read().clone().unwrap_or_default()
}

/// Re-reads the pins after a sync, restore or import (see `crate::synced`).
pub fn reload() {
    let fresh = Some(read_favourites());
    if *FAVOURITES.peek() != fresh {
        *FAVOURITES.write() = fresh;
    }
}

/// The list with `id` pinned (at the end) or unpinned.
fn toggled(list: &[&'static str], id: &'static str) -> Vec<&'static str> {
    if list.contains(&id) {
        list.iter().copied().filter(|x| *x != id).collect()
    } else {
        list.iter().copied().chain([id]).collect()
    }
}

fn toggle_favourite(id: &'static str) {
    let next = toggled(&favourites(), id);
    crate::store::save_setting(FAVOURITES_KEY, &next);
    *FAVOURITES.write() = Some(next);
}

/// `id` first, then the rest, at most [`RECENT_MAX`].
fn pushed(recent: &[&'static str], id: &'static str) -> Vec<&'static str> {
    [id].into_iter().chain(recent.iter().copied().filter(|x| *x != id)).take(RECENT_MAX).collect()
}

/// What the wide pane shows when he hasn't picked one this time: the last
/// one he opened, else his first pin, else the first calculator.
fn fallback(recent: &[&'static str], favourites: &[&'static str]) -> &'static str {
    recent.first().or(favourites.first()).copied().unwrap_or(calc::all()[0].id)
}

/// Text folded for search: lower case, Arabic without tashkeel or tatweel,
/// one alef, ة as ه, ى and ئ as ي, ؤ as و, Arabic-Indic digits as ASCII.
fn fold(s: &str) -> String {
    s.chars()
        .filter_map(|c| match c {
            '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{0640}' => None,
            'أ' | 'إ' | 'آ' | 'ٱ' => Some('ا'),
            'ة' => Some('ه'),
            'ى' | 'ئ' => Some('ي'),
            'ؤ' => Some('و'),
            '٠'..='٩' => char::from_digit(c as u32 - '٠' as u32, 10),
            _ => Some(c),
        })
        .flat_map(char::to_lowercase)
        .collect()
}

/// Letters and digits only, so "fib4" finds FIB-4 and "meldna" MELD-Na.
fn squash(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect()
}

/// More words a calculator is looked up by (English names, abbreviations,
/// the condition it is used in).
fn keywords(id: &str) -> &'static str {
    match id {
        "child_pugh" => "cirrhosis ctp class سيروزس",
        "meld" => "cirrhosis transplant end-stage liver disease سيروزس زرع",
        "meld_na" => "cirrhosis transplant sodium hyponatremia سيروزس زرع",
        "albi" => "hepatocellular carcinoma albumin bilirubin",
        "fib4" => "fibrosis non-invasive nafld masld",
        "apri" => "fibrosis non-invasive hepatitis",
        "fli" => "fatty liver index nafld masld steatosis bedogni",
        "baveno_vi" => {
            "varices screening endoscopy fibroscan elastography stiffness cacld portal hypertension"
        }
        "r_ratio" => "dili drug induced liver injury alt alp",
        "maddrey" => "alcohol steroids prednisolone discriminant",
        "glasgow_ah" => "gahs alcohol steroids",
        "saag" => "ascites serum albumin gradient portal hypertension",
        "ascitic_pmn" => "spontaneous bacterial peritonitis paracentesis neutrophils",
        "sos_criteria" => "sinusoidal obstruction syndrome veno-occlusive vod bmt hsct seattle baltimore",
        "cha2ds2_vasc" => "chads atrial fibrillation af stroke anticoagulation bridging endoscopy",
        "bclc" => "hcc barcelona liver cancer",
        "milan" => "hcc transplant liver cancer",
        "okuda" => "hcc liver cancer stage",
        "page_b" => "hcc hbv hepatitis b surveillance screening",
        "kings_college" => "acute liver failure paracetamol acetaminophen transplant kcc",
        "clichy" => "acute liver failure factor v transplant",
        "ranson" | "bisap" => "pancreatitis",
        "ctsi" => "pancreatitis balthazar ct necrosis",
        "atlanta_2012" => "pancreatitis organ failure",
        "glasgow_blatchford" => "gbs upper gi bleeding haematemesis melaena نزيف",
        "shock_index" => "si haemodynamic instability bleeding hypotension نزيف",
        "oakland" => "lower gi bleeding lgib haematochezia discharge نزيف",
        "uc_activity" => "ulcerative colitis mayo truelove witts severity ibd",
        "kyoto_gastritis" => "h pylori helicobacter endoscopy atrophy intestinal metaplasia gastric cancer",
        "rome4_ibs" => "irritable bowel syndrome functional bristol ibs-c ibs-d",
        "anion_gap" => "acidosis metabolic acid base",
        "delta_ratio" => "delta gap acidosis acid base",
        "expected_compensation" => "winter formula acid base abg respiratory metabolic",
        "osmolal_gap" => "toxic alcohol methanol ethylene glycol",
        "kdigo_aki" => "acute kidney injury creatinine urine output",
        "fena" => "fractional excretion sodium prerenal atn aki",
        "feurea" => "fractional excretion urea diuretics aki",
        "bun_creatinine" => "urea ratio prerenal atn aki azotemia",
        "ckd_epi_2021" => "egfr gfr chronic kidney disease creatinine ckd",
        "kdigo_ckd" => "ckd chronic kidney disease albuminuria acr heat map g3a",
        "aa_gradient" => "alveolar arterial oxygen hypoxia abg pao2",
        "free_water_deficit" => "hypernatremia hypernatraemia sodium water deficit fluid",
        "plasmic" => "ttp thrombotic microangiopathy adamts13 tma schistocytes",
        "cockcroft_gault" => "crcl gfr renal dose elderly creatinine clearance",
        "katz_adl" => "adl activities daily living function elderly katz",
        "lawton_iadl" => "iadl instrumental activities daily living function elderly",
        "gds15" => "gds depression elderly mood اكتئاب",
        "mna_sf" => "mna nutrition malnutrition elderly bmi weight تغذية",
        "clinical_frailty" => "cfs frailty frail rockwood elderly هشاشة",
        "cam" => "delirium confusion acute confusional state",
        "sarc_f" => "sarcopenia muscle falls elderly",
        "mmse" => "dementia cognition memory folstein mini mental خرف",
        "abi" => "abpi ankle brachial pad peripheral arterial disease claudication diabetic foot",
        "acr_eular_2010_ra" => "rheumatoid arthritis criteria",
        "caspar" => "psoriatic arthritis psa spondyloarthritis criteria صدفية",
        "asas_axial_spa" => "axial spondyloarthritis nr-axspa ankylosing hla-b27 sacroiliitis criteria",
        "asas_peripheral_spa" => "peripheral spondyloarthritis enthesitis dactylitis reactive criteria",
        "modified_new_york" => "ankylosing spondylitis as sacroiliitis criteria 1984",
        "sle_acr_1997" => "lupus systemic lupus erythematosus criteria ذئبة",
        "sle_eular_acr_2019" => "lupus systemic lupus erythematosus ana criteria ذئبة",
        "gpa_2022" => "wegener granulomatosis polyangiitis anca vasculitis pr3",
        "mpa_2022" => "microscopic polyangiitis anca vasculitis mpo",
        "corrected_sodium" => "hyperglycemia hyperglycaemia pseudohyponatremia dka hhs glucose",
        "effective_osmolality" => "hhs hyperosmolar dka glucose tonicity",
        "dka_severity" => "diabetic ketoacidosis ketones bicarbonate euglycemic",
        "premixed_insulin" => "biphasic insulin mixtard dose twice daily انسولين",
        "bmi" => "body mass index obesity overweight weight height سمنة",
        "ipss_mf" => "myelofibrosis prognosis",
        "rpi" => "reticulocyte index ri arc absolute count anaemia anemia haemolysis hemolysis أنيميا",
        "rai_cll" | "binet_cll" => "cll chronic lymphocytic leukaemia leukemia staging",
        "ann_arbor" => "lymphoma hodgkin nhl staging b symptoms ليمفوما",
        "ipi_nhl" => "lymphoma nhl prognosis dlbcl ليمفوما",
        "who_pv" => "polycythemia polycythaemia prv jak2 mpn myeloproliferative",
        "who_et" => "thrombocythemia thrombocytosis platelets jak2 calr mpl mpn myeloproliferative",
        "who_pmf" => "myelofibrosis pmf jak2 calr mpl mpn myeloproliferative",
        "incidence" | "prevalence" | "prevalence_id" => "epidemiology rate",
        "attack_rate" => "outbreak epidemic",
        "sar" => "secondary attack contacts",
        "cfr" => "case fatality mortality",
        "pmr" => "mortality",
        "imr" => "infant mortality",
        "nmr" => "neonatal mortality",
        "relative_risk" => "rr cohort",
        "odds_ratio" => "or case control",
        "attributable_risk" => "ar risk difference",
        "nnt" => "number needed to treat harm nnh",
        "diagnostic_accuracy" => "sensitivity specificity ppv npv likelihood ratio",
        "z_score" => "standard deviation sd normal distribution",
        _ => "",
    }
}

/// Everything a calculator can be found by, folded.
fn haystack(spec: &CalcSpec) -> String {
    fold(&format!(
        "{} {} {} {} {}",
        spec.name,
        spec.id.replace('_', " "),
        tx::blurb(spec.id).unwrap_or_default(),
        area_label(spec.area),
        keywords(spec.id),
    ))
}

/// Whether every word of `query` is in the calculator's name, its line, its
/// area or its keywords.
fn matches(spec: &CalcSpec, query: &str) -> bool {
    let hay = haystack(spec);
    let flat = squash(&hay);
    fold(query).split_whitespace().all(|word| {
        let word_flat = squash(word);
        hay.contains(word) || (!word_flat.is_empty() && flat.contains(&word_flat))
    })
}

/// Names that start with the query first, then names that hold it, then the
/// rest; the screen's order inside each.
fn rank(spec: &CalcSpec, query: &str) -> u8 {
    let name = fold(spec.name);
    let q = fold(query.trim());
    if name.starts_with(&q) || squash(&name).starts_with(&squash(&q)) {
        0
    } else if name.contains(&q) {
        1
    } else {
        2
    }
}

/// The calculators a search (and area filter) shows, best first.
fn search(query: &str, area: Option<Area>) -> Vec<&'static CalcSpec> {
    let mut found: Vec<&'static CalcSpec> =
        calc::all().iter().filter(|s| area.is_none_or(|a| s.area == a) && matches(s, query)).collect();
    found.sort_by_key(|s| rank(s, query));
    found
}

/// One block of rows under a heading.
struct Group {
    key: &'static str,
    label: String,
    ids: Vec<&'static str>,
    /// Search results name their area on each row.
    with_area: bool,
}

/// What the list shows: results for a search; one area when the filter is
/// set; otherwise pins, recents, then every area.
fn groups(query: &str, area: Option<Area>, favs: &[&'static str], recent: &[&'static str]) -> Vec<Group> {
    if !query.trim().is_empty() {
        let ids: Vec<&'static str> = search(query, area).iter().map(|s| s.id).collect();
        return if ids.is_empty() {
            Vec::new()
        } else {
            vec![Group { key: "results", label: tx::RESULTS.to_owned(), ids, with_area: true }]
        };
    }
    let mut out = Vec::new();
    if area.is_none() {
        if !favs.is_empty() {
            out.push(Group {
                key: "favourites",
                label: tx::FAVOURITES.to_owned(),
                ids: favs.to_vec(),
                with_area: true,
            });
        }
        // Pinned ones are already on top.
        let recent: Vec<&'static str> = recent.iter().copied().filter(|id| !favs.contains(id)).collect();
        if !recent.is_empty() {
            out.push(Group { key: "recent", label: tx::RECENT.to_owned(), ids: recent, with_area: true });
        }
    }
    for a in AREAS.into_iter().filter(|a| area.is_none_or(|x| x == *a)) {
        let ids: Vec<&'static str> = calc::all().iter().filter(|s| s.area == a).map(|s| s.id).collect();
        if !ids.is_empty() {
            out.push(Group { key: area_key(a), label: area_label(a).to_owned(), ids, with_area: false });
        }
    }
    out
}

/// A stable name for an area, for element ids and the filter's values.
fn area_key(area: Area) -> &'static str {
    match area {
        Area::Liver => "liver",
        Area::Ascites => "ascites",
        Area::Tumour => "tumour",
        Area::AcuteFailure => "acute",
        Area::Pancreas => "pancreas",
        Area::Gi => "gi",
        Area::Renal => "renal",
        Area::Elderly => "elderly",
        Area::Rheum => "rheum",
        Area::Diabetes => "diabetes",
        Area::Blood => "blood",
        Area::Research => "research",
    }
}

fn scroll_top() {
    document::eval(r#"document.querySelector("main.content")?.scrollTo(0, 0);"#);
}

/// Every calculator: the list, and the open one beside it or instead of it.
#[component]
pub fn Calculators() -> Element {
    // STEPS_OPEN_CALC=child_pugh opens one at launch (previews, screenshots).
    let mut open =
        use_signal(|| std::env::var("STEPS_OPEN_CALC").ok().and_then(|id| calc::spec(&id)).map(|s| s.id));
    let mut query = use_signal(String::new);
    let mut area = use_signal(|| None::<Area>);
    let mut recent = use_signal(|| known(crate::store::shared().local_setting::<Vec<String>>(RECENT_KEY)));
    // The row he opened it from, so Back on a phone lands on it again.
    let mut from = use_signal(|| None::<String>);
    let favs = favourites();
    let current = open();
    let shown = current.unwrap_or_else(|| fallback(&recent.read(), &favs));
    let q = query();
    let picked = area();
    let groups = groups(&q, picked, &favs, &recent.read());
    let first = groups.first().and_then(|g| g.ids.first().map(|id| (*id, format!("calc-{}-{id}", g.key))));
    let mut go = move |id: &'static str, row: String| {
        open.set(Some(id));
        from.set(Some(row));
        let next = pushed(&recent.peek(), id);
        let _ = crate::store::shared().put_local_setting(RECENT_KEY, &next);
        recent.set(next);
        scroll_top();
        // On a phone the list goes away: focus moves to the way back.
        document::eval(
            r#"requestAnimationFrame(() => requestAnimationFrame(() => { const b = document.querySelector(".calc-back"); if (b && b.offsetParent) b.focus({preventScroll: true}); }));"#,
        );
    };
    let back = move |_| {
        open.set(None);
        if let Some(row) = from() {
            // Once the list is drawn again: the row he left from, in view and focused.
            document::eval(&format!(
                r#"requestAnimationFrame(() => requestAnimationFrame(() => {{ const r = document.getElementById("{row}"); r?.scrollIntoView({{block: "center"}}); r?.focus({{preventScroll: true}}); }}));"#
            ));
        }
    };
    let total = calc::all().len();
    rsx! {
        style { {CSS.text()} }
        div { class: "calcs", "data-open": if current.is_some() { "yes" } else { "no" },
            nav { class: "calc-list", aria_label: tx::PICK,
                header { class: "page-head",
                    h1 { {tx::TITLE} }
                    p { class: "page-sub", {tx::count(total)} }
                }
                div { class: "calc-tools",
                    label { class: "search calc-search",
                        {crate::icons::search()}
                        input {
                            r#type: "search",
                            // Not "auto": an empty field would turn LTR and its
                            // text would run under the magnifier.
                            dir: "rtl",
                            aria_label: tx::SEARCH_LABEL,
                            placeholder: tx::SEARCH,
                            value: "{q}",
                            oninput: move |e| query.set(e.value()),
                            // Enter opens the first one listed.
                            onkeydown: move |e: KeyboardEvent| {
                                if e.key() == Key::Enter
                                    && let Some((id, row)) = first.clone()
                                {
                                    e.prevent_default();
                                    go(id, row);
                                }
                            },
                        }
                    }
                    select {
                        class: "calc-area",
                        aria_label: tx::AREA_FILTER,
                        "data-set": if picked.is_some() { "true" } else { "false" },
                        onchange: move |e: FormEvent| {
                            let v = e.value();
                            area.set(AREAS.into_iter().find(|a| area_key(*a) == v));
                        },
                        option { value: "", selected: picked.is_none(), {tx::ALL_AREAS} }
                        for a in AREAS {
                            option { key: "{area_key(a)}", value: area_key(a), selected: picked == Some(a), {area_label(a)} }
                        }
                    }
                }
                if groups.is_empty() {
                    div { class: "calc-empty",
                        p { class: "muted", {tx::NOTHING_FOUND} }
                        button {
                            class: "btn-link",
                            onclick: move |_| {
                                query.set(String::new());
                                area.set(None);
                            },
                            {tx::CLEAR_SEARCH}
                        }
                    }
                }
                for g in groups {
                    section { key: "{g.key}", class: "calc-sec", aria_labelledby: "calc-sec-{g.key}",
                        h2 { id: "calc-sec-{g.key}", class: "calc-sec-head",
                            span { {g.label} }
                            span { class: "calc-sec-n", "{g.ids.len()}" }
                        }
                        ul { class: "coll calc-rows", "data-layout": "list",
                            for id in g.ids {
                                CalcRow {
                                    key: "{id}",
                                    id,
                                    row: format!("calc-{}-{id}", g.key),
                                    with_area: g.with_area,
                                    shown: shown == id,
                                    current: current == Some(id),
                                    pinned: favs.contains(&id),
                                    on_open: move |(id, row): (&'static str, String)| go(id, row),
                                }
                            }
                        }
                    }
                }
            }
            div { class: "calc-pane",
                if let Some(spec) = calc::spec(shown) {
                    div { key: "{spec.id}", class: "calc-open",
                        div { class: "calc-bar",
                            span { class: "calc-crumb muted small", {area_label(spec.area)} }
                            button {
                                class: "btn-link calc-back",
                                "aria-keyshortcuts": "Escape",
                                "data-back": "true",
                                onclick: back,
                                span { aria_hidden: "true", "→ " }
                                {tx::BACK}
                            }
                            PinButton { id: spec.id, pinned: favs.contains(&spec.id), with_label: true }
                        }
                        CalcForm { id: spec.id, initial: Inputs::new() }
                    }
                }
            }
        }
    }
}

/// One calculator in the list: the row opens it, the star pins it.
#[component]
fn CalcRow(
    id: &'static str,
    row: String,
    with_area: bool,
    shown: bool,
    current: bool,
    pinned: bool,
    on_open: EventHandler<(&'static str, String)>,
) -> Element {
    let Some(spec) = calc::spec(id) else { return rsx! {} };
    let dom = row.clone();
    rsx! {
        li { class: "calc-li", "data-shown": if shown { "true" } else { "false" },
            button {
                id: "{dom}",
                class: "coll-btn calc-row",
                "aria-current": if current { "true" } else { "false" },
                onclick: move |_| on_open.call((id, row.clone())),
                span { class: "coll-main",
                    bdi { class: "coll-title", dir: "ltr", lang: "en", {spec.name} }
                    span { class: "coll-sub",
                        if with_area {
                            span { class: "calc-row-area", {area_label(spec.area)} }
                            " · "
                        }
                        {tx::blurb(id).unwrap_or_default()}
                    }
                }
            }
            PinButton { id, pinned, with_label: false }
        }
    }
}

/// The star that pins a calculator to the top of the list.
#[component]
fn PinButton(id: &'static str, pinned: bool, with_label: bool) -> Element {
    let hint = if pinned { tx::UNPIN } else { tx::PIN };
    let name = calc::spec(id).map_or(id, |s| s.name);
    rsx! {
        if with_label {
            // A toggle named by its text; aria-pressed says whether it's on.
            button {
                class: "calc-pin calc-pin-wide",
                "aria-pressed": if pinned { "true" } else { "false" },
                title: hint,
                onclick: move |_| toggle_favourite(id),
                {crate::icons::star(pinned)}
                span { if pinned { {tx::PINNED} } else { {tx::PIN} } }
            }
        } else {
            button {
                class: "calc-pin",
                "aria-pressed": if pinned { "true" } else { "false" },
                aria_label: "{tx::PIN}: {name}",
                title: hint,
                onclick: move |_| toggle_favourite(id),
                {crate::icons::star(pinned)}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(found: &[&'static CalcSpec]) -> Vec<&'static str> {
        found.iter().map(|s| s.id).collect()
    }

    #[test]
    fn every_calculator_has_a_blurb_and_an_area_in_the_list() {
        for spec in calc::all() {
            assert!(tx::blurb(spec.id).is_some_and(|b| !b.is_empty()), "{} has no Arabic line", spec.id);
            assert!(AREAS.contains(&spec.area), "{} sits in an area the list never shows", spec.id);
        }
        let listed: usize = groups("", None, &[], &[]).iter().map(|g| g.ids.len()).sum();
        assert_eq!(listed, calc::all().len());
    }

    #[test]
    fn search_finds_by_name_abbreviation_arabic_and_area() {
        assert_eq!(ids(&search("meld", None))[..2], ["meld", "meld_na"]);
        assert_eq!(ids(&search("meldna", None)), ["meld_na"]);
        assert_eq!(ids(&search("fib4", None)), ["fib4"]);
        assert_eq!(ids(&search("child pugh", None)), ["child_pugh"]);
        assert!(ids(&search("صوديوم", None)).contains(&"corrected_sodium"));
        // ى and ي, and the area's own name.
        let renal = ids(&search("كلي", None));
        assert!(renal.contains(&"kdigo_aki") && renal.contains(&"anion_gap"), "{renal:?}");
        assert_eq!(ids(&search("كلى", None)), renal);
        // An English word for a calculator named by its author.
        assert_eq!(ids(&search("pancreatitis", None)), ["ranson", "bisap", "ctsi", "atlanta_2012"]);
        assert!(search("zzz", None).is_empty());
    }

    #[test]
    fn the_area_filter_narrows_search_and_list() {
        let renal = ids(&search("ratio", Some(Area::Renal)));
        assert!(renal.contains(&"delta_ratio") && !renal.contains(&"odds_ratio"), "{renal:?}");
        let only = groups("", Some(Area::Pancreas), &["meld"], &["apri"]);
        assert_eq!(only.len(), 1, "no pins or recents under a filter");
        assert_eq!(only[0].ids, ["ranson", "bisap", "ctsi", "atlanta_2012"]);
    }

    #[test]
    fn pins_and_recents_lead_the_list_without_repeating() {
        let g = groups("", None, &["meld", "anion_gap"], &["anion_gap", "apri"]);
        assert_eq!(g[0].key, "favourites");
        assert_eq!(g[0].ids, ["meld", "anion_gap"]);
        assert_eq!(g[1].key, "recent");
        assert_eq!(g[1].ids, ["apri"]);
        assert_eq!(g[2].key, "liver");
    }

    #[test]
    fn recents_are_newest_first_unique_and_capped() {
        let mut r = Vec::new();
        for id in ["meld", "apri", "fib4", "saag", "bclc", "meld", "ranson"] {
            r = pushed(&r, id);
        }
        assert_eq!(r, ["ranson", "meld", "bclc", "saag", "fib4"]);
    }

    #[test]
    fn pins_toggle_and_unknown_ids_drop_out() {
        assert_eq!(toggled(&["meld"], "apri"), ["meld", "apri"]);
        assert_eq!(toggled(&["meld", "apri"], "meld"), ["apri"]);
        let saved = Some(vec!["apri".to_owned(), "gone".to_owned(), "apri".to_owned()]);
        assert_eq!(known(saved), ["apri"]);
    }

    #[test]
    fn the_wide_pane_is_never_empty() {
        assert_eq!(fallback(&["saag"], &["meld"]), "saag");
        assert_eq!(fallback(&[], &["meld"]), "meld");
        assert_eq!(fallback(&[], &[]), calc::all()[0].id);
    }

    #[test]
    fn folding_evens_out_arabic_spelling() {
        assert_eq!(fold("الإستسقاء"), fold("الاستسقاء"));
        assert_eq!(fold("كُلَى"), "كلي");
        assert_eq!(fold("٤٨ ساعة"), "48 ساعه");
        assert_eq!(fold("MELD-Na"), "meld-na");
    }
}
