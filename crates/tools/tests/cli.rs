//! The `steps-tools` binary end to end: each command's exit code and what
//! it prints, on the repository's content (read only) or on a temp copy of
//! the First Aid fixture lecture (`cvs.L04`). Nothing here touches the network.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tools(args: &[&str]) -> Output {
    tools_with(args, &[])
}

fn tools_with(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_steps-tools"));
    c.args(args).env_remove("STEPS_UPDATE_SECRET");
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// A tree with only the fixture lecture, `cvs.L04` (Cardiovascular › Pathology).
fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("content/cvs")).unwrap();
    std::fs::write(
        root.join("content/index.yaml"),
        "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [300, 320], chunk: 6 }\n",
    )
    .unwrap();
    for f in ["content/program.yaml", "content/sources.yaml"] {
        std::fs::copy(repo().join(f), root.join(f)).unwrap();
    }
    std::fs::copy(repo().join("fixtures/content/sample.yaml"), root.join("content/cvs/L04.yaml")).unwrap();
    dir
}

#[test]
fn usage_without_a_command() {
    for args in [&[][..], &["publish"], &["validate", "--root"]] {
        let o = tools(args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(err(&o).starts_with("usage: steps-tools <validate | anki"), "{}", err(&o));
    }
}

#[test]
fn validate_passes_on_the_repo_and_counts_problems_elsewhere() {
    let root = repo();
    let o = tools(&["validate", "--root", root.to_str().unwrap()]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).starts_with("content OK: "), "{}", out(&o));

    // The fixture's items have no page-reference lock in a fresh tree.
    let dir = tree();
    let o = tools(&["validate", "--root", dir.path().to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    let e = err(&o);
    assert!(e.contains("missing page-reference lock; run steps-tools refs --lecture cvs.L04"), "{e}");
    assert!(e.trim_end().ends_with("1 problem(s)"), "{e}");
}

#[test]
fn each_command_reports_success_on_stdout_and_problems_on_stderr() {
    let dir = tree();
    let root = dir.path().to_str().unwrap();
    let o = tools(&["packs", "build", "--check", "--root", root]);
    assert!(o.status.success() && out(&o).starts_with("pack OK: 1 lecture(s), 8 items"), "{}", err(&o));
    let o = tools(&["ids", "list", "--root", root]);
    assert!(o.status.success() && out(&o).contains("item cvs.L04.008\tkind order"), "{}", err(&o));
    let o = tools(&["content", "next-id", "cvs.L04", "--root", root]);
    assert_eq!(out(&o), "next item id: cvs.L04.009\nnext lesson id: cvs.L04.lesson1\n");
    let anki_out = dir.path().join("anki");
    let o = tools(&["anki", "--merged-only", "--out", anki_out.to_str().unwrap(), "--root", root]);
    assert!(o.status.success() && out(&o).contains("steps.apkg: 11 notes"), "{}", err(&o));
    let o = tools(&["update", "channels", "--version", "1.0.0-beta.1", "--root", root]);
    assert_eq!(out(&o), "alpha\nbeta\n");

    // Problems go to stderr with exit code 1.
    let o = tools(&["verdicts", "--root", root]);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(err(&o), "usage: steps-tools verdicts FILE [--approve]\n");
    let o = tools(&["refs", "--lecture", "cvs.L04", "--book", "/nonexistent", "--root", root]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("/nonexistent/L04.txt"), "{}", err(&o));
}

#[test]
fn update_sign_reads_the_secret_from_the_environment() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let key = d.join("update-key.pub");
    let o = tools(&["update", "keygen", "--public-out", key.to_str().unwrap()]);
    assert!(o.status.success(), "{}", err(&o));
    let secret = out(&o).trim().to_owned();
    assert!(err(&o).starts_with("public key "), "{}", err(&o));

    let pack = d.join("steps-content.pack");
    let o =
        tools(&["packs", "build", "--out", d.to_str().unwrap(), "--root", tree().path().to_str().unwrap()]);
    assert!(o.status.success(), "{}", err(&o));
    let feed = d.join("feed");
    let asset = format!("content={}", pack.display());
    let o = tools(&[
        "update",
        "manifest",
        "--channel",
        "alpha",
        "--version",
        "1.0.0-alpha.4",
        "--build",
        "9",
        "--asset",
        &asset,
        "--out",
        feed.to_str().unwrap(),
    ]);
    assert!(o.status.success(), "{}", err(&o));
    let f = feed.to_str().unwrap();
    let k = key.to_str().unwrap();
    let o = tools(&["update", "sign", f, "--public-key", k]);
    assert_eq!(err(&o), "STEPS_UPDATE_SECRET is not set\n");
    let o = tools_with(&["update", "sign", f, "--public-key", k], &[("STEPS_UPDATE_SECRET", &secret)]);
    assert!(o.status.success() && out(&o).starts_with("signed "), "{}", err(&o));
    let o = tools(&["update", "verify", f, "--public-key", k]);
    assert_eq!(out(&o), "ok: alpha 1.0.0-alpha.4 build 9, 1 file(s) match\n", "{}", err(&o));
}
