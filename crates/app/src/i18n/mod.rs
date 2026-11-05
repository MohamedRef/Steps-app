//! UI text, in Egyptian Arabic. One file per feature: a feature adds its
//! own `i18n/<feature>.rs` and one `pub mod <feature>;` line here, and
//! touches nothing else, so parallel work never edits the same lines.
//! Medical terms stay in English (wrap them in `bdi` in the markup).

pub mod app;

/// Counted nouns in Egyptian Arabic, so no screen writes "5 ساعة" or
/// "1 أيام": `count(1, &HOUR)` is "ساعة", 2 is "ساعتين", 3–10 take the
/// plural ("5 ساعات") and 11 and up the singular ("11 ساعة"). Digits are
/// always 0–9.
pub mod count;

/// How every screen cites the book and names a lecture.
pub mod cite;

/// The web app's boot screen (`platform/web.rs`).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod web;

/// A book page's sheet (pagesheet/mod.rs).
pub mod sheet;

/// A lesson step's abbreviations chip and sheet (abbrsheet/mod.rs).
pub mod abbr;

pub mod book;

/// Weak-fact marks (weak/mod.rs): neutral words, no red counts.
pub mod weak;

/// B&B videos and UWorld QIDs per lecture (qbank/mod.rs).
pub mod qbank;

pub mod nav;

pub mod today;

pub mod study_settings;

pub mod study;

pub mod anki;

/// The two optional taps around an answer (`crate::metacog`): how sure he
/// was before it came out, and why he missed it.
pub mod metacog;

pub mod bank;

pub mod stats;

pub mod scores;

pub mod plan;

pub mod table;

pub mod order;

pub mod oral;

pub mod case;

pub mod desk;

pub mod hover;

pub mod gopage;

pub mod catchup;

pub mod keys;

pub mod verify;

pub mod own;

pub mod data;

pub mod cloud;

pub mod settings;

pub mod update;

pub mod sound;

pub mod remind;

pub mod onboard;

pub mod recap;

pub mod learn;

/// The view switcher on list screens (parts::ViewBar).
pub mod views;

pub mod labs;

pub mod courses;

#[cfg(test)]
mod grammar_tests;

pub mod refs;

pub mod lecture;

/// Shared preference write recovery.
pub mod saving;

/// His notes on items, lesson steps and pages (notes/mod.rs).
pub mod notes;
