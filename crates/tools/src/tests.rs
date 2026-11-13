use super::*;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn repo_content_is_valid() {
    if let Err(problems) = validate_all(&root()) {
        panic!("{problems:#?}");
    }
}

#[test]
fn course_glossaries_join_the_shared_one() {
    let dir = tempfile::tempdir().unwrap();
    let content = dir.path();
    std::fs::create_dir_all(content.join("glossary")).unwrap();
    std::fs::copy(root().join("content/program.yaml"), content.join("program.yaml")).unwrap();
    let entry = |t: &str| {
        format!(
            "schema: steps.glossary/v1\nentries:\n  - {{ term: {t}, kind: abbr, text: x, status: unverified }}\n"
        )
    };
    std::fs::write(content.join("glossary.yaml"), entry("ASD")).unwrap();
    std::fs::write(content.join("glossary/cvs.yaml"), entry("VSD")).unwrap();
    let g = read_glossary(content).unwrap().unwrap();
    assert_eq!(g.entries.len(), 2);
    // A term two course files both add (threads merged side by side)
    // stays once, from the file read first, and main stays valid.
    std::fs::write(content.join("glossary/renal.yaml"), entry("VSD")).unwrap();
    let g = read_glossary(content).unwrap().unwrap();
    assert_eq!(g.entries.len(), 2);
    assert!(validate::glossary(&g, None).is_empty(), "{:?}", validate::glossary(&g, None));
    // A word one course keeps plain and another explains, either way
    // round: the entry wins and main stays valid.
    let plain = |w: &str| format!("schema: steps.glossary/v1\nentries: []\nplain: [{w}]\n");
    std::fs::write(content.join("glossary/mic.yaml"), plain("VSD")).unwrap();
    std::fs::write(content.join("glossary/resp.yaml"), entry("PEEP")).unwrap();
    std::fs::write(content.join("glossary/neuro.yaml"), plain("PEEP")).unwrap();
    let g = read_glossary(content).unwrap().unwrap();
    assert!(validate::glossary(&g, None).is_empty(), "{:?}", validate::glossary(&g, None));
    assert!(g.plain.is_empty(), "{:?}", g.plain);
    assert!(g.find("PEEP").is_some() && g.find("VSD").is_some());
    // Within one file it is still a mix-up.
    let both = entry("TRAP") + "plain: [TRAP]\n";
    std::fs::write(content.join("glossary/heme.yaml"), both).unwrap();
    let g = read_glossary(content).unwrap().unwrap();
    assert!(validate::glossary(&g, None).iter().any(|p| p.message.contains("isn't plain")));
    for f in ["mic", "resp", "neuro", "heme"] {
        std::fs::remove_file(content.join(format!("glossary/{f}.yaml"))).unwrap();
    }
    // Twice within one file is still a mistake.
    let twice = entry("GFR") + "  - { term: GFR, kind: abbr, text: y, status: unverified }\n";
    std::fs::write(content.join("glossary/renal.yaml"), twice).unwrap();
    let g = read_glossary(content).unwrap().unwrap();
    assert!(validate::glossary(&g, None).iter().any(|p| p.message.contains("listed twice")));
    std::fs::remove_file(content.join("glossary/renal.yaml")).unwrap();
    // A file must be named after a course.
    std::fs::write(content.join("glossary/cardiology.yaml"), entry("BNP")).unwrap();
    let e = read_glossary(content).unwrap_err();
    assert!(e[0].contains("cardiology.yaml: not a course"), "{e:?}");
}

#[test]
fn a_tree_with_problems_is_reported() {
    let dir = std::env::temp_dir().join(format!("steps-validate-{}", std::process::id()));
    let cvs = dir.join("content/cvs");
    std::fs::create_dir_all(&cvs).unwrap();
    std::fs::write(
            dir.join("content/index.yaml"),
            "schema: steps.content/v1\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [300, 320], chunk: 6 }\n",
        )
        .unwrap();
    let sample = std::fs::read_to_string(root().join("fixtures/content/sample.yaml")).unwrap();
    std::fs::write(
        cvs.join("L04.yaml"),
        sample.replace("front: Which valve", "fornt: x\n    front: Which valve"),
    )
    .unwrap();
    std::fs::write(cvs.join("L99.yaml"), "").unwrap();
    let problems = validate_all(&dir).unwrap_err();
    assert!(problems.iter().any(|p| p.contains("unknown key `fornt`")), "{problems:#?}");
    assert!(problems.iter().any(|p| p.contains("L99.yaml: not listed")), "{problems:#?}");
    assert!(problems.iter().any(|p| p.contains("program.yaml")), "{problems:#?}");
    std::fs::remove_dir_all(&dir).unwrap();
}

use crate::testkit::{content_tree, write};

#[test]
fn every_file_level_problem_is_named() {
    let dir = content_tree();
    let root = dir.path();
    write(
        root,
        "content/index.yaml",
        "schema: steps.content/v0\nlectures:\n  - { id: cvs.L04, title: Pathology, pages: [300, 321], chunk: 6 }\n  - { id: cvs.L05, title: Pharmacology, pages: [321, 328], chunk: 6 }\n  - { id: cvs.L06, title: Extra, pages: [329, 330], chunk: 6 }\n",
    );
    write(
        root,
        "content/sources.yaml",
        "schema: steps.sources/v0\nsources:\n  - { id: pathoma, title: Pathoma, short: P, kind: book }\n  - { id: pathoma, title: Pathoma, short: P, kind: book }\n",
    );
    // L05 says it is L04; L06 doesn't parse.
    write(root, "content/cvs/L05.yaml", crate::testkit::SAMPLE);
    write(root, "content/cvs/L06.yaml", "lecture: [");
    let problems = validate_all(root).unwrap_err();
    for want in [
        "content/index.yaml: schema must be steps.content/v1",
        "content/sources.yaml: schema must be steps.sources/v1",
        "content/sources.yaml: pathoma appears twice",
        "content/sources.yaml: the primary source fa2026 is missing",
        "content/cvs/L04.yaml: book pages [300, 320] differ from content/index.yaml [300, 321]",
        "content/cvs/L05.yaml: lecture is cvs.L04, the file name says cvs.L05",
        "content/cvs/L04.yaml: cvs.L04.001: src.book `fa2026` is not in content/sources.yaml",
        "content/cvs/L06.yaml: ",
    ] {
        assert!(problems.iter().any(|p| p.starts_with(want)), "{want}: {problems:#?}");
    }
    // No sources file, no program, and no index at all.
    std::fs::remove_file(root.join("content/sources.yaml")).unwrap();
    std::fs::remove_file(root.join("content/program.yaml")).unwrap();
    let problems = validate_all(root).unwrap_err();
    assert!(problems.iter().any(|p| p.contains("sources.yaml")), "{problems:#?}");
    assert!(problems.iter().any(|p| p.contains("program.yaml")), "{problems:#?}");
    std::fs::remove_file(root.join("content/index.yaml")).unwrap();
    let problems = validate_all(root).unwrap_err();
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("index.yaml"), "{problems:#?}");
}

#[test]
fn glossary_problems_reach_validate() {
    let dir = content_tree();
    let root = dir.path();
    write(
        root,
        "content/glossary.yaml",
        "schema: steps.glossary/v1\nentries:\n  - { term: PDA, kind: abbr, text: Patent ductus arteriosus, status: approved }\n",
    );
    let problems = validate_all(root).unwrap_err();
    assert!(
        problems.iter().any(|p| p.starts_with("content/glossary: glossary.PDA: status approved")),
        "{problems:#?}"
    );
    // A course file in another schema, and a broken one.
    write(root, "content/glossary/cvs.yaml", "schema: steps.glossary/v0\nentries: []\n");
    write(root, "content/glossary/renal.yaml", "entries: [");
    let e = read_glossary(&root.join("content")).unwrap_err();
    assert!(e.iter().any(|p| p.contains("cvs.yaml: schema is \"steps.glossary/v0\"")), "{e:?}");
    assert!(e.iter().any(|p| p.contains("renal.yaml")), "{e:?}");
    assert!(validate_all(root).unwrap_err().iter().any(|p| p.contains("renal.yaml")));
    write(root, "content/glossary.yaml", "entries: [");
    assert!(read_glossary(&root.join("content")).is_err());
}

#[test]
fn raw_keys_skip_what_isnt_an_item() {
    let keys = raw_keys(
        "items:\n  - just text\n  - { kind: card }\n  - { id: cvs.L04.001, kind: card, fornt: x }\n",
    );
    assert_eq!(keys.len(), 1);
    assert_eq!(keys["cvs.L04.001"], ["id", "kind", "fornt"]);
    assert!(raw_keys("items: [").is_empty());
    assert!(raw_keys("lecture: cvs.L04").is_empty());
    assert_eq!(flag(&["--out".to_string()], "--out"), None);
    assert_eq!(rel(Path::new("/a"), Path::new("/b/c")), "/b/c");
}

// Issues #665 and #666: validate prints the key-longest share per lecture
// and refuses padded distractors and a bank-wide share over the limit.
#[test]
fn answer_cues_are_reported_and_gated() {
    use steps_core::validate::cues::Cues;
    let ok = Cues { mcqs: 4, key_longest: 1, padded: vec![], key_at: [1, 1, 1, 1, 0] };
    let (report, problems) =
        cue_report(&[("cvs.L04".into(), ok.clone()), ("cvs.L05".into(), Cues::default())]);
    assert!(problems.is_empty(), "{problems:?}");
    assert!(report.contains("1 of 4 MCQs (25.0%"), "{report}");
    assert!(report.contains("cvs.L04 1/4 (25%)") && !report.contains("cvs.L05"), "{report}");
    let bad =
        Cues { mcqs: 4, key_longest: 2, padded: vec![("cvs.L04.002".into(), 4)], key_at: [4, 0, 0, 0, 0] };
    let (_, problems) = cue_report(&[("cvs.L04".into(), bad)]);
    assert!(problems.iter().any(|p| p.starts_with("cvs.L04.002: option E ends in")), "{problems:?}");
    assert!(problems.iter().any(|p| p.contains("50.0% of MCQs, over the 25% limit")), "{problems:?}");
}
