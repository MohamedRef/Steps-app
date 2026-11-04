//! Mohamed's own cards (PARITY 7.6–7.7): the "add a card" form, the list on
//! the Study screen, and cards shared in from other apps on Android.
//!
//! The cards are one extra deck, `steps_core::own::lecture`, that the study
//! player treats like a book lecture. The review backend publishes it here
//! whenever it reloads, so imports and Steps Cloud changes show up too.

pub(crate) mod form;
pub mod share;

#[cfg(not(test))]
use std::sync::Mutex;

use steps_core::content::{Item, Lecture};
use steps_core::own::OwnCard;

pub use form::{OwnCardsCard, ShareInbox, open_new};

type Deck = Option<(Vec<OwnCard>, &'static Lecture)>;

/// The cards the deck was built from, and the deck.
#[cfg(not(test))]
static DECK: Mutex<Deck> = Mutex::new(None);

// Unit tests: each test thread has its own store, so its own deck.
#[cfg(test)]
thread_local! {
    static DECK: std::cell::RefCell<Deck> = const { std::cell::RefCell::new(None) };
}

#[cfg(not(test))]
fn with_deck<R>(f: impl FnOnce(&mut Deck) -> R) -> R {
    f(&mut DECK.lock().unwrap_or_else(|e| e.into_inner()))
}

#[cfg(test)]
fn with_deck<R>(f: impl FnOnce(&mut Deck) -> R) -> R {
    DECK.with(|d| f(&mut d.borrow_mut()))
}

/// Builds the deck from the store's cards. It is rebuilt only when the cards
/// changed; each build is kept for the rest of the run (a few KB per edit),
/// so screens can hold `&'static` references like they do for book lectures.
pub fn publish(cards: Vec<OwnCard>) -> &'static Lecture {
    with_deck(|deck| {
        if let Some((old, lec)) = deck.as_ref()
            && *old == cards
        {
            return *lec;
        }
        let lec: &'static Lecture = Box::leak(Box::new(steps_core::own::lecture(&cards)));
        *deck = Some((cards, lec));
        lec
    })
}

/// Whether the add-a-card sheet is open (with whatever he has typed).
pub fn drafting() -> bool {
    dioxus::prelude::ReadableExt::peek(&form::DRAFT).is_some()
}

/// The deck, when it has at least one card.
pub fn lecture() -> Option<&'static Lecture> {
    with_deck(|d| d.as_ref().map(|(_, l)| *l)).filter(|l| !l.items.is_empty())
}

/// One of his cards by id.
pub fn find(id: &str) -> Option<(&'static Lecture, &'static Item)> {
    let lec = lecture()?;
    lec.items.iter().find(|i| i.id == id).map(|i| (lec, i))
}

/// The cards as stored (for the list and the edit form).
pub fn cards() -> Vec<OwnCard> {
    with_deck(|d| d.as_ref().map(|(c, _)| c.clone())).unwrap_or_default()
}
