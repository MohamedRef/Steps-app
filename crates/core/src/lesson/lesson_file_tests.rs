use super::*;

fn lecture() -> Lecture {
    serde_yaml_ng::from_str(include_str!("../../../../fixtures/content/sample.yaml")).unwrap()
}

const GOOD: &str = r##"
schema: steps.lesson/v1
lecture: cvs.L04
lessons:
  - id: cvs.L04.lesson1
    title: الـ tamponade
    hook: ليه الضغط بيقع مع الشهيق؟
    minutes: 6
    pages: [305, 319]
    topics: [shock, rheumatic]
    goals: [تعرف Beck triad]
    steps:
      - kind: explain
        title: Beck triad
        text: "الـ ==tamponade== بيضغط على القلب:\n- hypotension\n- distended neck veins"
        page: 317
        quote: "hypotension, distended neck veins, distant heart sounds"
      - kind: figure
        title: Pulsus paradoxus
        svg: '<svg viewBox="0 0 10 10"><text fill="var(--ink)">RV &lt; LV</text></svg>'
        caption: الـ septum بيتزق ناحية الـ LV
        alt: Septum bowing toward the LV
      - kind: figure
        image: { file: cvs/tamponade.webp, book: true }
        caption: صورة الكتاب
        alt: Echo of tamponade
        page: 317
      - kind: reveal
        prompt: ليه الـ neck veins منفوخة؟
        answer: الدم مش عارف يدخل الـ RA
      - kind: mnemonic
        word: "J♥NES"
        parts:
          - { key: J, text: Joints }
          - { key: "♥", text: Pancarditis }
          - { key: N, text: Nodules }
          - { key: E, text: Erythema marginatum }
          - { key: S, text: Sydenham chorea }
        hook: اسم واحد صاحبك
      - kind: aside
        tone: exam
        text: بيحبوا يسألوا على الـ electrical alternans
      - kind: update
        text: الـ guideline الجديدة بتقول نفس الكلام
        ref: ESC 2025
      - kind: check
        question: What does echo show in tamponade?
        options: [Diastolic RV collapse, Dilated LV, Septal hypertrophy]
        answer: 0
        why: [Yes, Dilated cardiomyopathy, HOCM]
      - kind: order
        prompt: Pulsus paradoxus
        steps: [Inspiration, More RV filling, Septum bows, Less LV output]
        why: الـ RV بياخد مكان الـ LV
      - kind: match
        prompt: Match the valve
        pairs: [[Rheumatic, Mitral], [Bicuspid, Aortic], [IV drug use, Tricuspid]]
        why: الأشهر في كل واحد
      - kind: sort
        prompt: Sort the shock
        bins: [Cardiogenic, Hypovolemic]
        cards:
          - { text: "PCWP ↑", bin: 0 }
          - { text: "PCWP ↓", bin: 1 }
          - { text: "CVP ↓", bin: 1 }
        why: الـ preload
      - kind: case
        stem: A 45-year-old man after a stab wound.
        steps:
          - reveal: "BP 80/50, distended neck veins"
            question: Diagnosis?
            options: [Tamponade, MI]
            answer: 0
            why: Beck triad
      - kind: table
        title: Shock
        columns: [Type, PCWP]
        rows: [[Cardiogenic, ↑], [Hypovolemic, ↓]]
        hide: [[0, 1]]
      - kind: try
        items: [cvs.L04.001, cvs.L04.002]
    recap: [Beck triad = hypotension, JVD, muffled sounds]
    practice: [cvs.L04.001, cvs.L04.002, cvs.L04.003, cvs.L04.004, cvs.L04.005, cvs.L04.006, cvs.L04.007, cvs.L04.008]
    status: unverified
"##;

fn good() -> LessonFile {
    serde_yaml_ng::from_str(GOOD).unwrap()
}

fn problems(file: &LessonFile) -> Vec<String> {
    let lec = lecture();
    let cx = LessonContext { lecture: &lec, approved: None, raw_step_keys: Vec::new() };
    validate(file, &cx).into_iter().map(|p| format!("{}: {}", p.at, p.message)).collect()
}

fn step_of(body: StepBody) -> Vec<String> {
    let items: BTreeSet<&str> = ["cvs.L04.001"].into();
    step_problems(&body, &items)
}

#[test]
fn a_good_lesson_of_every_step_kind_passes() {
    let f = good();
    assert_eq!(problems(&f), Vec::<String>::new());
    let kinds: Vec<_> = f.lessons[0].steps.iter().map(|s| s.body.kind()).collect();
    assert_eq!(
        kinds,
        [
            "explain", "figure", "figure", "reveal", "mnemonic", "aside", "update", "check", "order",
            "match", "sort", "case", "table", "try"
        ]
    );
    // Every kind's keys are known; nothing else is.
    for k in kinds {
        assert!(allowed_keys(k).is_some(), "{k}");
    }
    assert_eq!(allowed_keys("video"), None);
    let questions = f.lessons[0].steps.iter().filter(|s| s.body.is_question()).count();
    assert_eq!(questions, 5);
}

#[test]
fn shown_text_lists_each_kind_in_reading_order() {
    let f = good();
    let text: Vec<Vec<String>> = f.lessons[0].steps.iter().map(Step::shown_text).collect();
    assert_eq!(text[0][0], "Beck triad");
    assert!(text[0].iter().all(|t| !t.contains("distant heart sounds")), "the quote stays out");
    assert!(text[1][1].contains("RV < LV"), "{:?}", text[1]);
    assert_eq!(text[2], ["صورة الكتاب"]);
    assert_eq!(text[3], ["ليه الـ neck veins منفوخة؟", "الدم مش عارف يدخل الـ RA"]);
    assert_eq!(text[4].first().map(String::as_str), Some("Joints"));
    assert!(!text[4].iter().any(|t| t == "J♥NES"), "the word is not shown as text");
    assert_eq!(text[4].last().map(String::as_str), Some("اسم واحد صاحبك"));
    assert_eq!(text[5], ["بيحبوا يسألوا على الـ electrical alternans"]);
    assert_eq!(text[6], ["الـ guideline الجديدة بتقول نفس الكلام", "ESC 2025"]);
    assert_eq!(text[7].len(), 7, "question, 3 options, 3 whys");
    assert_eq!(text[8].last().map(String::as_str), Some("الـ RV بياخد مكان الـ LV"));
    assert!(text[9].contains(&"Tricuspid".to_string()) && text[9].contains(&"الأشهر في كل واحد".to_string()));
    assert_eq!(
        text[10],
        ["Sort the shock", "Cardiogenic", "Hypovolemic", "PCWP ↑", "PCWP ↓", "CVP ↓", "الـ preload"]
    );
    assert_eq!(
        text[11],
        [
            "A 45-year-old man after a stab wound.",
            "BP 80/50, distended neck veins",
            "Diagnosis?",
            "Tamponade",
            "MI",
            "Beck triad"
        ]
    );
    assert_eq!(text[12], ["Shock", "Type", "PCWP", "Cardiogenic", "↑", "Hypovolemic", "↓"]);
    assert!(text[13].is_empty(), "a try step shows the items themselves");
}

#[test]
fn a_try_step_plays_every_item_it_lists() {
    let mut f = good();
    let last = f.lessons[0].steps.len() - 1;
    // An MCQ and a case: neither plays inside a lesson (#177, #449, #520).
    f.lessons[0].steps[last].body =
        StepBody::Try { items: vec!["cvs.L04.003".into(), "cvs.L04.006".into(), "cvs.L04.008".into()] };
    let p = problems(&f);
    assert!(p.iter().any(|m| m.contains("try item cvs.L04.003 is a mcq")), "{p:?}");
    assert!(p.iter().any(|m| m.contains("try item cvs.L04.006 is a case")), "{p:?}");
    assert!(!p.iter().any(|m| m.contains("cvs.L04.008")), "{p:?}");
    f.lessons[0].steps[last].body = StepBody::Try { items: vec!["cvs.L04.008".into()] };
    assert_eq!(problems(&f), Vec::<String>::new());
}

#[test]
fn every_lesson_has_a_figure_and_every_item_is_practised() {
    let mut f = good();
    f.lessons[0].steps.retain(|s| !matches!(s.body, StepBody::Figure { .. }));
    f.lessons[0].practice.retain(|id| id != "cvs.L04.005" && id != "cvs.L04.007");
    let p = problems(&f);
    assert!(
        p.iter().any(|m| m.contains("cvs.L04.lesson1: a lesson needs at least one figure step")),
        "{p:?}"
    );
    assert!(
        p.iter().any(
            |m| m.contains("cvs.L04: 2 items are in no lesson's practice list: cvs.L04.005, cvs.L04.007")
        ),
        "{p:?}"
    );
}

#[test]
fn file_level_problems() {
    let mut f = good();
    f.schema = "steps.lesson/v0".into();
    f.lecture = "cvs.L05".into();
    f.lessons.clear();
    let p = problems(&f);
    assert!(p.iter().any(|m| m.contains("expected \"steps.lesson/v1\"")), "{p:?}");
    assert!(p.iter().any(|m| m.contains("lessons are for cvs.L05 but the lecture is cvs.L04")), "{p:?}");
    assert!(p.iter().any(|m| m.ends_with("no lessons")), "{p:?}");
}

#[test]
fn lesson_level_problems() {
    let mut f = good();
    let mut l = f.lessons[0].clone();
    l.id = "cvs.L04.lesson01".into();
    l.title = " ".into();
    l.minutes = 30;
    l.pages = [290, 330];
    l.topics = vec!["murmurs".into()];
    l.recap.clear();
    l.steps.truncate(2);
    l.practice = vec!["cvs.L01.001".into()];
    l.status = Status::Approved;
    f.lessons.push(l.clone());
    f.lessons.push(l);
    let p = problems(&f);
    for want in [
        "cvs.L04.lesson01: lesson id is used twice",
        "lesson id must be cvs.L04.lessonN",
        "title and hook are required",
        "minutes is 30",
        "pages 290-330 are not inside the lecture (300-320)",
        "topic `murmurs` is not one of the lecture's topics",
        "recap is empty",
        "a lesson needs at least 3 steps",
        "a lesson needs at least one check",
        "status approved, but Mohamed hasn't approved it",
        "practice item cvs.L01.001 is not in cvs.L04",
    ] {
        assert!(p.iter().any(|m| m.contains(want)), "{want}: {p:?}");
    }
    // Approved in content/approved.txt: no problem.
    let mut f = good();
    f.lessons[0].status = Status::Approved;
    let lec = lecture();
    let approved: BTreeSet<String> = ["cvs.L04.lesson1".to_string()].into();
    let cx = LessonContext { lecture: &lec, approved: Some(&approved), raw_step_keys: Vec::new() };
    assert!(validate(&f, &cx).is_empty());
}

#[test]
fn step_level_problems() {
    let mut f = good();
    let steps = &mut f.lessons[0].steps;
    steps[0].page = Some(330);
    steps[3].quote = Some("a quote".into());
    steps[2].page = None;
    let lec = lecture();
    let cx = LessonContext {
        lecture: &lec,
        approved: None,
        raw_step_keys: vec![((0, 0), vec!["kind".into(), "text".into(), "answer".into(), "page".into()])],
    };
    let p: Vec<String> = validate(&f, &cx).into_iter().map(|p| format!("{}: {}", p.at, p.message)).collect();
    for want in [
        "cvs.L04.lesson1 step 1: unknown key `answer` for a explain step",
        "cvs.L04.lesson1 step 1: page 330 is outside the lesson's pages 305-319",
        "cvs.L04.lesson1 step 4: a quote needs its page",
        "cvs.L04.lesson1 step 3: a book figure needs its page",
    ] {
        assert!(p.contains(&want.to_string()), "{want}: {p:?}");
    }
    assert!(!p.iter().any(|m| m.contains("`text`") || m.contains("`page`")), "{p:?}");
}

#[test]
fn each_step_kind_has_its_own_rules() {
    let has = |body: StepBody, want: &str| {
        let p = step_of(body);
        assert!(p.iter().any(|m| m.contains(want)), "{want}: {p:?}");
    };
    has(StepBody::Explain { title: None, text: " ".into() }, "text is empty");
    has(StepBody::Aside { tone: Tone::Trap, text: "".into() }, "text is empty");
    has(
        StepBody::Figure {
            title: None,
            svg: "<svg/>".into(),
            image: Some(Picture::default()),
            caption: "".into(),
            alt: "x".into(),
        },
        "either svg or image",
    );
    has(
        StepBody::Figure { title: None, svg: "".into(), image: None, caption: "c".into(), alt: "".into() },
        "caption and an alt",
    );
    has(
        StepBody::Figure { title: None, svg: "".into(), image: None, caption: "c".into(), alt: "a".into() },
        "one <svg>",
    );
    has(
        StepBody::Figure {
            title: None,
            svg: "".into(),
            image: Some(Picture::default()),
            caption: "c".into(),
            alt: "a".into(),
        },
        "must be .webp",
    );
    has(StepBody::Reveal { prompt: "Why?".into(), answer: " ".into() }, "prompt and answer are required");
    has(
        StepBody::Mnemonic {
            word: "MI".into(),
            parts: vec![MnemonicPart { key: "M".into(), text: "Morphine".into() }],
            hook: None,
        },
        "at least 2 parts",
    );
    has(
        StepBody::Mnemonic {
            word: "MONA".into(),
            parts: vec![
                MnemonicPart { key: "M".into(), text: "Morphine".into() },
                MnemonicPart { key: "O".into(), text: "Oxygen".into() },
            ],
            hook: None,
        },
        "the keys spell \"mo\", not the word \"MONA\"",
    );
    has(StepBody::Update { text: "".into(), reference: "AHA".into() }, "needs text and ref");
    has(
        StepBody::Update { text: "Newer advice".into(), reference: "AHA".into() },
        "name the guideline's year",
    );
    has(
        StepBody::Check {
            question: "Q".into(),
            options: vec!["A".into(), "A".into()],
            answer: 4,
            why: vec![],
        },
        "a check has 2 options",
    );
    for want in
        ["answer is not one of the options", "why needs one line per option", "two options are the same"]
    {
        has(
            StepBody::Check {
                question: "Q".into(),
                options: vec!["A".into(), "A".into()],
                answer: 4,
                why: vec![],
            },
            want,
        );
    }
    has(
        StepBody::Order { prompt: "P".into(), steps: vec!["a".into()], why: None },
        "order needs 3 to 8 steps",
    );
    let pair = |a: &str, b: &str| [a.to_string(), b.to_string()];
    has(
        StepBody::Match { prompt: "P".into(), pairs: vec![pair("a", "x")], why: None },
        "match needs 3 to 6 pairs",
    );
    has(
        StepBody::Match {
            prompt: "P".into(),
            pairs: vec![pair("a", "x"), pair("b", "x"), pair("c", "y")],
            why: None,
        },
        "same right side",
    );
    let card = |bin| SortCard { text: "c".into(), bin };
    for want in ["sort needs 2 to 4 bins", "sort needs 3 to 10 cards", "bin is out of range"] {
        has(
            StepBody::Sort { prompt: "P".into(), bins: vec!["one".into()], cards: vec![card(3)], why: None },
            want,
        );
    }
    has(
        StepBody::Sort {
            prompt: "P".into(),
            bins: vec!["a".into(), "b".into()],
            cards: vec![card(0), card(0), card(0)],
            why: None,
        },
        "every bin needs a card",
    );
    has(StepBody::Case { stem: "S".into(), steps: vec![] }, "a case needs steps");
    let cs = |options: Vec<String>, answer| CaseStep {
        reveal: None,
        question: "Q".into(),
        options,
        answer,
        why: "".into(),
    };
    has(
        StepBody::Case { stem: "S".into(), steps: vec![cs(vec!["a".into()], Some(3))] },
        "case step 1: answer out of range",
    );
    has(
        StepBody::Case { stem: "S".into(), steps: vec![cs(vec![], None), cs(vec!["a".into()], None)] },
        "case step 2: options without an answer",
    );
    for want in ["2+ columns and a row", "one cell per column", "hidden cell is outside"] {
        has(
            StepBody::Table {
                title: "T".into(),
                columns: vec!["Type".into()],
                rows: vec![vec![]],
                hide: vec![[5, 0]],
            },
            want,
        );
    }
    has(StepBody::Try { items: vec![] }, "try 1 to 5 items");
    has(StepBody::Try { items: vec!["cvs.L04.009".into()] }, "item cvs.L04.009 is not in this lecture");
    assert!(step_of(StepBody::Try { items: vec!["cvs.L04.001".into()] }).is_empty());
}

#[test]
fn markup_is_checked_in_every_field_and_triple_equals_is_caught() {
    let p = markup_problems(&StepBody::Explain {
        title: None, text: "الـ ===LVEF== واطي و **bold".into()
    });
    assert!(p.iter().any(|m| m.contains("`===`")), "{p:?}");
    assert!(p.iter().any(|m| m.contains("`**` is not closed")), "{p:?}");
    // SVG text is a drawing, not lesson text.
    let fig = StepBody::Figure {
        title: None,
        svg: "<svg viewBox=\"0 0 1 1\"><text>**</text></svg>".into(),
        image: None,
        caption: "c".into(),
        alt: "==".into(),
    };
    assert!(markup_problems(&fig).is_empty());
}

#[test]
fn pictures_need_an_open_licence_or_the_book() {
    let open = Picture {
        file: "cvs/aortic-stenosis.webp".into(),
        credit: Some("Wikimedia Commons".into()),
        license: Some("CC BY-SA 4.0".into()),
        source: Some("https://commons.wikimedia.org/wiki/File:AS.webp".into()),
        ..Picture::default()
    };
    assert!(picture_problems(&open).is_empty(), "{:?}", picture_problems(&open));
    let book = Picture { file: "cvs/p317.png".into(), book: true, ..Picture::default() };
    assert!(picture_problems(&book).is_empty());

    let bad = Picture {
        file: "../secret.gif".into(),
        license: Some("CC BY-ND 4.0".into()),
        source: Some("http://example.org".into()),
        data: "data:x".into(),
        ..Picture::default()
    };
    let p = picture_problems(&bad);
    for want in [
        "filled in by the pack build",
        "must be .webp, .jpg or .png",
        "must be a path under content/lessons/img/",
        "needs credit, license and source",
        "use public domain, CC0 or a CC BY licence",
        "source must be the https://",
    ] {
        assert!(p.iter().any(|m| m.contains(want)), "{want}: {p:?}");
    }
    let credited_book = Picture { credit: Some("First Aid".into()), ..book };
    assert!(picture_problems(&credited_book).iter().any(|m| m.contains("takes no credit")));
    assert!(license_ok("CC0") && license_ok("PD-old") && license_ok("public-domain"));
}

#[test]
fn embedding_images_fills_data_urls_and_reports_missing_or_big_files() {
    let mut f = good();
    let fig = |file: &str| Step {
        body: StepBody::Figure {
            title: None,
            svg: String::new(),
            image: Some(Picture { file: file.into(), book: true, ..Picture::default() }),
            caption: "c".into(),
            alt: "a".into(),
        },
        page: Some(317),
        quote: None,
    };
    f.lessons[0].steps = vec![fig("cvs/tamponade.webp"), fig("cvs/missing.png"), fig("cvs/huge.JPG")];
    let mut asked = Vec::new();
    let problems = embed_images(std::slice::from_mut(&mut f), |path| {
        asked.push(path.to_string());
        match path {
            "cvs/tamponade.webp" => Some(vec![1, 2, 3]),
            "cvs/huge.JPG" => Some(vec![0; MAX_IMAGE_BYTES + 1]),
            _ => None,
        }
    });
    assert_eq!(asked, ["cvs/tamponade.webp", "cvs/missing.png", "cvs/huge.JPG"]);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert_eq!(problems[0], "cvs.L04.lesson1 step 2: content/lessons/img/cvs/missing.png is missing");
    assert!(problems[1].starts_with("cvs.L04.lesson1 step 3: cvs/huge.JPG is 150 KB"), "{problems:?}");
    let data = |k: usize| match &f.lessons[0].steps[k].body {
        StepBody::Figure { image: Some(p), .. } => p.data.clone(),
        _ => unreachable!(),
    };
    assert_eq!(data(0), "data:image/webp;base64,AQID");
    assert_eq!(data(1), "", "a missing file stays empty");
    assert!(data(2).starts_with("data:image/jpeg;base64,"));
}

#[test]
fn svg_rules_cover_size_links_handlers_and_fixed_colours() {
    let big = format!("<svg viewBox=\"0 0 1 1\">{}</svg>", "<g/>".repeat(MAX_SVG_BYTES / 4 + 1));
    assert!(svg_problems(&big).iter().any(|m| m.contains("keep figures under 24 KB")));
    let p = svg_problems(
        r#"<svg viewBox="0 0 1 1"><a href="https://x.org"><rect onclick ="go()" style="fill:rgb(1,2,3)"/></a></svg>"#,
    );
    for want in ["`href=\"http`", "event handlers (`onclick`)", "fixed colour `rgb(…)`"] {
        assert!(p.iter().any(|m| m.contains(want)), "{want}: {p:?}");
    }
    // `-stroke="` inside another attribute and url(#id) / href="#id" are fine.
    let ok = r##"<svg viewBox="0 0 1 1"><use href="#abc"/><rect fill="url(#fff)" data-stroke="x" stroke="none"/></svg>"##;
    assert!(svg_problems(ok).is_empty(), "{:?}", svg_problems(ok));
}
