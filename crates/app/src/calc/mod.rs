//! Calculators: the generic form that any calculator in `steps_core::calc`
//! renders into, and the result card. The screen that lists them is in
//! [`picker`]; the `calc` study item lives in [`item`].

pub mod item;
mod picker;

pub use picker::{Calculators, reload};

use std::collections::BTreeMap;

use dioxus::prelude::*;
use steps_core::calc::{self, Area, CalcError, CalcSpec, CalcValue, FieldKind, Inputs, Outcome};

use crate::i18n::calc as tx;
use crate::look::{Css, css};

pub const CSS: Css = css!("src/calc/calc.css");

fn area_label(area: Area) -> &'static str {
    match area {
        Area::Liver => tx::AREA_LIVER,
        Area::Ascites => tx::AREA_ASCITES,
        Area::Tumour => tx::AREA_TUMOUR,
        Area::AcuteFailure => tx::AREA_ACUTE,
        Area::Pancreas => tx::AREA_PANCREAS,
        Area::Gi => tx::AREA_GI,
        Area::Renal => tx::AREA_RENAL,
        Area::Elderly => tx::AREA_ELDERLY,
        Area::Rheum => tx::AREA_RHEUM,
        Area::Diabetes => tx::AREA_DIABETES,
        Area::Blood => tx::AREA_BLOOD,
        Area::Research => tx::AREA_RESEARCH,
    }
}

/// The course whose book a calculator's page cites, so its page chip opens
/// that book (each course's pages start at 1).
fn area_course(area: Area) -> &'static str {
    match area {
        Area::Gi => "git",
        Area::Renal => "neph",
        Area::Elderly => "geri",
        Area::Rheum => "rheum",
        Area::Diabetes => "dm",
        Area::Blood => "hem",
        Area::Research => "res",
        _ => "hb",
    }
}

const AREAS: [Area; 12] = [
    Area::Liver,
    Area::Ascites,
    Area::Tumour,
    Area::AcuteFailure,
    Area::Pancreas,
    Area::Gi,
    Area::Renal,
    Area::Elderly,
    Area::Rheum,
    Area::Diabetes,
    Area::Blood,
    Area::Research,
];

/// The text a field shows for a value.
fn field_text(v: Option<&CalcValue>) -> String {
    match v {
        Some(CalcValue::Number(n)) => trim_num(*n),
        Some(CalcValue::Text(s)) => s.clone(),
        Some(CalcValue::Bool(b)) => if *b { "yes" } else { "no" }.to_owned(),
        None => String::new(),
    }
}

pub fn trim_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e12 {
        format!("{n:.0}")
    } else {
        format!("{n}").trim_end_matches('0').to_owned()
    }
}

/// A form for one calculator with a live result, laid out like MDCalc's: the
/// name and what it is for, one row per input (its name, then its options as
/// buttons, or a number box with the unit inside), and the result in a bar
/// that stays at the foot of the screen while he fills it in, with what it
/// means under it. `initial` pre-fills it (a `calc` item's inputs); every
/// field stays editable.
#[component]
pub fn CalcForm(id: &'static str, initial: Inputs) -> Element {
    let Some(spec) = calc::spec(id) else { return rsx! {} };
    let mut values = use_signal(|| initial.0.clone());
    let inputs = Inputs(
        values.read().iter().filter(|(_, v)| !is_blank(v)).map(|(k, v)| (k.clone(), v.clone())).collect(),
    );
    let result = (spec.compute)(&inputs);
    // The field the engine rejected, marked in place (a missing one is not
    // an error yet: the result bar counts what is left).
    let bad = match &result {
        Err(CalcError::Invalid { field, .. }) => Some(*field),
        _ => None,
    };
    let left = missing(spec, &inputs);
    rsx! {
        style { {CSS.text()} }
        div { class: "calc-form",
            crate::ui::Card { class: "calc-fields",
                header { class: "calc-title",
                    h1 { bdi { {spec.name} } }
                    if let Some(line) = tx::blurb(spec.id) {
                        p { class: "calc-intro muted", {line} }
                    }
                }
                div { class: "calc-inputs",
                    for field in spec.fields {
                        div {
                            key: "{field.key}",
                            class: "field calc-field",
                            "data-invalid": if bad == Some(field.key) { "true" } else { "false" },
                            div { class: "calc-field-head",
                                // Name and "optional" as separate pieces, so the
                                // English name and the Arabic tag never mix.
                                if matches!(field.kind, FieldKind::Number { .. }) {
                                    label { class: "calc-label", id: "l-{field.key}", r#for: "f-{field.key}",
                                        bdi { {field.label} }
                                        if field.optional { span { class: "chip", {tx::OPTIONAL} } }
                                    }
                                } else {
                                    span { class: "calc-label", id: "l-{field.key}",
                                        bdi { {field.label} }
                                        if field.optional && !matches!(field.kind, FieldKind::Flag) {
                                            span { class: "chip", {tx::OPTIONAL} }
                                        }
                                    }
                                }
                                match tx::hint(spec.id, field.key) {
                                    Some(h) => rsx! { p { class: "field-hint calc-hint", {h} } },
                                    None if !field.hint.is_empty() => rsx! { bdi { class: "field-hint calc-hint", {field.hint} } },
                                    None => rsx! {},
                                }
                            }
                            div { class: "calc-control",
                                {field_input(field, values, bad == Some(field.key))}
                                if bad == Some(field.key) {
                                    p { id: "e-{field.key}", class: "field-error", role: "alert", {tx::INVALID} }
                                }
                            }
                        }
                    }
                }
                div { class: "calc-foot",
                    button { class: "btn-link", onclick: move |_| values.set(BTreeMap::new()), {tx::RESET} }
                }
            }
            match result {
                Ok(o) => rsx! {
                    div { class: "calc-bar-result", "data-state": "ready", "aria-live": "polite",
                        span { class: "calc-bar-label", {tx::RESULT} }
                        bdi { class: "calc-display", "{o.display}" }
                        bdi { class: "calc-bar-band", {o.label} }
                    }
                    OutcomeCard { id, outcome: o, headline: false }
                },
                Err(e) => {
                    let (what, label) = error_text(spec, &e);
                    rsx! {
                        div { class: "calc-bar-result", "data-state": "waiting", "aria-live": "polite",
                            span { class: "calc-bar-label", {tx::RESULT} }
                            span { class: "calc-bar-wait",
                                if left > 0 { {tx::left(left)} } else { {tx::FILL} }
                            }
                            span { class: "calc-bar-next", "{what}: " bdi { "{label}" } }
                        }
                    }
                }
            }
        }
    }
}

fn is_blank(v: &CalcValue) -> bool {
    matches!(v, CalcValue::Text(s) if s.trim().is_empty())
}

/// How many required fields are still empty.
fn missing(spec: &CalcSpec, inputs: &Inputs) -> usize {
    spec.fields
        .iter()
        .filter(|f| !f.optional && !matches!(f.kind, FieldKind::Flag) && !inputs.0.contains_key(f.key))
        .count()
}

fn field_input(
    field: &'static calc::Field,
    mut values: Signal<BTreeMap<String, CalcValue>>,
    invalid: bool,
) -> Element {
    let key = field.key;
    let invalid = if invalid { "true" } else { "false" };
    let current = values.read().get(key).cloned();
    match field.kind {
        FieldKind::Number { unit, min, max, step } => rsx! {
            // The unit sits inside the box, at its end, as on MDCalc.
            div { class: "calc-num", dir: "ltr",
                input {
                    id: "f-{key}",
                    r#type: "number",
                    inputmode: "decimal",
                    dir: "ltr",
                    min: "{min}",
                    max: "{max}",
                    step: "{step}",
                    "aria-invalid": invalid,
                    "aria-describedby": "e-{key}",
                    value: field_text(current.as_ref()),
                    oninput: move |e: FormEvent| {
                        values.write().insert(key.to_owned(), CalcValue::Text(e.value()));
                    },
                }
                if !unit.is_empty() {
                    bdi { class: "calc-unit", dir: "ltr", "{unit}" }
                }
            }
        },
        FieldKind::Flag => {
            let on = matches!(current, Some(CalcValue::Bool(true)))
                || matches!(&current, Some(CalcValue::Text(s)) if s == "yes" || s == "true");
            rsx! {
                div { class: "calc-options", role: "group", aria_labelledby: "l-{key}",
                    button {
                        class: "calc-opt",
                        "aria-pressed": if on { "false" } else { "true" },
                        onclick: move |_| { values.write().insert(key.to_owned(), CalcValue::Bool(false)); },
                        {tx::NO}
                    }
                    button {
                        class: "calc-opt",
                        "aria-pressed": if on { "true" } else { "false" },
                        onclick: move |_| { values.write().insert(key.to_owned(), CalcValue::Bool(true)); },
                        {tx::YES}
                    }
                }
            }
        }
        FieldKind::Choice(options) => {
            let text = field_text(current.as_ref());
            let known = options.iter().any(|(id, _)| *id == text);
            // As on MDCalc: a pair (Man / Woman) shares a line; three or more,
            // or a long label ("< 0.5 mL/kg/h for 6–12 h"), stack one per line.
            let stack = options.len() > 2 || options.iter().any(|(_, label)| label.chars().count() > 18);
            let alias = text.clone();
            rsx! {
                div {
                    class: "calc-options",
                    "data-stack": if stack { "true" } else { "false" },
                    role: "group",
                    aria_labelledby: "l-{key}",
                    // A content alias ("mild") the list doesn't spell the same way.
                    if !known && !text.is_empty() {
                        button {
                            class: "calc-opt",
                            "aria-pressed": "true",
                            onclick: move |_| {
                                values.write().insert(key.to_owned(), CalcValue::Text(alias.clone()));
                            },
                            bdi { dir: "ltr", "{text}" }
                        }
                    }
                    for (id, label) in options.iter() {
                        button {
                            key: "{id}",
                            class: "calc-opt",
                            "aria-pressed": if text == *id { "true" } else { "false" },
                            onclick: move |_| {
                                values.write().insert(key.to_owned(), CalcValue::Text((*id).to_owned()));
                            },
                            bdi { dir: "ltr", {*label} }
                        }
                    }
                }
            }
        }
    }
}

/// What is missing or wrong, and the field's label.
fn error_text(spec: &CalcSpec, e: &CalcError) -> (&'static str, String) {
    let label = |k: &str| spec.fields.iter().find(|f| f.key == k).map_or(k, |f| f.label).to_owned();
    match e {
        CalcError::Missing(k) => (tx::MISSING, label(k)),
        CalcError::Invalid { field, .. } => (tx::INVALID, label(field)),
        CalcError::Unknown(id) => (tx::INVALID, id.clone()),
    }
}

/// The result: the number or stage, its band and meaning, the breakdown,
/// and where it comes from.
///
/// The calculator form shows the number in its own bar (`headline: false`)
/// and this card under it as the interpretation.
#[component]
pub fn OutcomeCard(id: &'static str, outcome: Outcome, #[props(default = true)] headline: bool) -> Element {
    let source = calc::spec(id).map_or("", |s| s.source);
    let explain = if headline { "" } else { " calc-explain" };
    rsx! {
        crate::ui::Card {
            class: "calc-result{explain}",
            "aria-live": if headline { "polite" } else { "off" },
            if headline {
                p { class: "small muted", {tx::RESULT} }
                div { class: "calc-headline",
                    bdi { class: "calc-display", "{outcome.display}" }
                    bdi { class: "chip calc-band", "data-tone": "accent", {outcome.label} }
                }
            } else {
                h2 { class: "calc-result-head", {tx::MEANS} }
            }
            bdi { class: "calc-meaning", "{outcome.meaning}" }
            details { class: "calc-parts",
                summary { {tx::BREAKDOWN} }
                ul {
                    for part in outcome.parts.iter() {
                        li { key: "{part.label}",
                            bdi { "{part.label}" }
                            span { class: "calc-points",
                                match part.met {
                                    Some(true) => "✓".to_owned(),
                                    Some(false) => "—".to_owned(),
                                    None => format!("{:.2}", part.points).trim_end_matches('0').trim_end_matches('.').to_owned(),
                                }
                            }
                        }
                    }
                }
            }
            for note in outcome.notes.iter() {
                bdi { key: "{note}", class: "small calc-note", "{note}" }
            }
            // What to do next and what else it could be, when written.
            if let Some(guide) = calc::guide(outcome.band) {
                if !guide.next.is_empty() {
                    div { class: "calc-guide calc-next",
                        h2 { class: "calc-result-head", {tx::NEXT_STEP} }
                        // English medical text: bullets on its own (left) side.
                        ul { dir: "ltr",
                            for line in guide.next.iter() {
                                li { key: "{line}", "{line}" }
                            }
                        }
                    }
                }
                if !guide.dd.is_empty() {
                    div { class: "calc-guide calc-dd",
                        h2 { class: "calc-result-head", {tx::DD} }
                        ul { dir: "ltr",
                            for line in guide.dd.iter() {
                                li { key: "{line}", "{line}" }
                            }
                        }
                    }
                }
            }
            // The book page opens its sheet, like every "ص N" chip.
            p { class: "small muted calc-source",
                span { {tx::SOURCE} ": " bdi { "{source}" } }
                if let Some(page) = outcome.page {
                    crate::hover::BookPage { page, quote: None, of: calc::spec(id).map_or("hb", |s| area_course(s.area)).to_string() }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_counts_required_fields_only() {
        let spec = calc::spec("child_pugh").unwrap();
        let required =
            spec.fields.iter().filter(|f| !f.optional && !matches!(f.kind, FieldKind::Flag)).count();
        assert_eq!(missing(spec, &Inputs::new()), required);
        let first = spec.fields.iter().find(|f| !f.optional).unwrap();
        let mut one = Inputs::new();
        one.0.insert(first.key.to_owned(), CalcValue::Number(1.0));
        assert_eq!(missing(spec, &one), required - 1);
    }

    // Issue #110: hints show in Arabic; core keeps the English ones.
    #[test]
    fn every_field_hint_has_arabic() {
        for spec in calc::all() {
            for f in spec.fields.iter().filter(|f| !f.hint.is_empty()) {
                assert!(tx::hint(spec.id, f.key).is_some(), "{}.{}", spec.id, f.key);
            }
        }
    }
}
