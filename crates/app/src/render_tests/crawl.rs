//! Every button on every screen, and what each one opens: none of them may
//! panic or leave the app without a screen.

use super::App;
use crate::Screen;

fn open(screen: Screen) -> App {
    let mut app = App::open(Screen::Today);
    if screen == Screen::Settings {
        app.tap(Screen::Settings.label());
    } else {
        app.tap(screen.label());
    }
    app
}

/// Runs `f` on a thread of its own: each test thread has its own data
/// (`prefs::test_dir`, the in-memory store), so `f` starts from an empty app
/// whatever earlier taps saved.
fn fresh<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    std::thread::Builder::new()
        .stack_size(16 << 20)
        .spawn(f)
        .expect("a crawl thread starts")
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e))
}

/// At most this many buttons of one kind (the same class) are crawled per
/// screen. Lists drawn from the content repeat one kind of button per row
/// (Stats had a topic link for each of ~1,400 topics, Learn a row per
/// lecture), so crawling every row made these tests grow with every course
/// written (#111); a few rows of each kind prove the kind works.
const PER_KIND: usize = 6;

/// `sigs` with at most [`PER_KIND`] of each class, in order.
fn per_kind(sigs: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashMap::<String, usize>::new();
    sigs.into_iter()
        .filter(|sig| {
            let kind = sig.split('|').nth(1).unwrap_or_default().to_owned();
            let n = seen.entry(kind).or_default();
            *n += 1;
            *n <= PER_KIND
        })
        .collect()
}

fn crawl(screen: Screen) {
    let first = fresh(move || per_kind(open(screen).tappables()));
    // Every button starts from a fresh app. Taps saved by an earlier button
    // (Today's session-length chips change the hero's label) would otherwise
    // rename the next one, and the crawl would skip it. Fresh apps share
    // nothing, so the buttons are crawled on several threads at once: one
    // after another, Stats alone took about 4 minutes and was the last test
    // the CI run waited for.
    let workers = std::thread::available_parallelism().map_or(2, |n| n.get()).clamp(2, 8);
    let queue = std::sync::Mutex::new(first.into_iter());
    let missed = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let Some(sig) = queue.lock().unwrap_or_else(|e| e.into_inner()).next() else {
                        break;
                    };
                    let tapped = fresh({
                        let sig = sig.clone();
                        move || crawl_from(screen, &sig)
                    });
                    if !tapped {
                        missed.lock().unwrap_or_else(|e| e.into_inner()).push(sig);
                    }
                }
            });
        }
    });
    let mut missed = missed.into_inner().unwrap_or_else(|e| e.into_inner());
    missed.sort();
    assert!(missed.is_empty(), "{screen:?}: a fresh app did not show {missed:#?}");
}

/// Taps `sig` on `screen`, then what it brought up, two levels deep.
/// False when a fresh app does not show `sig`.
fn crawl_from(screen: Screen, sig: &str) -> bool {
    let mut app = open(screen);
    let before = app.tappables();
    if !app.tap_signature(sig) {
        return false;
    }
    // Then everything the tap brought up, in turn, in the same app.
    let new: Vec<String> =
        per_kind(app.tappables().into_iter().filter(|s| !before.contains(s))).into_iter().take(25).collect();
    for next in new {
        let seen = app.tappables();
        if app.tap_signature(&next) {
            for third in
                per_kind(app.tappables().into_iter().filter(|s| !seen.contains(s))).into_iter().take(6)
            {
                app.tap_signature(&third);
            }
        }
    }
    assert!(app.html().contains("data-screen="), "{screen:?} after {sig}");
    true
}

#[test]
fn per_kind_keeps_a_few_of_each_kind() {
    let rows = (0..20).map(|i| format!("|tap topic-link|Topic {i}"));
    let tabs = ["|tab|A", "|tab|B"].map(String::from);
    let kept = per_kind(tabs.clone().into_iter().chain(rows));
    assert_eq!(kept.len(), 2 + PER_KIND);
    assert_eq!(kept[..2], tabs);
    assert_eq!(kept.last().map(String::as_str), Some("|tap topic-link|Topic 5"));
}

#[test]
fn crawl_today() {
    crawl(Screen::Today);
}
#[test]
fn crawl_learn() {
    crawl(Screen::Learn);
}
#[test]
fn crawl_study() {
    crawl(Screen::Study);
}
#[test]
fn crawl_bank() {
    crawl(Screen::Bank);
}
#[test]
fn crawl_cases() {
    crawl(Screen::Cases);
}
#[test]
fn crawl_oral() {
    crawl(Screen::Oral);
}
#[test]
fn crawl_stats() {
    crawl(Screen::Stats);
}
#[test]
fn crawl_settings() {
    crawl(Screen::Settings);
}
