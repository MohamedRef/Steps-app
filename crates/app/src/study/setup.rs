//! Study session setup and duration choices.

use super::*;

#[component]
pub(super) fn Setup(
    ready: Vec<(String, String)>,
    chosen: Signal<BTreeSet<String>>,
    topics: Signal<HashSet<String>>,
    minutes: Signal<u32>,
    preview_count: usize,
    preview_new: usize,
    preview_mins: u32,
    elsewhere: usize,
    on_start: EventHandler<()>,
) -> Element {
    // Topic chips only when one lecture is picked: across lectures they'd be a wall.
    let single = {
        let c = chosen.read();
        if c.len() == 1 { c.iter().next().and_then(|id| Library::get().lectures.get(id)) } else { None }
    };
    let reviews = preview_count - preview_new;
    let all_ids: Vec<String> = ready.iter().map(|(id, _)| id.clone()).collect();
    let all_chosen = chosen.read().len() == all_ids.len();
    rsx! {
        crate::ui::Card { class: "st-setup",
            // Tablet landscape and the Mac: the time and Start stay in view
            // beside the lecture and topic picker.
            div { class: "tab-cols", "data-wide": "aside",
                div { class: "tab-main tab-sticky st-setup-go",
                    crate::courses::CourseChip {}
                    h1 { {tx::TITLE} }

                    h2 { {tx::TIME} }
                    div { class: "segmented", role: "radiogroup", aria_label: tx::TIME,
                        for m in MINUTES {
                            button {
                                key: "{m}",
                                role: "radio",
                                "aria-checked": if minutes() == m { "true" } else { "false" },
                                onclick: move |_| minutes.set(m),
                                {count(m, &MINUTE)}
                            }
                        }
                    }

                    if elsewhere > 0 {
                        p { class: "note small st-elsewhere",
                            {tx::due_elsewhere(elsewhere)}
                            " "
                            button { class: "btn-link", onclick: move |_| crate::courses::focus::set(None), {tx::ELSEWHERE_ALL} }
                        }
                    }
                    if preview_count == 0 {
                        p { class: "note", {tx::NOTHING_NOW} }
                    } else {
                        p { class: "st-preview",
                            {count(preview_count, &ITEM)} " · " {count(reviews, &REVIEW)} " · {preview_new} {tx::NEW} · {tx::ABOUT} " {count(preview_mins, &MINUTE)}
                        }
                        button { class: "btn btn-primary btn-lg st-start", onclick: move |_| on_start.call(()), {tx::START} }
                    }
                }
                div { class: "tab-aside st-setup-pick",
                    div { class: "st-section-head",
                        h2 { {tx::LECTURES} }
                        button {
                            class: "btn-link",
                            onclick: move |_| {
                                let mut c = chosen.write();
                                if c.len() == all_ids.len() {
                                    c.clear();
                                } else {
                                    *c = all_ids.iter().cloned().collect();
                                }
                                topics.write().clear();
                            },
                            {if all_chosen { tx::CLEAR_ALL } else { tx::PICK_ALL }}
                        }
                    }
                    for group in Library::get().by_course(ready, |(id, _)| id.as_str()) {
                        h3 { key: "{group.id}", class: "label course-label",
                            if group.id == steps_core::program::course_of(steps_core::own::LECTURE) { {crate::i18n::own::TITLE} } else { "{group.short()}" }
                        }
                        div { class: "st-chips st-lectures",
                            for (id , title) in group.rows {
                                button {
                                    key: "{id}",
                                    class: "st-chip",
                                    "aria-pressed": if chosen.read().contains(&id) { "true" } else { "false" },
                                    onclick: {
                                        let id = id.clone();
                                        move |_| {
                                            let mut c = chosen.write();
                                            if !c.remove(&id) {
                                                c.insert(id.clone());
                                            }
                                            topics.write().clear();
                                        }
                                    },
                                    span { class: "st-chip-num",
                                        if id == steps_core::own::LECTURE { "✎" } else { {crate::content::lecture_no(&id)} }
                                    }
                                    bdi { "{title}" }
                                }
                            }
                        }
                    }

                    if let Some(lecture) = single {
                        if !lecture.topics.is_empty() {
                            h2 { {tx::TOPICS} }
                            div { class: "st-chips",
                                for t in lecture.topics.iter() {
                                    {
                                        let key = format!("{}/{}", lecture.lecture, t.id);
                                        let on = topics.read().contains(&key);
                                        rsx! {
                                            button {
                                                key: "{key}",
                                                class: "st-chip",
                                                "aria-pressed": if on { "true" } else { "false" },
                                                onclick: move |_| {
                                                    let mut s = topics.write();
                                                    if !s.remove(&key) {
                                                        s.insert(key.clone());
                                                    }
                                                },
                                                bdi { "{t.title}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
