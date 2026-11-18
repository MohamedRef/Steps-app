use super::*;
use crate::testkit::{args, content_tree, write};

/// A key made here, never a real one.
fn key(fill: u8) -> Key {
    Key::from_bytes([fill; 32])
}

fn with(fill: u8) -> Env {
    Env { key: Some(key(fill)), required: false }
}

/// A figure: a WebP header is enough for the checks.
const FIGURE: &[u8] = b"RIFF\x0c\x00\x00\x00WEBPVP8L\x00\x00";

/// What the first question teaches.
const OBJECTIVE: &str = "Reflex (vasovagal) syncope is the most common cause of fainting.";

/// The open folder `vq open` would write: two questions under the fixture
/// lecture `cvs.L04`, the first with a figure and an educational objective.
fn plain_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let taught = question(1, &format!("    objective: \"{OBJECTIVE}\"\n"));
    write(dir.path(), "cvs/L04.yaml", &plain(&[question(2, ""), taught]));
    std::fs::create_dir_all(dir.path().join("images")).unwrap();
    std::fs::write(dir.path().join("images/VQ-0001.webp"), FIGURE).unwrap();
    dir
}

fn plain(items: &[String]) -> String {
    format!("schema: steps.content/v1\nlecture: cvs.L04\nitems:\n{}", items.concat())
}

fn question(n: u32, extra: &str) -> String {
    format!(
        "  - id: cvs.L04.vq{n:04}\n    kind: mcq\n    topic: shock\n    stem: \"A 70-year-old woman faints at church: question {n}. Which is most likely?\"\n    options: [Vagal syncope, Heart block, Hemorrhage, Anaphylaxis, Sepsis, Tamponade]\n    answer: 0\n    why: [\"Right. Reflex.\", b, c, d, e, f]\n    src: {{ book: vq, page: {n} }}\n    status: unverified\n    tags: [vq]\n{extra}"
    )
}

fn sealed_text(root: &Path) -> String {
    std::fs::read_to_string(root.join("content/cvs/vq/L04.yaml")).unwrap()
}

/// `check` on the tree, with the fixture lecture as written.
fn check_tree(root: &Path, env: &Env) -> (String, Vec<String>) {
    let index: Index = crate::read_yaml(&root.join("content/index.yaml")).unwrap();
    let (_, lectures) = crate::packs::load(root).unwrap();
    let approved = BTreeSet::new();
    let qbank: BTreeSet<String> = ["vq".to_string(), "bb-uworld".to_string()].into();
    check(root, &index, &lectures, &approved, &qbank, env)
}

#[test]
fn keygen_prints_a_key_and_its_id_and_writes_nothing() {
    let dir = content_tree();
    let before = std::fs::read_dir(dir.path()).unwrap().count();
    let out = run(dir.path(), &args(&["keygen"])).unwrap();
    let text = out.lines().next().unwrap().strip_prefix("STEPS_VQ_KEY=").unwrap();
    let k = Key::parse(text).unwrap();
    assert!(out.contains(&format!("key id: {}", k.id())), "{out}");
    let again = run(dir.path(), &args(&["keygen"])).unwrap();
    assert_ne!(again.lines().next(), out.lines().next(), "a new key every time");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
    assert!(run(dir.path(), &args(&["burn"])).unwrap_err()[0].starts_with("usage: steps-tools vq"));
    assert!(run(dir.path(), &args(&["open"])).unwrap_err()[0].starts_with("usage: steps-tools vq"));
}

#[test]
fn seal_then_open_then_seal_changes_nothing() {
    let tree = content_tree();
    let root = tree.path();
    let dir = plain_dir();
    let out = seal(root, dir.path(), &with(7)).unwrap();
    assert!(out.starts_with("sealed 2 questions into 1 files with key"), "{out}");
    assert!(out.contains("wrote content/cvs/vq/L04.yaml"), "{out}");
    let text = sealed_text(root);
    // Nothing of the questions is in clear, and the file names its key.
    assert!(!text.contains("church") && !text.contains("syncope"), "{text}");
    assert!(text.contains(&format!("key: \"{}\"", key(7).id())), "{text}");
    let file: SealedLecture = serde_yaml_ng::from_str(&text).unwrap();
    assert_eq!(
        file.items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
        ["cvs.L04.vq0001", "cvs.L04.vq0002"]
    );
    let first = sealed::open_item(&key(7), &file.items[0]).unwrap();
    assert_eq!(first.image_bytes().as_deref(), Some(FIGURE));
    assert_eq!(first.image_type.as_deref(), Some(sealed::IMAGE_TYPE));
    assert_eq!(first.objective.as_deref(), Some(OBJECTIVE));
    let second = sealed::open_item(&key(7), &file.items[1]).unwrap();
    assert_eq!((second.image, second.objective), (None, None));

    // Open into a fresh folder: the same questions and figure come out.
    let out_dir = tempfile::tempdir().unwrap();
    let opened = out_dir.path().join("vq-open");
    let out = open(root, &opened, &with(7)).unwrap();
    assert!(out.starts_with("opened 2 questions and 1 figures"), "{out}");
    assert_eq!(std::fs::read(opened.join("images/VQ-0001.webp")).unwrap(), FIGURE);
    let written = std::fs::read_to_string(opened.join("cvs/L04.yaml")).unwrap();
    assert!(
        written.contains(&format!("objective: {OBJECTIVE}")),
        "the objective sits on its item:\n{written}"
    );
    let back: Plain = serde_yaml_ng::from_str(&written).unwrap();
    let taught = question(1, &format!("    objective: \"{OBJECTIVE}\"\n"));
    let want: Plain = serde_yaml_ng::from_str(&plain(&[taught, question(2, "")])).unwrap();
    assert_eq!(back.items, want.items);
    assert_eq!(back.items[0].objective.as_deref(), Some(OBJECTIVE));
    // Sealing it again writes nothing: the same bytes.
    let out = seal(root, &opened, &with(7)).unwrap();
    assert!(out.ends_with("nothing changed"), "{out}");
    assert_eq!(sealed_text(root), text);
}

#[test]
fn open_needs_the_key_a_folder_outside_the_repo_and_an_empty_one() {
    let tree = content_tree();
    let root = tree.path();
    seal(root, plain_dir().path(), &with(7)).unwrap();
    let out = tempfile::tempdir().unwrap();
    let e = open(root, &out.path().join("x"), &Env::default()).unwrap_err();
    assert!(e[0].starts_with("STEPS_VQ_KEY is not set"), "{e:?}");
    // Inside the repo, even through a folder that doesn't exist yet.
    for inside in [root.join("vq-open"), root.join("content/new/deeper")] {
        let e = open(root, &inside, &with(7)).unwrap_err();
        assert!(e[0].contains("is inside the repo"), "{e:?}");
        assert!(!inside.exists());
    }
    // A folder with something in it already.
    write(out.path(), "full/notes.txt", "x");
    let e = open(root, &out.path().join("full"), &with(7)).unwrap_err();
    assert!(e[0].contains("is not empty"), "{e:?}");
    // Another key.
    let e = open(root, &out.path().join("y"), &with(8)).unwrap_err();
    assert!(
        e[0].contains(&format!("sealed with key {}, but STEPS_VQ_KEY is key {}", key(7).id(), key(8).id())),
        "{e:?}"
    );
    assert!(!out.path().join("y/cvs").exists(), "nothing written");
}

#[test]
fn seal_removes_a_lecture_left_without_questions() {
    let tree = content_tree();
    let root = tree.path();
    let dir = plain_dir();
    seal(root, dir.path(), &with(7)).unwrap();
    // Both questions moved out of the lecture: its file goes.
    write(dir.path(), "cvs/L04.yaml", &plain(&[]));
    std::fs::remove_file(dir.path().join("images/VQ-0001.webp")).unwrap();
    let out = seal(root, dir.path(), &with(7)).unwrap();
    assert!(out.contains("removed content/cvs/vq/L04.yaml (no questions left)"), "{out}");
    assert!(!root.join("content/cvs/vq/L04.yaml").exists());
    // Sealing with no folder at all is refused, not taken as "remove everything".
    let e = seal(root, &dir.path().join("gone"), &with(7)).unwrap_err();
    assert!(e[0].contains("no such folder"), "{e:?}");
}

#[test]
fn seal_refuses_a_folder_with_problems_and_writes_nothing() {
    let tree = content_tree();
    let root = tree.path();
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let typo = question(3, "").replace("    answer: 0\n", "    answer: 0\n    anwser: 1\n");
    write(
        d,
        "cvs/L04.yaml",
        &plain(&[
            question(1, ""),
            typo,
            question(1, "").replace("vq0001", "vq0004").replace("page: 1", "page: 4"),
        ]),
    );
    // VQ-0001 again in another lecture's file, which says it is cvs.L04.
    write(d, "cvs/L05.yaml", &plain(&[question(1, "")]));
    write(d, "cvs/notes.txt", "x");
    write(d, "images/VQ-0009.webp", "RIFF");
    write(d, "images/figure.png", "x");
    let e = seal(root, d, &with(7)).unwrap_err();
    for want in [
        "cvs/L04.yaml: cvs.L04.vq0003: unknown key `anwser`",
        "cvs/L05.yaml: lecture is cvs.L04, the file name says cvs.L05",
        "cvs/L05.yaml: lecture cvs.L05 is not in content/index.yaml",
        "cvs/L05.yaml: cvs.L04.vq0001: id must be cvs.L05.vqNNNN",
        "cvs/notes.txt: not a lecture file",
        "images/figure.png: figures are named VQ-NNNN.webp",
        "images/VQ-0009.webp: no question VQ-0009 in the folder",
    ] {
        assert!(e.iter().any(|p| p.starts_with(want)), "{want}: {e:#?}");
    }
    assert!(!root.join("content/cvs/vq").exists(), "nothing written");
    // A question filed twice.
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "cvs/L04.yaml", &plain(&[question(1, ""), question(1, "")]));
    let e = seal(root, dir.path(), &with(7)).unwrap_err();
    assert!(e.iter().any(|p| p.contains("VQ-0001 is filed twice")), "{e:#?}");
    // Files sealed with another key stay as they are.
    seal(root, plain_dir().path(), &with(7)).unwrap();
    let e = seal(root, plain_dir().path(), &with(8)).unwrap_err();
    assert!(e[0].contains("seal every file with one key"), "{e:#?}");
    // Inside the repo is refused here too.
    write(root, "vq-open/cvs/L04.yaml", &plain(&[question(1, "")]));
    assert!(seal(root, &root.join("vq-open"), &with(7)).unwrap_err()[0].contains("inside the repo"));
}

#[test]
fn without_the_key_validate_counts_what_it_left_closed() {
    let tree = content_tree();
    let root = tree.path();
    let (line, problems) = check_tree(root, &Env::default());
    assert_eq!(line, "VQ: 0 sealed questions not opened (STEPS_VQ_KEY is not set)");
    assert!(problems.is_empty());
    // CI asks for the key, but with nothing sealed nothing fails.
    assert!(check_tree(root, &Env { key: None, required: true }).1.is_empty());
    seal(root, plain_dir().path(), &with(7)).unwrap();
    let (line, problems) = check_tree(root, &Env::default());
    assert_eq!(line, "VQ: 2 sealed questions not opened (STEPS_VQ_KEY is not set)");
    assert!(problems.is_empty(), "{problems:#?}");
    let (_, problems) = check_tree(root, &Env { key: None, required: true });
    assert_eq!(
        problems,
        [
            "VQ: 2 sealed questions not opened (STEPS_VQ_KEY is not set), and STEPS_VQ_REQUIRED=1 needs them checked"
        ]
    );
}

#[test]
fn with_the_key_validate_opens_every_question_and_checks_it() {
    let tree = content_tree();
    let root = tree.path();
    let dir = plain_dir();
    seal(root, dir.path(), &with(7)).unwrap();
    let (line, problems) = check_tree(root, &with(7));
    assert_eq!(line, "VQ: 2 sealed questions opened and checked, 1 with a figure");
    assert!(problems.is_empty(), "{problems:#?}");
    // The wrong key opens nothing and says so.
    let (_, problems) = check_tree(root, &with(8));
    assert_eq!(
        problems,
        [format!("VQ: STEPS_VQ_KEY is key {}, but the sealed files name key {}", key(8).id(), key(7).id())]
    );
    // An item's own rules, and the bank's: a topic the lecture doesn't have,
    // another page, no tag, and a figure that isn't WebP.
    let broken = question(2, "    objective: \" \"\n")
        .replace("topic: shock", "topic: murmurs")
        .replace("page: 2", "page: 3")
        .replace("tags: [vq]", "tags: [no-figure]");
    write(dir.path(), "cvs/L04.yaml", &plain(&[question(1, ""), broken]));
    std::fs::write(dir.path().join("images/VQ-0001.webp"), b"\x89PNG\r\n\x1a\n0000").unwrap();
    seal(root, dir.path(), &with(7)).unwrap();
    let (_, problems) = check_tree(root, &with(7));
    for want in [
        "content/cvs/vq/L04.yaml: cvs.L04.vq0001: image is not a WebP file",
        "content/cvs/vq/L04.yaml: cvs.L04.vq0002: topic `murmurs` is not in the lecture's topics",
        "content/cvs/vq/L04.yaml: cvs.L04.vq0002: src.page must be 2, the id's VQ number",
        "content/cvs/vq/L04.yaml: cvs.L04.vq0002: tags need `vq`",
        "content/cvs/vq/L04.yaml: cvs.L04.vq0002: objective is empty; write it or leave it out",
    ] {
        assert!(problems.iter().any(|p| p == want), "{want}: {problems:#?}");
    }
    // A blob that was changed by hand.
    let text = sealed_text(root);
    let (head, tail) = text.split_once("sealed: \"").unwrap();
    let mut tail: Vec<char> = tail.chars().collect();
    tail[10] = if tail[10] == 'B' { 'C' } else { 'B' };
    write(
        root,
        "content/cvs/vq/L04.yaml",
        &format!("{head}sealed: \"{}", tail.into_iter().collect::<String>()),
    );
    let (_, problems) = check_tree(root, &with(7));
    assert!(
        problems.iter().any(|p| p.contains("cvs.L04.vq0001: doesn't open with this key")),
        "{problems:#?}"
    );
}

#[test]
fn the_files_are_checked_without_the_key() {
    let tree = content_tree();
    let root = tree.path();
    seal(root, plain_dir().path(), &with(7)).unwrap();
    let good = sealed_text(root);
    // The same lecture in the wrong file, under another key.
    let other_key =
        good.replace(&key(7).id(), &key(8).id()).replace("vq0001", "vq0011").replace("vq0002", "vq0012");
    write(root, "content/cvs/vq/L05.yaml", &other_key);
    // Something that isn't a sealed file, and a stray file.
    write(root, "content/renal/vq/L01.yaml", "schema: steps.content/v1\nlecture: renal.L01\nitems: []\n");
    write(root, "content/cvs/vq/VQ-0001.webp", "RIFF");
    // A lecture the index doesn't list, with an id an open item has.
    write(
        root,
        "content/gi/vq/L02.yaml",
        &good.replace("lecture: cvs.L04", "lecture: gi.L02").replace("cvs.L04.vq0001", "cvs.L04.003"),
    );
    let (_, problems) = check_tree(root, &Env::default());
    for want in [
        "content/cvs/vq/L05.yaml: holds lecture cvs.L04, which belongs in content/cvs/vq/L04.yaml",
        "sealed files name different keys",
        "content/renal/vq/L01.yaml: not a sealed file (steps.sealed/v1)",
        "content/cvs/vq/VQ-0001.webp: only sealed files belong in a vq folder",
        "content/gi/vq/L02.yaml: lecture gi.L02 is not in content/index.yaml",
        "content/gi/vq/L02.yaml: cvs.L04.003: an open item has this id",
        "content/gi/vq/L02.yaml: cvs.L04.003: id must be gi.L02.vqNNNN",
        "content/gi/vq/L02.yaml: cvs.L04.vq0002: id is used twice",
        "content/gi/vq/L02.yaml: cvs.L04.vq0002: id must be gi.L02.vqNNNN",
    ] {
        assert!(problems.iter().any(|p| p.starts_with(want)), "{want}: {problems:#?}");
    }
    // A sealed lecture whose open lecture isn't written.
    let tree = content_tree();
    let root = tree.path();
    write(
        root,
        "content/index.yaml",
        &format!(
            "{}  - {{ id: cvs.L05, title: Pharmacology, pages: [321, 328], chunk: 6 }}\n",
            crate::testkit::INDEX
        ),
    );
    write(root, "content/cvs/vq/L05.yaml", &good.replace("cvs.L04", "cvs.L05"));
    let (_, problems) = check_tree(root, &Env::default());
    assert_eq!(
        problems,
        [
            "content/cvs/vq/L05.yaml: lecture cvs.L05 has no content/cvs/L05.yaml, so its questions would never show"
        ]
    );
}

#[test]
fn packs_ids_and_counts_see_the_sealed_files_as_they_are() {
    let tree = content_tree();
    let root = tree.path();
    let before = crate::counts::count(root).unwrap();
    seal(root, plain_dir().path(), &with(7)).unwrap();
    // The pack carries the file as it is, with the sources.
    let out = root.join("out");
    let report = crate::packs::run(root, &args(&["build", "--out", out.to_str().unwrap()])).unwrap();
    assert!(report.contains("2 sealed question(s)"), "{report}");
    let pack =
        steps_core::pack::Pack::decode(&std::fs::read(out.join("steps-content.pack")).unwrap()).unwrap();
    assert_eq!(pack.sealed, [serde_yaml_ng::from_str::<SealedLecture>(&sealed_text(root)).unwrap()]);
    assert!(pack.sources.iter().any(|s| s.id == "vq" && s.kind == "qbank"), "{:?}", pack.sources);
    // Their ids are his records' ids like any other.
    let ids = crate::ids::run(root, &args(&["list"])).unwrap();
    assert!(ids.lines().any(|l| l == "item cvs.L04.vq0001\tkind mcq"), "{ids}");
    // The public numbers leave them out.
    assert_eq!(crate::counts::count(root).unwrap(), before);
    // A broken sealed file never builds.
    write(root, "content/cvs/vq/L04.yaml", "schema: [");
    assert!(
        crate::packs::run(root, &args(&["build", "--check"])).unwrap_err()[0].contains("not a sealed file")
    );
}
