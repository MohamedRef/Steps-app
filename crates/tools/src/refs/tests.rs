use super::*;

#[test]
fn squash_ignores_case_punctuation_and_glued_words() {
    assert_eq!(squash("Equilibration of diastolic pressures"), squash("Equilibration of diastolicpressures"));
    assert_eq!(squash("(>10 mm Hg) drop"), "10mmhgdrop");
    assert_eq!(quote_hash(None), "-");
    assert_eq!(quote_hash(Some("A b")), quote_hash(Some("a, B")));
    assert_ne!(quote_hash(Some("type A")), quote_hash(Some("type B")));
}

#[test]
fn o_bullets_do_not_break_a_quote() {
    let page = "Findings:\n    o   Hypotension.\n    o   Distant heart sounds.\none line";
    assert!(
        squash(&drop_o_bullets(page)).contains(&squash("Findings:\n  Hypotension.\n  Distant heart sounds."))
    );
    assert!(squash(&drop_o_bullets(page)).contains("oneline"));
}

#[test]
fn terms_keep_numbers_and_drop_common_words() {
    let t = terms("In the patients, pulsus paradoxus is a fall > 10 mm Hg (type A)");
    assert!(t.contains("pulsus") && t.contains("10") && t.contains("paradoxus"));
    assert!(!t.contains("the") && !t.contains("patients") && !t.contains("in"));
}

fn r(quote: &str, text: &str) -> Ref {
    Ref { id: "x".into(), page: 305, quote: Some(quote.into()), text: text.into(), book_figure: false }
}

#[test]
fn a_quote_about_another_fact_does_not_relate() {
    let stanford =
        "Stanford type A: ascending aorta, surgery. Stanford type B: descending only, beta-blockers.";
    assert!(!quote_relates(&r("Hypertension is the strongest risk factor for dissection", stanford)));
    assert!(quote_relates(&r("type A involves the ascending aorta; treatment is surgery", stanford)));
    assert!(quote_relates(&r("diastolic RV collapse", "the echo shows diastolic RV collapse")));
    assert!(quote_relates(&r("treated with surgery", "النوع A علاجه surgery")));
}

#[test]
fn the_lock_catches_changed_pages_and_quotes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("content/cvs")).unwrap();
    std::fs::create_dir_all(root.join(LOCKS).join("cvs")).unwrap();
    let item = |page: u32, quote: &str| {
        format!(
            "items:\n  - id: cvs.L04.001\n    kind: card\n    front: Which part of the aorta does Stanford type A involve?\n    back: The ascending aorta.\n    src: {{ page: {page}, quote: \"{quote}\" }}\n"
        )
    };
    let quote = "involves Ascending aorta";
    std::fs::write(root.join("content/cvs/L04.yaml"), item(305, quote)).unwrap();
    let lecture = vec!["cvs.L04".to_string()];
    std::fs::write(
        root.join(LOCKS).join("cvs").join("L04.tsv"),
        format!("# header\ncvs.L04.001\t305\t{}\tquote\t\n", quote_hash(Some(quote))),
    )
    .unwrap();
    let p = check_locks(root, &lecture);
    assert!(p.is_empty(), "{p:#?}");

    std::fs::write(root.join("content/cvs/L04.yaml"), item(306, quote)).unwrap();
    let p = check_locks(root, &lecture);
    assert!(p.iter().any(|p| p.contains("now cites p306, checked at p305")), "{p:#?}");

    std::fs::write(root.join("content/cvs/L04.yaml"), item(305, "involves only descending aorta")).unwrap();
    let p = check_locks(root, &lecture);
    assert!(p.iter().any(|p| p.contains("quote changed")), "{p:#?}");
}

#[test]
fn the_book_check_writes_a_lock_and_flags_wrong_pages() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let book = root.join("book");
    std::fs::create_dir_all(root.join("content/cvs")).unwrap();
    std::fs::create_dir_all(&book).unwrap();
    std::fs::write(
            root.join("content/index.yaml"),
            "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [317, 319], chunk: 6 }\n",
        )
        .unwrap();
    std::fs::write(
            book.join("L04.txt"),
            "=== book page 317 (PDF page 339) ===\nBeck triad (hypotension, distended neck veins, distant heart sounds).\n\
             === book page 319 (PDF page 341) ===\nRheumatic heart disease affects heart valves,\nmitral > aortic >> tricuspid.\n",
        )
        .unwrap();
    let yaml = |page: u32, quote: &str| {
        format!(
            "items:\n  - id: cvs.L04.001\n    kind: card\n    front: Which valve does rheumatic heart disease affect most?\n    back: Mitral.\n    src: {{ page: {page}{quote} }}\n  - id: cvs.L04.002\n    kind: card\n    front: What is Beck triad?\n    back: Hypotension, distended neck veins, distant heart sounds.\n    src: {{ page: 317, quote: \"hypotension, distended neck veins, distant heart sounds\" }}\n"
        )
    };
    let args = |extra: &[&str]| {
        let mut a: Vec<String> =
            ["--book", book.to_str().unwrap(), "--ocr", "/nonexistent", "--checked", "/nonexistent"]
                .iter()
                .map(|s| s.to_string())
                .collect();
        a.extend(extra.iter().map(|s| s.to_string()));
        a
    };
    let mitral = ", quote: \"mitral > aortic >> tricuspid\"";
    std::fs::write(root.join("content/cvs/L04.yaml"), yaml(319, "")).unwrap();
    let p = run(root, &args(&["--write"])).unwrap_err();
    assert!(p.iter().any(|p| p.contains("cvs.L04.001: has no quote")), "{p:#?}");
    assert!(!root.join(LOCKS).join("cvs").join("L04.tsv").exists());

    std::fs::write(root.join("content/cvs/L04.yaml"), yaml(317, mitral)).unwrap();
    let p = run(root, &args(&["--write"])).unwrap_err();
    assert!(p.iter().any(|p| p.contains("cvs.L04.001: quote is not on p317 (p319)")), "{p:#?}");

    std::fs::write(root.join("content/cvs/L04.yaml"), yaml(319, mitral)).unwrap();
    run(root, &args(&["--write"])).unwrap();
    let lock = read_lock(&root.join(LOCKS).join("cvs").join("L04.tsv")).unwrap();
    assert_eq!(lock["cvs.L04.001"].basis, "quote");
    assert_eq!(lock["cvs.L04.002"].basis, "quote");
    assert!(check_locks(root, &["cvs.L04".into()]).is_empty());
}

#[test]
fn a_page_without_the_terms_scores_zero_not_minus_zero() {
    let pages: BTreeMap<u32, BTreeSet<String>> = [
        (1, terms("Mitral stenosis follows rheumatic fever")),
        (2, terms("troponin rises after infarction aspirin")),
    ]
    .into();
    let mut df = HashMap::new();
    for t in pages.values() {
        for w in t {
            *df.entry(w.clone()).or_default() += 1;
        }
    }
    let sc = scores("troponin rises after infarction: give aspirin", &pages, &df);
    assert_eq!(format!("{:.0}%", sc[&1] * 100.0), "0%");
    assert_eq!(format!("{:.0}%", sc[&2] * 100.0), "100%");
}

#[test]
fn every_course_keeps_its_own_text_and_locks() {
    let root = Path::new("/r");
    assert_eq!(lock_dir(root, "cvs"), Path::new("/r/content/book-refs/cvs"));
    assert_eq!(lock_dir(root, "phs"), Path::new("/r/content/book-refs/phs"));
    assert_eq!(source_dir(None, "cvs"), Path::new(COURSES).join("cvs/text"));
    assert_eq!(source_dir(None, "phs"), Path::new(COURSES).join("phs/text"));
    assert_eq!(source_dir(Some("/x"), "phs"), Path::new("/x"));

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("L02.txt"),
        "=== book page 287 ===\nHeart anatomy\n\u{c}=== book page 288 ===\nCardiac output\n",
    )
    .unwrap();
    let pages = book_pages(dir.path(), &[], "L02").unwrap();
    assert_eq!(pages.len(), 2);
    assert!(pages[&288].contains("Cardiac output"));
    assert!(!pages[&287].contains("book page 288"));
}

#[test]
fn terms_keep_decimals_whole() {
    let t = terms("A QRS wider than 0.12 s, a PR over 0.2");
    assert!(t.contains("0.12") && t.contains("0.2") && t.contains("qrs"), "{t:?}");
}

/// A tree with the lecture `cvs.L04` written from `yaml`, and its lessons when given.
fn tree(yaml: &str, lessons: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("content/cvs")).unwrap();
    std::fs::create_dir_all(root.join(LOCKS).join("cvs")).unwrap();
    std::fs::write(
            root.join("content/index.yaml"),
            "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [305, 319], chunk: 6 }\n",
        )
        .unwrap();
    std::fs::write(root.join("content/cvs/L04.yaml"), yaml).unwrap();
    if let Some(l) = lessons {
        std::fs::create_dir_all(root.join("content/lessons/cvs")).unwrap();
        std::fs::write(root.join("content/lessons/cvs/L04.yaml"), l).unwrap();
    }
    dir
}

const ITEMS: &str = "items:
  - id: cvs.L04.001
    kind: card
    front: Which valve does rheumatic heart disease affect most?
    back: The mitral valve.
    src: { page: 319, quote: \"mitral > aortic >> tricuspid\" }
  - id: cvs.L04.002
    kind: card
    front: What does Pathoma say about the mitral valve?
    back: Same as First Aid.
    src: { book: pathoma, page: 77 }
  - id: cvs.L04.003
    kind: card
    front: No page yet
    back: x
    src: { page: 0 }
  - { id: cvs.L04.004, kind: card, front: No src, back: x }
";

const LESSONS: &str = "lessons:
  - id: cvs.L04.lesson1
    steps:
      - { kind: explain, text: الـ mitral valve أكتر صمام بيتأثر, page: 319, quote: \"mitral > aortic >> tricuspid\" }
      - { kind: explain, text: من غير صفحة }
      - { kind: figure, image: { file: cvs/p317.png, book: true }, caption: Beck triad, page: 317 }
";

#[test]
fn collect_takes_first_aid_pages_and_lesson_steps_only() {
    let dir = tree(ITEMS, Some(LESSONS));
    let refs = collect(dir.path(), "cvs", "L04").unwrap();
    let ids: Vec<&str> = refs.iter().map(|r| r.id.as_str()).collect();
    // Pathoma's page, page 0, a missing src and a step without a page are not book refs.
    assert_eq!(ids, ["cvs.L04.001", "cvs.L04.lesson1#1", "cvs.L04.lesson1#3"]);
    assert!(refs[0].text.contains("mitral valve") && !refs[0].text.contains("tricuspid"), "{refs:?}");
    assert!(refs[2].book_figure && refs[2].quote.is_none());
    assert!(collect(dir.path(), "cvs", "L09").is_err(), "no such lecture file");
}

#[test]
fn a_lock_file_must_be_well_formed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("L04.tsv");
    std::fs::write(&path, "# ref\tpage\n\ncvs.L04.001\t319\tabc\tquote\tread on p319\n").unwrap();
    let lock = read_lock(&path).unwrap();
    assert_eq!(lock["cvs.L04.001"].note, "read on p319");
    std::fs::write(&path, "cvs.L04.001\t319\tabc\tquote\ncvs.L04.001\t319\tabc\tquote\n").unwrap();
    assert!(read_lock(&path).unwrap_err().ends_with(":2: cvs.L04.001 is listed twice"));
    std::fs::write(&path, "cvs.L04.001\tp319\tabc\tquote\n").unwrap();
    assert!(read_lock(&path).unwrap_err().contains(":1: expected ref, page, quote hash, basis"));
    assert!(read_lock(&dir.path().join("none.tsv")).is_err());
}

#[test]
fn check_locks_names_every_way_a_lock_goes_stale() {
    let dir = tree(ITEMS, Some(LESSONS));
    let root = dir.path();
    let lock = root.join(LOCKS).join("cvs/L04.tsv");
    let lectures = ["cvs.L04".to_string(), "cvs.L05".to_string()];
    // A lecture with content needs a lock; one without content needs none.
    let p = check_locks(root, &lectures);
    assert_eq!(p.len(), 1, "{p:#?}");
    assert!(p[0].contains("L04.tsv: missing page-reference lock; run steps-tools refs --lecture cvs.L04"));
    // A lock for a lecture that has no content.
    std::fs::write(root.join(LOCKS).join("cvs/L05.tsv"), "").unwrap();
    // A broken lock.
    std::fs::write(&lock, "nonsense\n").unwrap();
    let p = check_locks(root, &lectures);
    assert!(p.iter().any(|m| m.contains("L05.tsv: lock exists but lecture content is missing")), "{p:#?}");
    assert!(p.iter().any(|m| m.contains("expected ref, page, quote hash")), "{p:#?}");
    // A lock that misses a ref, lists a stale one, and a lesson step with no quote.
    let mitral = quote_hash(Some("mitral > aortic >> tricuspid"));
    std::fs::write(
            &lock,
            format!(
                "cvs.L04.001\t319\t{mitral}\tquote\t\ncvs.L04.lesson1#3\t317\t-\tfigure\t\ncvs.L04.099\t310\t-\tquote\t\n"
            ),
        )
        .unwrap();
    let p = check_locks(root, &lectures[..1]);
    assert!(p.iter().any(|m| m.contains("cvs.L04.lesson1#1 (p319) was never checked")), "{p:#?}");
    assert!(p.iter().any(|m| m.contains("cvs.L04.099 is no longer in the content")), "{p:#?}");
    assert!(!p.iter().any(|m| m.contains("lesson1#3 has no quote")), "a book figure needs none: {p:#?}");
    // A step that cites a page with no quote, and a quote about another fact.
    let lessons = "lessons:
  - id: cvs.L04.lesson1
    steps:
      - { kind: explain, text: Pulsus paradoxus is a fall in systolic pressure on inspiration in tamponade, page: 317 }
      - { kind: explain, text: Beck triad points to cardiac tamponade after chest trauma, page: 317, quote: Kussmaul sign }
";
    std::fs::write(root.join("content/lessons/cvs/L04.yaml"), lessons).unwrap();
    let p = check_locks(root, &lectures[..1]);
    assert!(p.iter().any(|m| m.contains("cvs.L04.lesson1#1 has no quote")), "{p:#?}");
    assert!(p.iter().any(|m| m.contains("lesson1#2's quote shares no term")), "{p:#?}");
    // Content that no longer parses.
    std::fs::write(root.join("content/cvs/L04.yaml"), "items: [").unwrap();
    let p = check_locks(root, &lectures[..1]);
    assert_eq!(p.len(), 1, "{p:#?}");
    assert!(p[0].contains("L04.yaml"), "{p:#?}");
}

#[test]
fn ocr_and_hand_checks_join_the_book_text() {
    let dir = tempfile::tempdir().unwrap();
    let (book, ocr) = (dir.path().join("book"), dir.path().join("ocr"));
    std::fs::create_dir_all(&book).unwrap();
    std::fs::create_dir_all(&ocr).unwrap();
    std::fs::write(book.join("L04.txt"), "before any page\n=== book page 317 ===\nBeck triad\n").unwrap();
    std::fs::write(ocr.join("317.txt"), "Electrical alternans").unwrap();
    let pages = book_pages(&book, &[ocr], "L04").unwrap();
    assert_eq!(pages.len(), 1);
    assert!(pages[&317].contains("Beck triad") && pages[&317].contains("Electrical alternans"));
    assert!(book_pages(&book, &[], "L09").is_err());

    let tsv = dir.path().join("L04.tsv");
    std::fs::write(
        &tsv,
        "# ref\tpage\tverdict\n\
             cvs.L04.001\t319\tok\tthe table on p319\n\
             cvs.L04.002\tp317\tok\n\
             cvs.L04.003\t317\tquote-ok\tPulsus Paradoxus!\n\
             cvs.L04.004\t317\tno\n",
    )
    .unwrap();
    let checked = read_checked(&tsv);
    assert_eq!(checked.len(), 1);
    assert_eq!(checked["cvs.L04.001"], (319, "the table on p319".to_string()));
    let quotes = read_image_quotes(&tsv);
    assert_eq!(quotes.len(), 1);
    assert_eq!(quotes["cvs.L04.003"], (317, "pulsusparadoxus".to_string()));
    assert!(read_checked(&dir.path().join("missing.tsv")).is_empty());
}

#[test]
fn the_book_check_takes_image_quotes_and_hand_checks_and_names_the_rest() {
    let dir = tree(
        "items:
  - id: cvs.L04.001
    kind: card
    front: What does the echo show in cardiac tamponade?
    back: Diastolic collapse of the right ventricle.
    src: { page: 317, quote: \"RV diastolic collapse\" }
  - id: cvs.L04.002
    kind: card
    front: Which valve does rheumatic heart disease affect most?
    back: Mitral.
    src: { page: 317, quote: \"Beck triad\" }
  - id: cvs.L04.003
    kind: card
    front: Which valve does rheumatic fever scar most often?
    back: Mitral.
    src: { page: 305, quote: \"Aortic dissection\" }
  - id: cvs.L04.004
    kind: card
    front: Rheumatic valve disease
    back: Mitral first.
    src: { page: 311 }
  - id: cvs.L04.005
    kind: card
    front: Which murmur does rheumatic mitral stenosis give?
    back: A diastolic rumble after an opening snap.
    src: { page: 317, quote: \"Beck triad\" }
",
        None,
    );
    let root = dir.path();
    let (book, checked) = (root.join("book"), root.join("checked"));
    std::fs::create_dir_all(&book).unwrap();
    std::fs::create_dir_all(&checked).unwrap();
    std::fs::write(
            book.join("L04.txt"),
            "=== book page 305 ===\nAortic dissection: tearing chest pain, hypertension.\n\
             === book page 317 ===\nCardiac tamponade. Beck triad: hypotension, distended neck veins.\n\
             === book page 319 ===\nRheumatic fever: mitral valve, rheumatic heart disease, valve scarring.\n",
        )
        .unwrap();
    // 001's quote is in the echo figure (read off the image); 005 was read by a person.
    std::fs::write(
        checked.join("L04.tsv"),
        "cvs.L04.001\t317\tquote-ok\tRV diastolic collapse\ncvs.L04.005\t317\tok\tthe murmur table\n",
    )
    .unwrap();
    let args: Vec<String> = [
        "--lecture",
        "cvs.L04",
        "--book",
        book.to_str().unwrap(),
        "--ocr",
        "/nonexistent",
        "--checked",
        checked.to_str().unwrap(),
        "--write",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let p = run(root, &args).unwrap_err();
    assert!(p.iter().any(|m| m.contains("cvs.L04.002: p317 carries")), "{p:#?}");
    assert!(p.iter().any(|m| m.contains("p319")), "the better page is named: {p:#?}");
    assert!(p.iter().any(|m| m.contains("cvs.L04.003: p305 carries")), "{p:#?}");
    assert!(p.iter().any(|m| m.contains("cvs.L04.004: p311 is not in L04's book text")), "{p:#?}");
    assert!(!p.iter().any(|m| m.contains("cvs.L04.001") || m.contains("cvs.L04.005:")), "{p:#?}");
    let summary = p.last().unwrap();
    assert_eq!(summary, "cvs.L04: 5 refs, 0 quoted, 0 book figures, 1 checked by hand, 3 to look at");
    assert!(!root.join(LOCKS).join("cvs/L04.tsv").exists(), "nothing is written while refs are wrong");

    // Without the wrong ones, the lock records how each was proven.
    let yaml = std::fs::read_to_string(root.join("content/cvs/L04.yaml")).unwrap();
    let keep: String = yaml
        .split("  - id: ")
        .filter(|c| {
            !c.starts_with("cvs.L04.00") || c.starts_with("cvs.L04.001") || c.starts_with("cvs.L04.005")
        })
        .collect::<Vec<_>>()
        .join("  - id: ");
    std::fs::write(root.join("content/cvs/L04.yaml"), keep).unwrap();
    let out = run(root, &args).unwrap();
    assert!(out.contains("2 refs"), "{out}");
    let lock = read_lock(&root.join(LOCKS).join("cvs/L04.tsv")).unwrap();
    assert_eq!(lock["cvs.L04.001"].basis, "image");
    assert_eq!(
        (lock["cvs.L04.005"].basis.as_str(), lock["cvs.L04.005"].note.as_str()),
        ("checked", "the murmur table")
    );
    // A hand check already in the lock still counts without the checked file.
    std::fs::remove_file(checked.join("L04.tsv")).unwrap();
    std::fs::write(checked.join("L04.tsv"), "cvs.L04.001\t317\tquote-ok\tRV diastolic collapse\n").unwrap();
    assert!(run(root, &args).is_ok());
    // A quote nowhere in the lecture.
    std::fs::write(
            root.join("content/cvs/L04.yaml"),
            "items:\n  - { id: cvs.L04.001, kind: card, front: a, back: b, src: { page: 317, quote: Kussmaul sign } }\n",
        )
        .unwrap();
    let p = run(root, &args).unwrap_err();
    assert!(p.iter().any(|m| m.contains("quote is not on p317 (not in the lecture)")), "{p:#?}");
    // No index: nothing to check.
    std::fs::remove_file(root.join("content/index.yaml")).unwrap();
    assert!(run(root, &args).is_err());
}
