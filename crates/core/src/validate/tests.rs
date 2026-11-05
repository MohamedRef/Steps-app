use super::*;
use crate::content::Lecture;

fn lec(yaml: &str) -> Lecture {
    serde_yaml_ng::from_str(yaml).unwrap()
}

const HEAD: &str =
    "schema: steps.content/v1\nlecture: cvs.L04\ntitle: Pathology\nbook: { chapter: 1, pages: [300, 320] }\n";

#[test]
fn the_sample_fixture_is_clean() {
    let raw = include_str!("../../../../fixtures/content/sample.yaml");
    let l: Lecture = serde_yaml_ng::from_str(raw).unwrap();
    let problems = lecture(&l, &Context::default());
    assert!(problems.is_empty(), "{problems:#?}");
    let kinds: BTreeSet<_> = l.items.iter().map(|i| i.body.kind()).collect();
    assert_eq!(kinds.len(), 8, "the fixture should show every kind");
}

#[test]
fn catches_the_common_mistakes() {
    let l = lec(&format!(
            "{HEAD}items:
  - {{ id: cvs.L04.001, kind: card, front: a, back: b, src: {{ page: 30 }}, status: unverified }}
  - {{ id: cvs.L04.001, kind: mcq, stem: s, options: [a, b, c], answer: 3, why: [x], src: {{ page: 302 }}, status: approved }}
  - {{ id: cvs.L1.2, kind: cloze, text: 'no blank', src: {{ page: 302 }}, status: unverified }}
"
        ));
    let msgs: Vec<String> = lecture(&l, &Context::default()).iter().map(|p| p.to_string()).collect();
    let has = |s: &str| msgs.iter().any(|m| m.contains(s));
    assert!(has("outside the lecture"), "{msgs:#?}");
    assert!(has("used twice"));
    assert!(has("4 or 5 options"));
    assert!(has("out of range"));
    assert!(has("one `why` line"));
    assert!(has("only Mohamed approves"));
    assert!(has("three digits"));
    assert!(has("no {{cN"));
}

#[test]
fn enumerate_hooks_read_and_must_say_something() {
    let l = lec(&format!(
            "{HEAD}items:
  - {{ id: cvs.L04.001, kind: enumerate, prompt: p, points: [a, b], pass: 2, hook: 'حكاية', src: {{ page: 303 }}, status: unverified }}
  - {{ id: cvs.L04.002, kind: enumerate, prompt: p, points: [a, b], pass: 2, hook: ' ', src: {{ page: 303 }}, status: unverified }}
"
        ));
    assert!(matches!(&l.items[0].body, Body::Enumerate { hook: Some(h), .. } if h == "حكاية"));
    let msgs: Vec<String> = lecture(&l, &Context::default()).iter().map(|p| p.to_string()).collect();
    assert_eq!(msgs.len(), 1, "{msgs:#?}");
    assert!(msgs[0].contains("hook is empty"));
}

#[test]
fn approved_is_fine_when_mohamed_approved_it() {
    let l = lec(&format!(
        "{HEAD}items:\n  - {{ id: cvs.L04.001, kind: card, front: a, back: b, src: {{ page: 303 }}, status: approved }}\n"
    ));
    let ok: BTreeSet<String> = ["cvs.L04.001".to_string()].into();
    let cx = Context { approved: Some(&ok), ..Context::default() };
    assert!(lecture(&l, &cx).is_empty());
}

#[test]
fn unknown_keys_are_flagged_and_long_quotes_are_not() {
    let l = lec(&format!(
        "{HEAD}items:\n  - {{ id: cvs.L04.001, kind: card, front: a, back: b, src: {{ page: 303, quote: '{}' }}, status: unverified }}\n",
        "word ".repeat(120)
    ));
    let cx = Context {
        raw_keys: [("cvs.L04.001".to_string(), vec!["front".into(), "back".into(), "answer".into()])].into(),
        ..Context::default()
    };
    let msgs: Vec<String> = lecture(&l, &cx).iter().map(|p| p.to_string()).collect();
    assert!(msgs.iter().any(|m| m.contains("unknown key `answer`")), "{msgs:#?}");
    // No length cap on quotes (Mohamed, 2026-10-29): a whole table is fine.
    assert!(!msgs.iter().any(|m| m.contains("quote")), "{msgs:#?}");
}

#[test]
fn lecture_ids() {
    assert!(is_lecture_id("cvs.L04"));
    assert!(is_lecture_id("gi.L11"));
    assert!(!is_lecture_id("cvs.L1"));
    assert!(!is_lecture_id("CVS.L01"));
    assert!(!is_lecture_id("cvsL01"));
}

#[test]
fn glossary_rules() {
    let g: crate::glossary::Glossary = serde_yaml_ng::from_str(
        "schema: steps.glossary/v1
entries:
  - { term: MI, kind: abbr, text: Myocardial infarction, status: unverified }
  - { term: mi, kind: drug, text: Duplicate spelling, status: unverified }
  - { term: two words, kind: drug, text: x, status: approved }",
    )
    .unwrap();
    let msgs: Vec<String> = glossary(&g, None).iter().map(|p| p.to_string()).collect();
    assert!(msgs.iter().any(|m| m.contains("listed twice")), "{msgs:#?}");
    assert!(msgs.iter().any(|m| m.contains("must be one word")));
    assert!(msgs.iter().any(|m| m.contains("only Mohamed approves")));
    let ok: BTreeSet<String> = ["glossary.two words".to_string()].into();
    assert_eq!(glossary(&g, Some(&ok)).len(), 2);
}

#[test]
fn file_level_mistakes() {
    let l = lec("schema: steps.content/v0
lecture: CVS.4
title: Pathology
book: { chapter: 1, pages: [320, 300] }
topics:
  - { id: shock, title: Shock }
  - { id: shock, title: Shock again }
items:
  - { id: CVS.4.001, kind: card, topic: murmurs, front: ' ', back: b, src: { page: 310 }, status: unverified, update: { text: ' ', ref: AHA 2025 } }
  - { id: CVS.4.002, kind: card, front: a, back: b, src: { page: 310 }, status: unverified }
");
    let msgs: Vec<String> = lecture(&l, &Context::default()).iter().map(|p| p.to_string()).collect();
    for want in [
        "CVS.4: schema is \"steps.content/v0\", expected \"steps.content/v1\"",
        "CVS.4: lecture id must look like cvs.L04",
        "CVS.4: book pages 320-300 are not a range",
        "CVS.4: topic ids repeat",
        "CVS.4.001: topic `murmurs` is not in the lecture's topics",
        "CVS.4.002: topic is missing",
        "CVS.4.001: update needs both text and ref",
        "CVS.4.001: card needs front and back",
    ] {
        assert!(msgs.iter().any(|m| m == want), "{want}: {msgs:#?}");
    }
}

#[test]
fn every_kind_has_its_own_shape_rules() {
    let l = lec(&format!(
            "{HEAD}items:
  - {{ id: cvs.L04.001, kind: cloze, text: 'Beck triad: {{{{c1::hypotension', src: {{ page: 317 }}, status: unverified }}
  - {{ id: cvs.L04.002, kind: cloze, text: '{{{{c1::a}}}} {{{{c2::b}}}} {{{{c3::c}}}} {{{{c4::d}}}}', src: {{ page: 317 }}, status: unverified }}
  - {{ id: cvs.L04.003, kind: mcq, stem: ' ', options: [Tamponade, Tamponade, MI, PE], answer: 0, why: [a, b, c, d], src: {{ page: 317 }}, status: unverified }}
  - {{ id: cvs.L04.004, kind: enumerate, prompt: J♥NES, points: [Carditis], pass: 3, src: {{ page: 319 }}, status: unverified }}
  - {{ id: cvs.L04.005, kind: essay, prompt: Aortic dissection, outline: [], src: {{ page: 305 }}, status: unverified }}
  - id: cvs.L04.006
    kind: case
    stem: A 62-year-old man with tearing chest pain.
    steps:
      - {{ question: Diagnosis?, answer: 0, why: Dissection }}
      - {{ question: Next?, options: [CT, Echo], why: CT angiography }}
      - {{ question: Treatment?, options: [Surgery], answer: 4, why: ' ' }}
    src: {{ page: 305 }}
    status: unverified
  - {{ id: cvs.L04.007, kind: table, title: Shock, columns: [Type], rows: [[Cardiogenic, ↑]], blanks: [], src: {{ page: 317 }}, status: unverified }}
  - {{ id: cvs.L04.008, kind: table, title: Shock, columns: [Type, PCWP], rows: [[Cardiogenic, ↑]], blanks: [[0, 0]], src: {{ page: 317 }}, status: unverified }}
  - {{ id: cvs.L04.009, kind: order, prompt: Pulsus paradoxus, steps: [Inspiration, Less LV output], src: {{ page: 317 }}, status: unverified }}
"
        ));
    let msgs: Vec<String> = lecture(&l, &Context::default()).iter().map(|p| p.to_string()).collect();
    for want in [
        "cvs.L04.001: cloze braces are malformed",
        "cvs.L04.002: cloze has more than 3 blanks",
        "cvs.L04.003: mcq stem is empty",
        "cvs.L04.003: mcq options repeat",
        "cvs.L04.004: enumerate needs a prompt and at least 2 points",
        "cvs.L04.004: enumerate pass 3 must be 1..=1",
        "cvs.L04.005: essay needs a prompt and an outline",
        "cvs.L04.006: case step 1 has an answer but no options",
        "cvs.L04.006: case step 2 has options but no answer",
        "cvs.L04.006: case step 3 answer is out of range",
        "cvs.L04.006: case step 3 needs `why`",
        "cvs.L04.007: table needs at least 2 columns and 1 row",
        "cvs.L04.007: table row 1 has 2 cells, expected 1",
        "cvs.L04.007: table has no blanks",
        "cvs.L04.008: table blank [0, 0] is outside the grid",
        "cvs.L04.009: order needs at least 3 steps",
    ] {
        assert!(msgs.iter().any(|m| m.starts_with(want)), "{want}: {msgs:#?}");
    }
    let l = lec(&format!(
        "{HEAD}items:\n  - {{ id: cvs.L04.001, kind: case, stem: ' ', steps: [], src: {{ page: 305 }}, status: unverified }}\n"
    ));
    let msgs: Vec<String> = lecture(&l, &Context::default()).iter().map(|p| p.to_string()).collect();
    assert_eq!(msgs, ["cvs.L04.001: case needs a stem and at least one step"]);
}

#[test]
fn glossary_schema_text_and_plain_words() {
    let long = "word ".repeat(26);
    let g: crate::glossary::Glossary = serde_yaml_ng::from_str(&format!(
        "schema: steps.glossary/v0
entries:
  - {{ term: PDA, kind: abbr, text: '', status: unverified }}
  - {{ term: HOCM, kind: abbr, text: '{long}', status: unverified }}
plain: [PDA, NOT, NOT]"
    ))
    .unwrap();
    let msgs: Vec<String> = glossary(&g, None).iter().map(|p| p.to_string()).collect();
    for want in [
        "glossary: schema is \"steps.glossary/v0\", expected \"steps.glossary/v1\"",
        "glossary.PDA: text is empty",
        "glossary.HOCM: text is 26 words; keep it to 25 or fewer",
        "glossary.plain: `PDA` has an entry, so it isn't plain",
        "glossary.plain: `NOT` is listed twice",
    ] {
        assert!(msgs.iter().any(|m| m == want), "{want}: {msgs:#?}");
    }
}

#[test]
fn an_enumerate_prompt_count_matches_its_points() {
    assert_eq!(stated_count("List the 11 criteria for substance use disorder."), Some(11));
    assert_eq!(stated_count("Name 4 causes of SIADH"), Some(4));
    assert_eq!(stated_count("List the causes of SIADH"), None);
    assert_eq!(stated_count("Which 3 drugs?"), None);
    assert_eq!(stated_count("Enumerate the 3 D's of pellagra."), Some(3));
    assert_eq!(stated_count("Name three causes"), Some(3));
    assert_eq!(stated_count("Enumerate the features of pellagra"), None);
}
