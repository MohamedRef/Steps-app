//! Time spent in another app doesn't count as time on the answer: coming
//! back to the foreground restarts the item's clock. The item and the scroll
//! position stay as they were. Restoring after the process was killed is
//! `checkpoint`'s job. Adapted from batna-app main: crates/app/src/study/foreground.rs.

use dioxus::prelude::*;
use web_time::Instant;

use super::{PHASE, Phase, STARTED};

const FOREGROUND_JS: &str = include_str!("foreground.js");

/// Installed once by the app shell, not per Study mount, so switching tabs
/// never leaves a stale listener behind.
pub(crate) fn use_foreground_reset() {
    use_future(|| async move {
        let mut eval = document::eval(FOREGROUND_JS);
        while let Ok(message) = eval.recv::<String>().await {
            if message == "visible" {
                restart_clock();
            }
        }
    });
}

/// Restarts the answer clock, only while a session runs.
pub(super) fn restart_clock() {
    if *PHASE.peek() == Phase::Running {
        *STARTED.write() = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::Screen;
    use crate::render_tests::App;
    use crate::render_tests::kit::ids_of;
    use crate::study::{START, Start};

    #[test]
    fn coming_back_restarts_the_answer_clock_only_while_a_session_runs() {
        let hour = Duration::from_secs(3600);
        let mut app = App::open(Screen::Study);
        // Nothing running: the clock is left alone.
        let idle = app.run(|| {
            *STARTED.write() = Instant::now() - hour;
            restart_clock();
            STARTED.peek().elapsed()
        });
        assert!(idle >= hour);
        let ids = ids_of("card").into_iter().take(2).collect::<Vec<_>>();
        app.run(move || *START.write() = Some(Start { lectures: None, items: Some(ids), minutes: 5 }));
        let back = app.run(|| {
            *STARTED.write() = Instant::now() - hour;
            restart_clock();
            STARTED.peek().elapsed()
        });
        assert!(back < Duration::from_secs(60), "an hour in another app is not answer time");
    }
}
