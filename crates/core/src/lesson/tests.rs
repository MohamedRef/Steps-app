#[test]
fn shown_text_covers_what_the_step_shows_and_svg_words_only() {
    let svg = r#"<svg viewBox="0 0 10 10"><text x="1" class="t-sm">PCWP &amp; LVEF</text></svg>"#;
    let step = Step {
        body: StepBody::Figure {
            title: Some("Wedge pressure".into()),
            svg: svg.into(),
            image: None,
            caption: "Nitrates lower it".into(),
            alt: "A gauge".into(),
        },
        page: Some(3),
        quote: Some("not shown".into()),
        remind: vec![],
    };
    let t = step.shown_text();
    assert_eq!(t[0], "Wedge pressure");
    assert!(t[1].contains("PCWP & LVEF") && !t[1].contains("viewBox") && !t[1].contains("t-sm"), "{t:?}");
    assert_eq!(t[2], "Nitrates lower it");
    assert!(!t.iter().any(|x| x.contains("not shown")));
}

use super::*;

fn latin_runs(line: &str) -> Vec<String> {
    spans(line).into_iter().filter(|s| s.latin).map(|s| s.text).collect()
}

#[test]
fn terms_are_isolated_whole() {
    assert_eq!(latin_runs("الـ Stanford type A معناها"), ["Stanford type A"]);
    assert_eq!(latin_runs("الـ SMCs بتتنشط"), ["SMCs"]);
    assert_eq!(latin_runs("لما PaO2 <80 mmHg يبقى"), ["PaO2 <80 mmHg"]);
    assert_eq!(latin_runs("كله عربي"), Vec::<String>::new());
    assert_eq!(latin_runs("Zone 3"), ["Zone 3"]);
    assert_eq!(latin_runs("الشكل (morphology): كده"), ["(morphology)"]);
    assert_eq!(latin_runs("الـ nodules. وبعدين"), ["nodules"]);
    // Arabic quote marks stay with the Arabic on both sides (#293).
    assert_eq!(latin_runs("(شكل «beads on a string»)، والـ"), ["beads on a string"]);
    assert_eq!(latin_runs("الـ LVEF ≥ 21 هنا"), ["LVEF ≥ 21"]);
    assert_eq!(latin_runs("لازم يتحققوا (all 5 must be fulfilled). تمام"), ["(all 5 must be fulfilled)"]);
    assert_eq!(latin_runs("Low risk (≤ 2%)"), ["Low risk (≤ 2%)"]);
    assert_eq!(latin_runs("Low risk (≤ 2%) يعني"), ["Low risk (≤ 2%)"]);
    assert_eq!(latin_runs("الـ LVEF (يعني"), ["LVEF"]);
    assert_eq!(latin_runs("طوله حوالي 20 cm تقريبا"), ["20 cm"]);
    assert_eq!(latin_runs("20 cm"), ["20 cm"]);
    assert_eq!(latin_runs("خليه 1.5 ml/kg/h كده"), ["1.5 ml/kg/h"]);
    assert_eq!(latin_runs("حوالي 3–4 L في اليوم"), ["3–4 L"]);
    assert_eq!(latin_runs("في 24 h"), ["24 h"]);
    assert_eq!(latin_runs("من 5–6 نقط"), ["5–6"]);
    assert_eq!(latin_runs("الـ OR 7.4 هنا"), ["OR 7.4"]);
    assert_eq!(latin_runs("الـ CA 19-9 مش"), ["CA 19-9"]);
    assert_eq!(latin_runs("الـ ECOG 0–1 بس"), ["ECOG 0–1"]);
    assert_eq!(latin_runs("أكتر من (>80 g/day) كده"), ["(>80 g/day)"]);
    assert_eq!(latin_runs("لو (LDL ≥190) يبقى"), ["(LDL ≥190)"]);
    assert_eq!(latin_runs("الـ NYHA 2022"), ["NYHA 2022"]);
    assert_eq!(latin_runs("لو ≥250 cells يبقى"), ["≥250 cells"]);
    assert_eq!(latin_runs("(مع dextrose 20%) كده"), ["dextrose 20%"]);
    assert_eq!(latin_runs("حوالي 1/3 العيانين و 40 واحد"), ["1/3"]);
    assert_eq!(latin_runs("فيه ≤ 2%، والـ عالي > 2%:"), ["≤ 2%", "> 2%"]);
    assert_eq!(latin_runs("لو أقل من < 3 شهور"), ["< 3"]);
    assert_eq!(latin_runs("حوالي 40% منهم"), ["40%"]);
    assert_eq!(latin_runs("لو CHA2DS2-VASc < 2 ثم كده"), ["CHA2DS2-VASc < 2"]);
    assert_eq!(latin_runs("الـ platelets ≥ 150 (يعني"), ["platelets ≥ 150"]);
    assert_eq!(latin_runs("الصوت (S1) ده"), ["(S1)"]);
    assert_eq!(latin_runs("**JONES**: ≥ 2 خلال **20 يوم** بعد"), ["JONES", "≥ 2"]);
    assert_eq!(
        latin_runs(
            "ركّز في الأرقام كأزواج: **15–45** يخفّوا و **55–85** يزمنوا. الاتنين بيكمّلوا بعض لـ 100 (45 + 55 = 100، و 15 + 85 = 100)."
        ),
        ["15–45", "55–85", "45 + 55 = 100", "15 + 85 = 100"]
    );
    assert_eq!(latin_runs("لو عدّى 12 h فكّر"), ["12 h"]);
    assert_eq!(latin_runs("حوالي 2 mm/yr بس"), ["2 mm/yr"]);
    assert_eq!(latin_runs("الـ 25(OH)D واطي"), ["25(OH)D"]);
    assert_eq!(latin_runs("لونه (black أو brown) كده"), ["black", "brown"]);
    assert_eq!(latin_runs("(الـ P في PATCH) كده"), ["P", "PATCH"]);
    // "label: value" in an Arabic line: two terms in Arabic order.
    assert_eq!(latin_runs("الـ edge: **sharp**، وممكن يبقى **irregular**"), ["edge", "sharp", "irregular"]);
    assert_eq!(latin_runs("Hyperfiltration: GFR 120-180، من ضغط"), ["Hyperfiltration", "GFR 120-180"]);
    assert_eq!(latin_runs("TCMR: I tubulitis; II arteritis كده"), ["TCMR", "I tubulitis", "II arteritis"]);
    assert_eq!(latin_runs("Endocarditis → ceftriaxone 2 g لمدة"), ["Endocarditis", "ceftriaxone 2 g"]);
    assert_eq!(latin_runs("Endocarditis → ceftriaxone"), ["Endocarditis → ceftriaxone"]);
    // Not a separator: a ratio, a time, a list.
    assert_eq!(latin_runs("نسبة 1:2 هنا"), ["1:2"]);
    assert_eq!(latin_runs("الحجم: **shrunken, enlarged or normal**. تمام"), ["shrunken, enlarged or normal"]);
}

#[test]
fn bold_does_not_split_terms() {
    let s = spans("**Genome:** single-stranded **positive-sense RNA**. تمام");
    let latin: Vec<_> = s.iter().filter(|s| s.latin).map(|s| (s.text.as_str(), s.bold)).collect();
    assert_eq!(latin, [("Genome", true), ("single-stranded ", false), ("positive-sense RNA", true)]);
    // All English: one phrase, the colon kept inside it.
    let s = spans("**Genome:** single-stranded RNA");
    assert_eq!(
        s.iter().filter(|s| s.latin).map(|s| s.text.as_str()).collect::<String>(),
        "Genome: single-stranded RNA"
    );
    let s = spans("حوالي **58 million** عايشين");
    assert_eq!(s.iter().filter(|s| s.latin).map(|s| s.text.as_str()).collect::<Vec<_>>(), ["58 million"]);
}

#[test]
fn bold_and_blocks() {
    let s = spans("ده **مهم جدا** يا دكتور");
    assert_eq!(s.iter().filter(|s| s.bold).count(), 1);
    assert_eq!(s.iter().find(|s| s.bold).unwrap().text, "مهم جدا");
    let b = blocks("سطر أول\nكمّلة\n\n- نقطة A\n- نقطة B\n\nآخر");
    assert_eq!(b.len(), 3);
    assert!(matches!(&b[1], Block::Bullets(l) if l.len() == 2));
    // A list right under a lead-in line, with no blank line between.
    let b = blocks("الأسباب:\n- Rheumatic\n- Infective");
    assert!(matches!(&b[..], [Block::Para(_), Block::Bullets(l)] if l.len() == 2));
    // Numbered lines make a numbered list, also under a lead-in line.
    let b = blocks("الخطوات:\n1. ECG\n2. Aspirin (ASA)\n3) PCI");
    assert!(matches!(&b[..], [Block::Para(_), Block::Numbered(l)] if l.len() == 3), "{b:?}");
    // A text line after a list starts a new paragraph.
    let b = blocks("- A\n- B\nبعدها\n1. C");
    assert!(
        matches!(&b[..], [Block::Bullets(a), Block::Para(_), Block::Numbered(c)] if a.len() == 2 && c.len() == 1),
        "{b:?}"
    );
    // A year or a dose at the start of a line is not a list.
    assert!(matches!(&blocks("2024 guidelines.")[..], [Block::Para(_)]));
    assert!(matches!(&blocks("100. mg")[..], [Block::Para(_)]));
}

#[test]
fn key_terms_and_bare_punctuation() {
    let s = spans("دي ==aortic stenosis== و **LVEF**");
    let split = StepBody::Explain { title: None, text: "دي ==aortic\nstenosis== كده".into() };
    assert!(super::markup_problems(&split).iter().any(|p| p.contains("not closed")));
    let cap = StepBody::Reveal { prompt: "==a== ==b==".into(), answer: "==c== و ==d==".into() };
    assert!(super::markup_problems(&cap).iter().any(|p| p.contains("4 key terms")));
    let ok = StepBody::Reveal { prompt: "**x** ==y==".into(), answer: "45 + 55 = 100".into() };
    assert!(super::markup_problems(&ok).is_empty());
    assert!(super::license_ok("CC BY-SA 4.0") && super::license_ok("CC BY-NC-SA 3.0"));
    assert!(super::license_ok("Public domain") && !super::license_ok("CC BY-ND 4.0"));
    assert!(!super::license_ok("All rights reserved"));
    let key: Vec<_> = s.iter().filter(|s| s.key).map(|s| s.text.as_str()).collect();
    assert_eq!(key, ["aortic stenosis"]);
    assert!(s.iter().any(|s| s.bold && s.text == "LVEF"));
    // "**(RBCs):**" and "**(RBCs)**:" look the same: the colon isn't bold.
    for line in ["**cells (RBCs):** دي", "**cells (RBCs)**: دي"] {
        let s = spans(line);
        assert!(s.iter().all(|s| !s.bold || s.text.chars().any(char::is_alphanumeric)), "{line}: {s:?}");
    }
}

#[test]
fn svg_rules() {
    let good = r#"<svg viewBox="0 0 10 10"><rect class="fg-box" fill="var(--surface)" stroke="currentColor" width="5" height="5"/><path marker-end="url(#a)" d="M0 0"/></svg>"#;
    assert!(svg_problems(good).is_empty(), "{:?}", svg_problems(good));
    assert!(!svg_problems(r##"<svg viewBox="0 0 1 1"><rect fill="#ff0000"/></svg>"##).is_empty());
    assert!(!svg_problems(r#"<svg viewBox="0 0 1 1"><rect fill="red"/></svg>"#).is_empty());
    assert!(!svg_problems(r#"<svg viewBox="0 0 1 1" onload="x()"></svg>"#).is_empty());
    assert!(!svg_problems(r#"<svg viewBox="0 0 1 1"><script>1</script></svg>"#).is_empty());
    assert!(!svg_problems(r#"<svg><g/></svg>"#).is_empty());
}

#[test]
fn steps_round_trip_through_yaml() {
    let yaml = r#"
kind: check
question: Which West zone has the highest blood flow?
options: [Zone 1, Zone 2, Zone 3]
answer: 2
why: [no, no, yes]
page: 3
"#;
    let step: Step = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(step.body.is_question());
    assert_eq!(step.page, Some(3));
    let back: Step = serde_yaml_ng::from_str(&serde_yaml_ng::to_string(&step).unwrap()).unwrap();
    assert_eq!(step, back);
}

#[test]
fn reminders_parse_item_ids_and_lesson_mnemonics() {
    assert_eq!(Reminder::parse("cvs.L04.033"), Some(Reminder::Item("cvs.L04.033")));
    assert_eq!(
        Reminder::parse("cvs.L04.lesson2:MINT"),
        Some(Reminder::Mnemonic { lesson: "cvs.L04.lesson2", word: "MINT" })
    );
    assert_eq!(
        Reminder::parse("cvs.L04.lesson2:ABCD OF C"),
        Some(Reminder::Mnemonic { lesson: "cvs.L04.lesson2", word: "ABCD OF C" })
    );
    for bad in ["", "cvs.L04", "cvs.L04.lesson2", "cvs.L04.lesson2:", "cvs.L04.033:X", "cvs L04.033"] {
        assert_eq!(Reminder::parse(bad), None, "{bad:?}");
    }
}

#[test]
fn remind_lists_parse_stay_unique_and_skip_question_steps() {
    let explain = |remind: Vec<&str>| Step {
        body: StepBody::Explain { title: None, text: "Murmurs".into() },
        page: None,
        quote: None,
        remind: remind.into_iter().map(String::from).collect(),
    };
    assert!(remind_problems(&explain(vec!["cvs.L04.033", "cvs.L04.lesson2:MINT"])).is_empty());
    assert_eq!(
        remind_problems(&explain(vec!["cvs.L04.033", "cvs.L04.033"])),
        ["remind `cvs.L04.033` is listed twice"]
    );
    assert_eq!(
        remind_problems(&explain(vec!["MINT"])),
        ["remind `MINT` is neither an item id nor `<lesson id>:<WORD>`"]
    );
    let mut question = explain(vec!["cvs.L04.033"]);
    question.body = StepBody::Order { prompt: "p".into(), steps: vec!["a".into(), "b".into()], why: None };
    assert_eq!(
        remind_problems(&question),
        ["remind goes on an explain, figure, aside, mnemonic or open table step, not a order step"]
    );
    let mut table = explain(vec!["cvs.L04.033"]);
    table.body = StepBody::Table {
        title: "t".into(),
        columns: vec!["a".into()],
        rows: vec![vec!["x".into()]],
        hide: vec![],
    };
    assert!(remind_problems(&table).is_empty());
    if let StepBody::Table { hide, .. } = &mut table.body {
        hide.push([0, 0]);
    }
    assert_eq!(remind_problems(&table).len(), 1);
}
