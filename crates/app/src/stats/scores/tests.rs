use super::*;

fn entry(kind: Kind, day: &str, percent: u8) -> SelfAssessment {
    SelfAssessment {
        id: format!("sa.{day}"),
        kind,
        form: None,
        day: Day::parse(day).unwrap(),
        percent,
        pass_probability: None,
        note: None,
        created_ms: 0,
    }
}

#[test]
fn chart_marks_every_test_and_joins_nbme_only() {
    let svg = chart(
        &[
            entry(Kind::Nbme, "2027-06-01", 60),
            entry(Kind::Uwsa, "2027-06-04", 72),
            entry(Kind::Nbme, "2027-06-08", 70),
        ],
        "a < b",
    );
    assert!(svg.contains(r#"aria-label="a &lt; b""#));
    assert_eq!(svg.matches("<circle").count(), 3);
    assert_eq!(svg.matches("pt practice").count(), 1);
    assert_eq!(svg.matches("line-nbme").count(), 1);
    assert!(svg.contains(r#"class="band""#) && svg.contains(r#"class="ready""#));
    // The plot spans 50-80% here: 80% would be the top, 50% the bottom.
    assert!(svg.contains(r#"<line class="ready" x1="0" x2="320" y1="49.6""#));
    // Mirrored: the first test on the right edge, the last on the left.
    assert!(svg.contains(r#"cx="312.0""#) && svg.contains(r#"cx="8.0""#));
}

#[test]
fn a_single_test_sits_in_the_middle_without_a_line() {
    let svg = chart(&[entry(Kind::Nbme, "2027-06-01", 100)], "x");
    assert!(svg.contains(r#"cx="160.0""#) && !svg.contains("line-nbme"));
    // 100% is the top of the plot.
    assert!(svg.contains(r#"cy="8.0""#));
}

#[test]
fn words_for_every_verdict_trend_and_problem() {
    for v in [Readiness::NoScores, Readiness::OneScore, Readiness::Ready, Readiness::Close, Readiness::NotYet]
    {
        assert!(!verdict_text(v).is_empty());
    }
    assert!(trend_text(3).contains('3'));
    assert_eq!(trend_text(-2), tx::TREND_FLAT);
    assert!(trend_text(-5).contains('5'));
    for p in [
        Problem::PercentTooHigh,
        Problem::ProbabilityTooHigh,
        Problem::FormTooLong,
        Problem::NoteTooLong,
        Problem::InTheFuture,
    ] {
        assert!(!problem_text(p).is_empty());
    }
    assert_eq!(kind_name(Kind::Other), tx::KIND_OTHER);
    assert_eq!(kind_name(Kind::Free120), "Free 120");
}
