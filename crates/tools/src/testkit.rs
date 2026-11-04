//! Test helpers shared by the commands' tests: a small content tree on the
//! First Aid fixture lecture (`cvs.L04`, Cardiovascular › Pathology), and a
//! throwaway git repository for the commands that compare with a commit.
//! Everything lives in temp dirs; nothing touches the network.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository checkout the tests run in.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

pub const SAMPLE: &str = include_str!("../../../fixtures/content/sample.yaml");

pub const INDEX: &str = "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [300, 320], chunk: 6 }\n";

/// `content/` with `index.yaml`, the real `program.yaml` and
/// `sources.yaml`, `cvs/L04.yaml` from the fixture, and the fixture itself
/// under `fixtures/` (for `content new-item`).
pub fn content_tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let repo = repo_root();
    write(root, "content/index.yaml", INDEX);
    for f in ["content/program.yaml", "content/sources.yaml"] {
        write(root, f, &std::fs::read_to_string(repo.join(f)).unwrap());
    }
    write(root, "content/cvs/L04.yaml", SAMPLE);
    write(root, "fixtures/content/sample.yaml", SAMPLE);
    dir
}

pub fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

pub fn git(root: &Path, a: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(root).args(a).output().unwrap();
    assert!(out.status.success(), "git {a:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// Makes `root` a git repository with one commit of everything in it.
pub fn init_repo(root: &Path) -> String {
    for a in [&["init", "-q"][..], &["config", "user.email", "t@t"], &["config", "user.name", "t"]] {
        git(root, a);
    }
    commit(root, "first")
}

pub fn commit(root: &Path, message: &str) -> String {
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "--allow-empty", "-m", message]);
    git(root, &["rev-parse", "HEAD"])
}
