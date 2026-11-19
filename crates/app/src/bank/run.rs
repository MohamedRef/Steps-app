//! A paper on screen in the app's one scroller (`run.js`): the sticky top
//! bar's height for the scroller's scroll-padding, the verdict brought into
//! view after a check, focus that never scrolls the page, and sheets that
//! take focus when they open and give it back when they close (UX-01,
//! UX-35). Each call carries the script, so the order they run in never
//! matters.

use dioxus::prelude::*;

const RUN_JS: &str = include_str!("run.js");

fn run(call: &str) {
    document::eval(&format!("{RUN_JS}\n{call}"));
}

/// The paper's (or its review's) top bar is on screen: keep measuring it.
pub fn watch() {
    run("window.__stepsRun.watch();");
}

/// The paper left the screen.
pub fn unwatch() {
    document::eval("window.__stepsRun && window.__stepsRun.stop && window.__stepsRun.stop();");
}

/// The verdict over the answered question, in full below the bar.
pub fn verdict() {
    run("window.__stepsRun.verdict();");
}

/// Focus the first visible match of `selector` without scrolling the page.
pub fn focus(selector: &str) {
    run(&format!("window.__stepsRun.focus({selector:?});"));
}

/// Sheet `key` opened: focus `selector` in it, remembering the opener.
pub fn opened(key: &str, selector: &str) {
    run(&format!("window.__stepsRun.opened({key:?}, {selector:?});"));
}

/// Sheet `key` closed: focus back to its opener, or to `fallback`.
pub fn closed(key: &str, fallback: &str) {
    run(&format!("window.__stepsRun.closed({key:?}, {fallback:?});"));
}

/// A sheet over the paper: focus moves into it when it opens and back to
/// what opened it when it goes, whichever way it closes.
pub fn use_sheet_focus(key: &'static str, target: &'static str, fallback: &'static str) {
    use_hook(move || opened(key, target));
    use_drop(move || closed(key, fallback));
}

#[cfg(test)]
mod tests {
    use super::RUN_JS;

    #[test]
    fn the_script_keeps_focus_and_scroll_quiet() {
        // Focus never scrolls the page, and nothing here presses a button.
        assert_eq!(RUN_JS.matches(".focus(").count(), RUN_JS.matches("preventScroll: true").count());
        assert!(!RUN_JS.contains(".click()"));
        // The verdict moves only when part of it is hidden.
        assert!(RUN_JS.contains("if (r.top >= top && r.bottom <= bottom) return;"));
        // A sheet never takes focus back from a field or sheet that took it.
        assert!(RUN_JS.contains("if (!loose()) return;"));
    }
}
