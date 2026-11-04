//! Synced (LWW) settings that screens keep in globals for speed: the plan,
//! course statuses, sounds, reminders, the bank's finished papers and the
//! pinned calculators. Anything that changes the store from outside this run
//! (a sync from another device, a restore, an import) must call [`reload`],
//! or the next local save would write the stale copy back with a newer
//! timestamp and undo the other device (#32).

/// Re-reads every synced-setting global from the store. Each one only
/// changes (and re-renders) when its value really did.
pub fn reload() {
    crate::plan::reload();
    crate::sounds::reload();
    crate::reminders::reload();
    crate::bank::reload_history();
    crate::learn::reload();
    crate::courses::reload();
    crate::calc::reload();
}
