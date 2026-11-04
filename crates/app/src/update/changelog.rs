//! "What's new": the changes each build brought, from the signed feed's
//! history ([`BuildChanges`], written per pull request in `changes/*.yaml`).
//!
//! Every verified feed's history is merged into `<data>/updates/history.json`,
//! so the sheet can list what an offer brings before it's installed, what
//! the build just installed brought (once, at the first launch after an
//! update), and the whole history in Settings › Updates, offline.
//!
//! Adapted from batna-app main: `crates/app/src/update/changelog.rs` (one
//! language). The layout follows Morphe Manager's update dialog (ideas
//! only, no code).

use std::fs;
use std::path::Path;

use steps_core::update::{BuildChanges, Change, ChangeKind, HISTORY_BUILDS, Version};

/// The cache keeps a little more than one feed carries.
const KEEP: usize = HISTORY_BUILDS + 20;
pub(super) const FILE: &str = "history.json";

/// A build's place in time: version, then build number.
pub fn order(version: &str, build: u64) -> Option<(Version, u64)> {
    Version::parse(version).map(|v| (v, build))
}

fn order_of(b: &BuildChanges) -> Option<(Version, u64)> {
    order(&b.version, b.build)
}

pub(super) fn load(dir: &Path) -> Vec<BuildChanges> {
    fs::read(dir.join(FILE)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// `fresh` (a verified feed's history) over `known`, newest first. Returns
/// None when nothing changed.
pub(super) fn merge(known: &[BuildChanges], fresh: &[BuildChanges]) -> Option<Vec<BuildChanges>> {
    let mut all: Vec<BuildChanges> = fresh.iter().filter(|b| order_of(b).is_some()).cloned().collect();
    for k in known {
        if !all.iter().any(|b| b.key() == k.key()) {
            all.push(k.clone());
        }
    }
    all.sort_by_key(|a| std::cmp::Reverse(order_of(a)));
    all.truncate(KEEP);
    (all != known).then_some(all)
}

/// The builds after `from` up to and including `to`, newest first.
pub fn between(
    history: &[BuildChanges],
    from: Option<(Version, u64)>,
    to: (Version, u64),
) -> Vec<&BuildChanges> {
    history.iter().filter(|b| order_of(b).is_some_and(|o| from.is_none_or(|f| o > f) && o <= to)).collect()
}

/// How many changes of each kind, in [`ChangeKind::ALL`] order, zeros left out.
pub fn tally<'a>(builds: impl IntoIterator<Item = &'a BuildChanges>) -> Vec<(ChangeKind, usize)> {
    let mut n = [0usize; 3];
    for b in builds {
        for c in &b.changes {
            n[kind_index(c.kind)] += 1;
        }
    }
    ChangeKind::ALL.into_iter().zip(n).filter(|(_, n)| *n > 0).collect()
}

fn kind_index(k: ChangeKind) -> usize {
    ChangeKind::ALL.iter().position(|x| *x == k).unwrap_or(0)
}

/// The changes of one kind, in the order written.
pub fn of_kind(b: &BuildChanges, kind: ChangeKind) -> Vec<&Change> {
    b.changes.iter().filter(|c| c.kind == kind).collect()
}

/// A change's line (Egyptian Arabic, else the English fallback), and
/// whether it's Latin only (shown isolated, LTR).
pub fn line(c: &Change) -> (String, bool) {
    let s = [&c.text.ar_eg, &c.text.en].into_iter().map(|s| s.trim()).find(|s| !s.is_empty()).unwrap_or("");
    (s.to_owned(), !s.chars().any(|ch| ('\u{0600}'..='\u{06FF}').contains(&ch)))
}

/// The first `max` lines, new things first, then "+N".
pub fn lines_upto(builds: &[BuildChanges], more: fn(usize) -> String, max: usize) -> String {
    let mut all: Vec<&Change> = builds.iter().flat_map(|b| b.changes.iter()).collect();
    all.sort_by_key(|c| kind_index(c.kind));
    let mut out: Vec<String> = all.iter().take(max).map(|c| format!("• {}", line(c).0)).collect();
    if all.len() > max {
        out.push(more(all.len() - max));
    }
    out.join("\n")
}

/// Days since the epoch for an RFC 3339 date (`2026-10-28T…`).
pub fn day_of(rfc3339: &str) -> Option<i64> {
    let y: i64 = rfc3339.get(0..4)?.parse().ok()?;
    let m: i64 = rfc3339.get(5..7)?.parse().ok()?;
    let d: i64 = rfc3339.get(8..10)?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Howard Hinnant's days_from_civil.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// Year, month and day for days since the epoch (Hinnant's civil_from_days).
pub fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// `0.1.0 (412)` for a preview build, the version alone for a release.
pub fn label(version: &str, build: u64) -> String {
    if build > 0 { format!("{version} ({build})") } else { version.to_owned() }
}

#[cfg(test)]
mod tests {
    use steps_core::update::UpdateNotes;

    use super::*;

    fn b(version: &str, build: u64, kinds: &[ChangeKind]) -> BuildChanges {
        BuildChanges {
            version: version.into(),
            build,
            published_at: "2026-10-28T09:00:00Z".into(),
            changes: kinds
                .iter()
                .enumerate()
                .map(|(i, k)| Change {
                    kind: *k,
                    text: UpdateNotes { en: format!("change {i}"), ar_eg: format!("تغيير {i}") },
                })
                .collect(),
            content_only: false,
        }
    }

    #[test]
    fn merging_keeps_newest_first_without_repeats() {
        use ChangeKind::*;
        let known = vec![b("0.1.0", 5, &[New]), b("0.1.0", 3, &[Fixed])];
        let fresh = vec![b("0.1.0", 7, &[Improved]), b("0.1.0", 5, &[New])];
        let all = merge(&known, &fresh).unwrap();
        assert_eq!(all.iter().map(|x| x.build).collect::<Vec<_>>(), [7, 5, 3]);
        assert_eq!(merge(&all, &fresh), None, "nothing new, nothing to write");
    }

    #[test]
    fn between_is_after_from_up_to_to() {
        use ChangeKind::*;
        let h = vec![b("0.1.0", 9, &[New]), b("0.1.0", 7, &[Fixed]), b("0.1.0", 4, &[Improved])];
        let got = between(&h, order("0.1.0", 4), order("0.1.0", 9).unwrap());
        assert_eq!(got.iter().map(|x| x.build).collect::<Vec<_>>(), [9, 7]);
        assert_eq!(tally(got), [(New, 1), (Fixed, 1)]);
    }

    #[test]
    fn lines_put_new_things_first() {
        use ChangeKind::*;
        let h = [b("0.1.0", 9, &[Fixed, New, Improved, Fixed, New])];
        assert_eq!(lines_upto(&h, |n| format!("و{n} كمان"), 3), "• تغيير 1\n• تغيير 4\n• تغيير 2\nو2 كمان");
        let latin = Change { kind: Fixed, text: UpdateNotes { en: "Bank".into(), ar_eg: String::new() } };
        assert_eq!(line(&latin), ("Bank".into(), true));
    }

    #[test]
    fn days_count_from_the_epoch() {
        let d = day_of("2026-10-28T10:00:00Z").unwrap();
        assert_eq!(d - day_of("2026-10-26T23:59:00Z").unwrap(), 2);
        assert_eq!(civil(d), (2026, 10, 28));
        assert_eq!(day_of("soon"), None);
    }
}
