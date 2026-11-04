//! App-wide reactive state and shared update paths.

use crate::{Screen, prefs};
use dioxus::prelude::*;

/// App-wide UI state, shared through context.
#[derive(Clone, Copy)]
pub struct Ui {
    pub screen: Signal<Screen>,
    /// This device's theme and text size (Settings › Look).
    pub prefs: Signal<prefs::DevicePrefs>,
    /// A focused study task is open: the phone bar hides
    /// (`parts::use_focus_mode`).
    pub focus: Signal<bool>,
}

impl Ui {
    /// One update-and-save path for every device preference control.
    pub fn update_prefs(mut self, update: impl FnOnce(&mut prefs::DevicePrefs)) {
        let mut next = *self.prefs.peek();
        update(&mut next);
        self.prefs.set(next);
        next.save();
    }
}
