//! `steps-tools ids`: the content ids Mohamed's records point at
//! (`steps_core::ids`, `docs/CONTENT-IDS.md`).
//!
//! - `ids check --base REF` compares the working tree's content with the
//!   content at that git commit (CI: `HEAD^1`), and fails on a removed or
//!   repurposed id that `content/ids-retired.txt` doesn't name.
//! - `ids check --base-pack FILE` compares with a published content pack.
//! - `ids list` prints every id and what it hangs under.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use steps_core::ids::{Registry, compare, parse_retired, registry};
use steps_core::pack::Pack;

pub const RETIRED: &str = "content/ids-retired.txt";

type Res<T> = Result<T, Vec<String>>;

pub fn run(root: &Path, args: &[String]) -> Res<String> {
    match args.first().map(String::as_str) {
        Some("list") => {
            let ids = registry(&pack_at(root)?);
            Ok(ids.iter().map(|(id, parent)| format!("{id}\t{parent}")).collect::<Vec<_>>().join("\n"))
        }
        Some("check") => {
            let base = match (crate::flag(args, "--base"), crate::flag(args, "--base-pack")) {
                (Some(rev), None) => registry(&pack_at_rev(root, rev)?),
                (None, Some(file)) => {
                    let bytes = std::fs::read(file).map_err(|e| vec![format!("{file}: {e}")])?;
                    registry(&Pack::decode(&bytes).map_err(|e| vec![format!("{file}: {e}")])?)
                }
                _ => return Err(vec!["usage: steps-tools ids check (--base REF | --base-pack FILE)".into()]),
            };
            check(root, &base, &registry(&pack_at(root)?))
        }
        _ => Err(vec!["usage: steps-tools ids <list | check (--base REF | --base-pack FILE)>".into()]),
    }
}

/// Compares; Ok with the report, or Err with it when it is breaking.
pub fn check(root: &Path, base: &Registry, head: &Registry) -> Res<String> {
    let retired = match std::fs::read_to_string(root.join(RETIRED)) {
        Ok(t) => parse_retired(&t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeSet::new(),
        Err(e) => return Err(vec![format!("{RETIRED}: {e}")]),
    };
    let report = compare(base, head, &retired);
    let mut lines: Vec<String> = report.lines().into_iter().map(|l| format!("ids: {l}")).collect();
    if report.breaking() {
        lines.push(format!(
            "content ids his records point at were removed or repurposed. Keep the id (fix the text instead), \
             or, after a deliberate migration decision, list it in {RETIRED}"
        ));
        return Err(lines);
    }
    lines.push(format!("ids: {} checked, no breaking change", head.len()));
    Ok(lines.join("\n"))
}

/// The content of a checkout as one pack (no stylesheets or images:
/// ids only).
pub fn pack_at(root: &Path) -> Res<Pack> {
    let (index, lectures) = crate::packs::load(root)?;
    let lessons = match root.join("content/lessons").exists() {
        true => crate::lessons::load(root)?.into_iter().map(|(_, f, _)| f).collect(),
        false => Vec::new(),
    };
    let mut pack = Pack::new(index, lectures).with_lessons(lessons);
    if let Ok(p) = crate::read_yaml(&root.join("content/program.yaml")) {
        pack = pack.with_program(p);
    }
    Ok(pack)
}

/// The content at git commit `rev`: `content/` exported to a scratch
/// directory (removed afterwards) and read like a checkout.
pub fn pack_at_rev(root: &Path, rev: &str) -> Res<Pack> {
    let scratch = std::env::temp_dir().join(format!(
        "steps-ids-{}-{}",
        std::process::id(),
        rev.replace(['/', '^', '~'], "_")
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).map_err(|e| vec![format!("{}: {e}", scratch.display())])?;
    let result = (|| {
        let archive = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["archive", "--format=tar", rev, "--", "content"])
            .output()
            .map_err(|e| vec![format!("git archive: {e}")])?;
        if !archive.status.success() {
            return Err(vec![format!(
                "git archive {rev}: {}",
                String::from_utf8_lossy(&archive.stderr).trim()
            )]);
        }
        let tar = scratch.join("content.tar");
        std::fs::write(&tar, &archive.stdout).map_err(|e| vec![e.to_string()])?;
        let status = Command::new("tar")
            .arg("-xf")
            .arg(&tar)
            .arg("-C")
            .arg(&scratch)
            .status()
            .map_err(|e| vec![format!("tar: {e}")])?;
        if !status.success() {
            return Err(vec![format!("tar couldn't unpack {rev}'s content")]);
        }
        pack_at(&scratch)
    })();
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_commit_and_the_working_tree_agree() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let Ok(at_head) = pack_at_rev(&root, "HEAD") else { return }; // not a git checkout
        let ids = registry(&at_head);
        assert!(ids.len() > 10, "{}", ids.len());
        assert!(check(&root, &ids, &ids).is_ok());
    }

    #[test]
    fn a_removed_id_fails_until_it_is_retired() {
        let dir = tempfile::tempdir().unwrap();
        let mut base = Registry::new();
        base.insert("item cvs.L04.001".into(), "kind mcq".into());
        let err = check(dir.path(), &base, &Registry::new()).unwrap_err();
        assert!(err.last().unwrap().contains(RETIRED), "{err:?}");
        std::fs::create_dir_all(dir.path().join("content")).unwrap();
        std::fs::write(dir.path().join(RETIRED), "item cvs.L04.001  # merged\n").unwrap();
        assert!(check(dir.path(), &base, &Registry::new()).is_ok());
    }

    use crate::testkit::{args, commit, content_tree, init_repo, write};

    #[test]
    fn list_prints_every_id_with_its_parent() {
        let dir = content_tree();
        let out = run(dir.path(), &args(&["list"])).unwrap();
        assert!(out.lines().any(|l| l == "item cvs.L04.003\tkind mcq"), "{out}");
        assert!(out.lines().any(|l| l == "topic cvs.L04#shock\tlecture cvs.L04"), "{out}");
        assert!(run(dir.path(), &args(&["purge"])).unwrap_err()[0].starts_with("usage: steps-tools ids"));
        assert!(run(dir.path(), &args(&["check"])).unwrap_err()[0].contains("--base REF | --base-pack FILE"));
    }

    #[test]
    fn check_against_a_published_pack() {
        let dir = content_tree();
        let root = dir.path();
        let pack = root.join("base.pack");
        std::fs::write(&pack, pack_at(root).unwrap().encode()).unwrap();
        let p = pack.to_str().unwrap();
        let out = run(root, &args(&["check", "--base-pack", p])).unwrap();
        assert!(out.ends_with("checked, no breaking change"), "{out}");
        // The card cvs.L04.001 disappears from the working tree.
        let sample = std::fs::read_to_string(root.join("content/cvs/L04.yaml")).unwrap();
        let start = sample.find("  - id: cvs.L04.001").unwrap();
        let end = sample.find("  - id: cvs.L04.002").unwrap();
        write(root, "content/cvs/L04.yaml", &format!("{}{}", &sample[..start], &sample[end..]));
        let err = run(root, &args(&["check", "--base-pack", p])).unwrap_err();
        assert!(err[0].starts_with("ids: BREAKING removed: item cvs.L04.001 (kind card)"), "{err:?}");
        let missing = root.join("none.pack");
        assert!(run(root, &args(&["check", "--base-pack", missing.to_str().unwrap()])).is_err());
        std::fs::write(&missing, b"not a pack").unwrap();
        let err = run(root, &args(&["check", "--base-pack", missing.to_str().unwrap()])).unwrap_err();
        assert!(err[0].contains("none.pack"), "{err:?}");
    }

    #[test]
    fn check_against_a_commit() {
        let dir = content_tree();
        let root = dir.path();
        let base = init_repo(root);
        // Repurposing an item (same id, another kind) breaks his records.
        let sample = std::fs::read_to_string(root.join("content/cvs/L04.yaml")).unwrap();
        write(
            root,
            "content/cvs/L04.yaml",
            &sample.replacen("kind: card", "kind: essay\n    prompt: x\n    outline: [a]", 1),
        );
        commit(root, "repurpose");
        let err = run(root, &args(&["check", "--base", &base])).unwrap_err();
        assert!(
            err.iter()
                .any(|l| l.contains("BREAKING repurposed: item cvs.L04.001 was kind card, now kind essay")),
            "{err:?}"
        );
        // An unknown commit is named.
        let err = pack_at_rev(root, "no-such-branch").unwrap_err();
        assert!(err[0].starts_with("git archive no-such-branch"), "{err:?}");
    }

    #[test]
    fn an_unreadable_retired_list_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        // A directory where the file should be.
        std::fs::create_dir_all(dir.path().join(RETIRED)).unwrap();
        let err = check(dir.path(), &Registry::new(), &Registry::new()).unwrap_err();
        assert!(err[0].starts_with(RETIRED), "{err:?}");
    }
}
