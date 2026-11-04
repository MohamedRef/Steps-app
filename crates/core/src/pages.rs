//! First Aid pages Mohamed has read, and the gate that keeps new items
//! locked until their page is read (feature list A2, after AnKing's
//! "unsuspend the pages you studied").
//!
//! Pages are First Aid's printed numbers. Reviews are never gated: only
//! an item's first look waits for its page.

use serde::{Deserialize, Serialize};

use crate::content::{BookRef, Item};

/// The First Aid source id in `content/sources.yaml`.
pub const FIRST_AID: &str = "fa2026";

/// A set of printed pages, kept as sorted, disjoint, non-touching ranges
/// (`[[12, 15], [20, 20]]`), which is also how it is stored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Vec<[u32; 2]>", into = "Vec<[u32; 2]>")]
pub struct PagesRead {
    ranges: Vec<[u32; 2]>,
}

impl From<Vec<[u32; 2]>> for PagesRead {
    fn from(ranges: Vec<[u32; 2]>) -> Self {
        let mut out = Self::default();
        for [a, b] in ranges {
            out.mark(a, b);
        }
        out
    }
}

impl From<PagesRead> for Vec<[u32; 2]> {
    fn from(p: PagesRead) -> Self {
        p.ranges
    }
}

impl PagesRead {
    pub fn ranges(&self) -> &[[u32; 2]] {
        &self.ranges
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub fn contains(&self, page: u32) -> bool {
        self.ranges.iter().any(|[a, b]| (*a..=*b).contains(&page))
    }

    /// How many of the pages `first..=last` are read.
    pub fn count_in(&self, first: u32, last: u32) -> u32 {
        let (first, last) = (first.min(last), first.max(last));
        self.ranges
            .iter()
            .map(|[a, b]| {
                let (lo, hi) = ((*a).max(first), (*b).min(last));
                if lo <= hi { hi - lo + 1 } else { 0 }
            })
            .sum()
    }

    /// The read ranges cut to `first..=last` ("what of this lecture is read").
    pub fn within(&self, first: u32, last: u32) -> Vec<[u32; 2]> {
        let (first, last) = (first.min(last), first.max(last));
        self.ranges
            .iter()
            .filter_map(|[a, b]| {
                let (lo, hi) = ((*a).max(first), (*b).min(last));
                (lo <= hi).then_some([lo, hi])
            })
            .collect()
    }

    /// Whether any page of `first..=last` is read.
    pub fn any_in(&self, first: u32, last: u32) -> bool {
        self.count_in(first, last) > 0
    }

    /// Marks `first..=last` read (either order).
    pub fn mark(&mut self, first: u32, last: u32) {
        let (mut lo, mut hi) = (first.min(last), first.max(last));
        let mut kept = Vec::with_capacity(self.ranges.len() + 1);
        for &[a, b] in &self.ranges {
            // Overlapping or touching ranges merge into the new one.
            if b.saturating_add(1) < lo || hi.saturating_add(1) < a {
                kept.push([a, b]);
            } else {
                lo = lo.min(a);
                hi = hi.max(b);
            }
        }
        kept.push([lo, hi]);
        kept.sort_unstable();
        self.ranges = kept;
    }

    /// Marks `first..=last` unread (either order).
    pub fn unmark(&mut self, first: u32, last: u32) {
        let (lo, hi) = (first.min(last), first.max(last));
        let mut kept = Vec::with_capacity(self.ranges.len() + 1);
        for &[a, b] in &self.ranges {
            if b < lo || hi < a {
                kept.push([a, b]);
                continue;
            }
            if a < lo {
                kept.push([a, lo - 1]);
            }
            if hi < b {
                kept.push([hi + 1, b]);
            }
        }
        self.ranges = kept;
    }

    /// Adds every page of `other`.
    pub fn extend(&mut self, other: &PagesRead) {
        for &[a, b] in &other.ranges {
            self.mark(a, b);
        }
    }
}

/// Whether the gate lets `item` be introduced. A First Aid item waits for
/// its own page. An item from another source (Pathoma, Sketchy…) cites
/// that source's page, so it waits for any page of its First Aid lecture
/// (`lecture`) instead.
pub fn unlocks(item: &Item, lecture: &BookRef, read: &PagesRead) -> bool {
    match item.src.book.as_deref() {
        None | Some(FIRST_AID) => read.contains(item.src.page),
        Some(_) => read.any_in(lecture.pages[0], lecture.pages[1]),
    }
}

/// "pp. 12–15, 20" for a set of ranges.
pub fn label(ranges: &[[u32; 2]]) -> String {
    ranges
        .iter()
        .map(|[a, b]| if a == b { a.to_string() } else { format!("{a}–{b}") })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Body, Source, Status};

    fn item(page: u32, book: Option<&str>) -> Item {
        Item {
            id: format!("cvs.L04.{page:03}"),
            body: Body::Card { front: "f".into(), back: "b".into() },
            topic: None,
            src: Source { book: book.map(str::to_string), page, quote: None },
            status: Status::Unverified,
            update: None,
            tags: Vec::new(),
        }
    }

    #[test]
    fn mark_merges_overlapping_and_touching_ranges() {
        let mut p = PagesRead::default();
        p.mark(20, 22);
        p.mark(10, 12);
        p.mark(13, 14);
        assert_eq!(p.ranges(), &[[10, 14], [20, 22]]);
        p.mark(15, 19);
        assert_eq!(p.ranges(), &[[10, 22]]);
        p.mark(30, 28);
        assert_eq!(p.ranges(), &[[10, 22], [28, 30]]);
    }

    #[test]
    fn unmark_splits_and_trims() {
        let mut p = PagesRead::from(vec![[10, 20]]);
        p.unmark(14, 15);
        assert_eq!(p.ranges(), &[[10, 13], [16, 20]]);
        p.unmark(9, 10);
        assert_eq!(p.ranges(), &[[11, 13], [16, 20]]);
        p.unmark(20, 16);
        assert_eq!(p.ranges(), &[[11, 13]]);
        p.unmark(1, 100);
        assert!(p.is_empty());
    }

    #[test]
    fn counts_and_contains() {
        let p = PagesRead::from(vec![[10, 12], [20, 20]]);
        assert!(p.contains(11) && p.contains(20) && !p.contains(13));
        assert_eq!(p.count_in(11, 25), 3);
        assert_eq!(p.count_in(25, 11), 3);
        assert_eq!(p.count_in(13, 19), 0);
        assert!(p.any_in(1, 10) && !p.any_in(21, 30));
        assert_eq!(p.within(11, 30), [[11, 12], [20, 20]]);
        assert!(p.within(13, 19).is_empty());
    }

    #[test]
    fn stored_form_is_the_ranges_and_normalises_on_read() {
        let p = PagesRead::from(vec![[10, 12], [20, 20]]);
        assert_eq!(serde_json::to_string(&p).unwrap(), "[[10,12],[20,20]]");
        let q: PagesRead = serde_json::from_str("[[20,20],[11,13],[10,10]]").unwrap();
        assert_eq!(q.ranges(), &[[10, 13], [20, 20]]);
    }

    #[test]
    fn extend_unions() {
        let mut p = PagesRead::from(vec![[1, 2]]);
        p.extend(&PagesRead::from(vec![[3, 4], [9, 9]]));
        assert_eq!(p.ranges(), &[[1, 4], [9, 9]]);
    }

    #[test]
    fn first_aid_items_wait_for_their_page_other_sources_for_the_lecture() {
        let lecture = BookRef { chapter: 2, pages: [300, 320] };
        let read = PagesRead::from(vec![[305, 305]]);
        assert!(unlocks(&item(305, None), &lecture, &read));
        assert!(unlocks(&item(305, Some(FIRST_AID)), &lecture, &read));
        assert!(!unlocks(&item(306, None), &lecture, &read));
        // A Pathoma page number says nothing about First Aid pages.
        assert!(unlocks(&item(42, Some("pathoma")), &lecture, &read));
        assert!(!unlocks(&item(42, Some("pathoma")), &lecture, &PagesRead::from(vec![[1, 299]])));
    }

    #[test]
    fn label_reads_like_the_book() {
        assert_eq!(label(&[[10, 12], [20, 20]]), "10–12, 20");
        assert_eq!(label(&[]), "");
    }
}
