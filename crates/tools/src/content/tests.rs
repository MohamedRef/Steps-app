use super::*;

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn next_ids_follow_the_highest_used_or_retired() {
    let (item, lesson) = next_ids(&root(), "cvs.L01").unwrap();
    let lec = read_lecture(&root(), "cvs.L01").unwrap();
    assert!(!lec.items.iter().any(|i| i.id == item), "{item} is free");
    assert!(item.starts_with("cvs.L01.") && lesson.starts_with("cvs.L01.lesson"));
}

#[test]
fn a_new_item_is_valid_but_marked_unfinished() {
    for kind in ["card", "cloze", "mcq", "enumerate", "essay"] {
        let out = new_item(&root(), "cvs.L01", kind).unwrap();
        assert!(out.contains(TEMPLATE_MARK), "{kind}");
        let yaml: String = out.lines().filter(|l| !l.starts_with('#')).map(|l| format!("{l}\n")).collect();
        let items: Vec<Item> = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(items[0].id, next_ids(&root(), "cvs.L01").unwrap().0, "{kind}");
    }
    assert!(new_item(&root(), "cvs.L01", "nonsense").is_err());
}

#[test]
fn the_repo_has_nothing_unfinished_and_a_template_is_caught() {
    assert_eq!(unfinished(&root()), Vec::<String>::new());
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("content/cvs")).unwrap();
    std::fs::write(dir.path().join("content/cvs/L99.yaml"), format!("x: {TEMPLATE_MARK} y\n")).unwrap();
    assert_eq!(unfinished(dir.path()).len(), 1);
}

#[test]
fn the_diff_counts_items_and_lessons_per_lecture() {
    let head = crate::ids::pack_at(&root()).unwrap();
    assert_eq!(diff(&head, &head), "no lecture or lesson differs");
    let mut base = head.clone();
    let first = base.lectures[0].lecture.clone();
    base.lectures[0].items.pop();
    base.lectures[0].items[0].status = steps_core::content::Status::Unverified;
    let out = diff(&base, &head);
    assert!(out.contains(&format!("| {first} | 1 / ")), "{out}");
}

use crate::testkit::{args, commit, content_tree, init_repo, repo_root, write};

#[test]
fn next_id_skips_retired_items_and_lessons() {
    let dir = content_tree();
    let root = dir.path();
    assert_eq!(
        run(root, &args(&["next-id", "cvs.L04"])).unwrap(),
        "next item id: cvs.L04.009\nnext lesson id: cvs.L04.lesson1"
    );
    write(root, crate::ids::RETIRED, "item cvs.L04.012  # merged\nlesson cvs.L04.lesson3  # split\n");
    write(
        root,
        "content/lessons/cvs/L04.yaml",
        "schema: steps.lesson/v1\nlecture: cvs.L04\nlessons:\n  - { id: cvs.L04.lesson1, title: t, hook: h, minutes: 5, pages: [317, 317], steps: [], recap: [], status: unverified }\n",
    );
    let (item, lesson) = next_ids(root, "cvs.L04").unwrap();
    assert_eq!((item.as_str(), lesson.as_str()), ("cvs.L04.013", "cvs.L04.lesson4"));
    assert!(run(root, &args(&["next-id"])).unwrap_err()[0].starts_with("usage"));
    assert!(run(root, &args(&["next-id", "cvs.L09"])).is_err(), "no such lecture file");
}

#[test]
fn new_item_marks_every_text_and_cites_the_lecture() {
    let dir = content_tree();
    let root = dir.path();
    let out = run(root, &args(&["new-item", "cvs.L04", "table"])).unwrap();
    assert!(out.starts_with("# Paste under `items:` in content/cvs/L04.yaml."), "{out}");
    let yaml: String = out.lines().filter(|l| !l.starts_with('#')).map(|l| format!("{l}\n")).collect();
    let items: Vec<Item> = serde_yaml_ng::from_str(&yaml).unwrap();
    let it = &items[0];
    assert_eq!(it.id, "cvs.L04.009");
    assert_eq!(it.topic.as_deref(), Some("valves"), "the lecture's first topic");
    assert_eq!(it.src.page, 300, "the lecture's first page");
    assert!(it.src.quote.as_deref().is_some_and(|q| q.starts_with(TEMPLATE_MARK)));
    assert!(
        matches!(&it.body, steps_core::content::Body::Table { title, .. } if title.starts_with(TEMPLATE_MARK))
    );
    assert!(run(root, &args(&["new-item", "cvs.L04"])).unwrap_err()[0].starts_with("usage"));
    std::fs::remove_file(root.join("fixtures/content/sample.yaml")).unwrap();
    assert!(new_item(root, "cvs.L04", "card").is_err());
    assert!(run(root, &args(&["publish"])).unwrap_err()[0].contains("next-id | new-item | diff | preflight"));
}

#[test]
fn diff_against_a_pack_file_or_a_commit() {
    let dir = content_tree();
    let root = dir.path();
    let base = crate::ids::pack_at(root).unwrap();
    let base_file = root.join("base.pack");
    std::fs::write(&base_file, base.encode()).unwrap();
    let b = base_file.to_str().unwrap();
    assert_eq!(run(root, &args(&["diff", "--base-pack", b])).unwrap(), "no lecture or lesson differs");
    // Edit one item, add a lesson, and change the index title.
    let sample = std::fs::read_to_string(root.join("content/cvs/L04.yaml")).unwrap();
    write(root, "content/cvs/L04.yaml", &sample.replace("The mitral valve (mitral", "Mitral (mitral"));
    write(
        root,
        "content/index.yaml",
        &crate::testkit::INDEX.replace("Pathology", "Cardiovascular pathology"),
    );
    write(
        root,
        "content/lessons/cvs/L04.yaml",
        "schema: steps.lesson/v1\nlecture: cvs.L04\nlessons:\n  - { id: cvs.L04.lesson1, title: t, hook: h, minutes: 5, pages: [317, 317], steps: [], recap: [], status: unverified }\n",
    );
    let out = run(root, &args(&["diff", "--base-pack", b])).unwrap();
    assert!(out.starts_with("index: changed\n"), "{out}");
    assert!(out.contains("| cvs.L04 | 0 / 1 / 0 | 1 / 0 / 0 |"), "{out}");
    // A built pack as the head.
    let head_file = root.join("head.pack");
    std::fs::write(&head_file, crate::ids::pack_at(root).unwrap().encode()).unwrap();
    let out =
        run(root, &args(&["diff", "--base-pack", b, "--head-pack", head_file.to_str().unwrap()])).unwrap();
    assert!(out.contains("| cvs.L04 | 0 / 1 / 0 | 1 / 0 / 0 |"), "{out}");
    assert!(run(root, &args(&["diff", "--base-pack", b, "--head-pack", "/nonexistent.pack"])).is_err());
    assert!(
        run(root, &args(&["diff", "--base-pack", head_file.with_extension("x").to_str().unwrap()])).is_err()
    );
    std::fs::write(&head_file, b"zip").unwrap();
    assert!(
        run(root, &args(&["diff", "--base-pack", b, "--head-pack", head_file.to_str().unwrap()])).is_err()
    );
    assert!(run(root, &args(&["diff", "--base-pack", head_file.to_str().unwrap()])).is_err());
    assert_eq!(run(root, &args(&["diff"])).unwrap_err(), ["give --base REF or --base-pack FILE"]);
    // Against a commit: the program changed.
    let rev = init_repo(root);
    let program = std::fs::read_to_string(root.join("content/program.yaml")).unwrap();
    write(root, "content/program.yaml", &program.replace("title: USMLE Step 1", "title: Step 1"));
    commit(root, "rename the program");
    let out = run(root, &args(&["diff", "--base", &rev])).unwrap();
    assert_eq!(out, "program: changed\nno lecture or lesson differs");
}

#[test]
fn preflight_passes_on_the_repo_and_names_failed_checks() {
    // The repository itself, against its own HEAD: every check passes.
    if crate::ids::pack_at_rev(&repo_root(), "HEAD").is_ok() {
        let out = preflight(&repo_root(), "HEAD").unwrap();
        assert!(
            out.starts_with("| Check | Result |\n|---|---|\n| content checks (validate) | ✓ content OK"),
            "{out}"
        );
        assert!(out.contains("| content ids (records keep their targets) | ✓ ids:"), "{out}");
    }
    // A tree whose fixture items have no page locks, and no git history.
    let dir = content_tree();
    let err = run(dir.path(), &args(&["preflight", "--base", "HEAD"])).unwrap_err();
    assert!(err[0].contains("| content checks (validate) | ✗ "), "{err:?}");
    assert!(err[0].contains("| pack devices take (packs build --check) | ✓ pack OK"), "{err:?}");
    assert!(err[0].contains("| content ids (records keep their targets) | ✗ 1 problem(s)"), "{err:?}");
    assert_eq!(err[1], "2 check(s) failed; see above");
}
