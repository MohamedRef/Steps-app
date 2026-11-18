//! A question's stem on a paper, with the words he highlighted drawn as
//! marks in the theme's `--gold-soft`. Selecting words in it highlights
//! them, or takes the highlight off when they all are (`stem.js` reads the
//! selection, `steps_core::exam::stem` keeps the ranges); the highlights stay
//! with the paper, so a resumed paper and its review keep them.

use dioxus::prelude::*;
use steps_core::exam::{Span, stem};

const STEM_JS: &str = include_str!("stem.js");

/// The stem of question `at` of the paper on screen.
#[component]
pub fn Stem(text: String, highlights: Vec<Span>, at: usize) -> Element {
    rsx! {
        p { class: "q-stem", lang: "en", dir: "ltr", "data-hl": "{at}",
            for (i, (piece, marked)) in stem::pieces(&text, &highlights).into_iter().enumerate() {
                if marked {
                    mark { key: "{i}", class: "q-hl", "{piece}" }
                } else {
                    span { key: "{i}", "{piece}" }
                }
            }
        }
    }
}

/// "3:12:40" from the script: question 3, characters 12 to 40.
fn parse(msg: &str) -> Option<(usize, usize, usize)> {
    let mut parts = msg.split(':').map(|p| p.parse::<usize>().ok());
    let (at, start, end) = (parts.next()??, parts.next()??, parts.next()??);
    (parts.next().is_none() && start < end).then_some((at, start, end))
}

/// While the paper or its review is on screen, a selection in a stem
/// toggles its highlight.
pub fn use_highlights() {
    use_hook(|| {
        spawn(async move {
            let script = format!(
                "{STEM_JS}\nwindow.__stepsStem.start((m) => dioxus.send(m));\nawait new Promise(() => {{}});"
            );
            let mut eval = document::eval(&script);
            while let Ok(msg) = eval.recv::<String>().await {
                if let Some((at, start, end)) = parse(&msg) {
                    super::highlight(at, start, end);
                }
            }
        });
    });
    use_drop(|| {
        document::eval("window.__stepsStem && window.__stepsStem.stop();");
    });
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn messages_name_a_question_and_its_characters() {
        assert_eq!(parse("3:12:40"), Some((3, 12, 40)));
        assert_eq!(parse("0:0:1"), Some((0, 0, 1)));
        assert_eq!(parse("3:12:12"), None);
        assert_eq!(parse("3:40:12"), None);
        assert_eq!(parse("3:12"), None);
        assert_eq!(parse("3:12:40:9"), None);
        assert_eq!(parse("a:1:2"), None);
    }
}
