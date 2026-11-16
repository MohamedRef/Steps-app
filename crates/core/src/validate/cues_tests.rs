use super::*;

fn lec(items: &str) -> Lecture {
    let head = "schema: steps.content/v1\nlecture: cvs.L04\ntitle: Pathology\nbook: { chapter: 1, pages: [300, 320] }\n";
    serde_yaml_ng::from_str(&format!("{head}items:\n{items}")).unwrap()
}

fn mcq(id: &str, options: &str, answer: usize) -> String {
    format!(
        "  - {{ id: {id}, kind: mcq, stem: s, options: [{options}], answer: {answer}, why: [a, b, c, d], src: {{ page: 302 }}, status: unverified }}\n"
    )
}

#[test]
fn counts_a_key_longer_than_every_other_option() {
    let l = lec(&[
        mcq("cvs.L04.001", "Aortic stenosis with a slow-rising pulse, Mitral, Tricuspid, Pulmonic", 0),
        // A tie is no cue.
        mcq("cvs.L04.002", "abcd, efgh, ij, kl", 1),
        mcq("cvs.L04.003", "a, bb, ccc, dddd", 3),
        mcq("cvs.L04.004", "a, bb, ccc, dddd", 2),
    ]
    .concat());
    let c = lecture(&l);
    assert_eq!(c.mcqs, 4);
    assert_eq!(c.key_longest, 2);
    assert_eq!(c.key_at, [1, 1, 1, 1, 0]);
    assert!((c.key_longest_share() - 0.5).abs() < 1e-9);
}

#[test]
fn flags_distractors_padded_with_only_or_alone() {
    let l =
        lec(&mcq("cvs.L04.001", "Prednisone alone, Stratum corneum only., Only aspirin, Lonely heart", 2));
    assert_eq!(lecture(&l).padded, [("cvs.L04.001".to_string(), 0), ("cvs.L04.001".to_string(), 1)]);
    // The key itself is not a cue of this kind.
    let l = lec(&mcq("cvs.L04.002", "Prednisone alone, b, c, d", 0));
    assert!(lecture(&l).padded.is_empty());
    assert!(!padded("Alone"), "a one-word option is the thing itself, not padding");
}

#[test]
fn adds_up_across_lectures() {
    let mut all = Cues::default();
    let one = lecture(&lec(&mcq("cvs.L04.001", "Aortic stenosis, b, c, Drug alone", 0)));
    all.add(&one);
    all.add(&one);
    assert_eq!((all.mcqs, all.key_longest, all.padded.len(), all.key_at[0]), (2, 2, 2, 2));
    assert_eq!(Cues::default().key_longest_share(), 0.0);
}

fn lesson_file(steps: &str) -> LessonFile {
    let head = "schema: steps.lesson/v1\nlecture: cvs.L04\nlessons:\n  - id: cvs.L04.lesson1\n    title: t\n    hook: h\n    minutes: 5\n    pages: [305, 319]\n    topics: [shock]\n    goals: [g]\n    recap: [r]\n    status: unverified\n    steps:\n";
    serde_yaml_ng::from_str(&format!("{head}{steps}")).unwrap()
}

fn check(options: &str, answer: usize) -> String {
    format!(
        "      - {{ kind: check, question: q, options: [{options}], answer: {answer}, why: [a, b, c] }}\n"
    )
}

// Issue #746: the lessons' check questions are counted like the MCQs.
#[test]
fn lesson_checks_are_counted_like_mcqs() {
    let f = lesson_file(
        &[
            "      - { kind: explain, text: x }\n".to_string(),
            check("Lipoic acid deficiency, TPP, FAD", 0),
            check("Wavy fibers only, Coagulative necrosis, Neutrophils alone", 1),
            check("aa, bb, c", 0),
        ]
        .concat(),
    );
    let c = lessons(&f);
    assert_eq!((c.mcqs, c.key_longest), (3, 2));
    assert_eq!(c.key_at, [2, 1, 0, 0, 0]);
    // The step is counted from 1, as the lesson shows it.
    assert_eq!(
        c.padded,
        [("cvs.L04.lesson1 step 3".to_string(), 0), ("cvs.L04.lesson1 step 3".to_string(), 2)]
    );
}
