//! APRI (AST to platelet ratio index).
//!
//! Formula: (AST / AST upper limit of normal) × 100 / platelets [10⁹/L]
//! (Wai et al., Hepatology 2003). Bands follow the book's figure (Lecture 2,
//! p. 34): < 0.5 minimal or no fibrosis, 0.5–1.5 moderate or significant
//! fibrosis, > 1.5 severe fibrosis or cirrhosis. The WHO hepatitis C
//! guidelines (2016) use > 2.0 for cirrhosis; a note says so above 2.0.

use super::{Area, CalcError, CalcSpec, Inputs, Outcome, Part, num, show};

pub const SPEC: CalcSpec = CalcSpec {
    id: "apri",
    name: "APRI",
    area: Area::Liver,
    fields: &[
        num("ast_u_l", "AST", "U/L", 1.0, 10000.0, 1.0),
        num("ast_uln_u_l", "AST upper limit of normal", "U/L", 10.0, 100.0, 1.0).hint("Often 40 U/L."),
        num("platelets_10e9_l", "Platelets", "×10⁹/L", 1.0, 1500.0, 1.0),
    ],
    source: "Book Lecture 2, p. 34; Wai 2003; WHO HCV guidelines 2016",
    compute,
};

fn compute(i: &Inputs) -> Result<Outcome, CalcError> {
    let ast = i.get_pos("ast_u_l")?;
    let uln = i.get_pos("ast_uln_u_l")?;
    let plt = i.get_pos("platelets_10e9_l")?;
    let score = ast / uln * 100.0 / plt;
    let (band, label, meaning) = if score < 0.5 {
        ("apri.low", "Below 0.5", "Minimal or no fibrosis.")
    } else if score <= 1.5 {
        ("apri.mid", "0.5–1.5", "Moderate or significant fibrosis.")
    } else {
        ("apri.high", "Above 1.5", "Severe fibrosis or cirrhosis.")
    };
    Ok(Outcome {
        score: Some(score),
        display: show(score, 2),
        band,
        label,
        meaning: meaning.to_owned(),
        page: Some(34),
        parts: vec![Part::new("AST ÷ ULN", ast / uln)],
        notes: if score > 2.0 {
            vec!["WHO 2016 (hepatitis C): above 2.0 suggests cirrhosis.".to_owned()]
        } else {
            Vec::new()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(ast: f64, uln: f64, plt: f64) -> Outcome {
        compute(&Inputs::new().num("ast_u_l", ast).num("ast_uln_u_l", uln).num("platelets_10e9_l", plt))
            .unwrap()
    }

    #[test]
    fn worked_examples() {
        // (80/40) × 100 / 100 = 2.0: above 1.5 is severe fibrosis or
        // cirrhosis in the book (p. 34), whatever WHO's 2.0 line says.
        let o = run(80.0, 40.0, 100.0);
        assert!((o.score.unwrap() - 2.0).abs() < 1e-9);
        assert_eq!(o.band, "apri.high");
        assert!(o.notes.is_empty());
        // 1.75 sat in "significant fibrosis likely" before #90.
        assert_eq!(run(70.0, 40.0, 100.0).band, "apri.high");
        assert_eq!(run(60.0, 40.0, 100.0).band, "apri.mid");
        assert_eq!(run(30.0, 40.0, 250.0).band, "apri.low");
        let high = run(120.0, 40.0, 90.0);
        assert_eq!(high.band, "apri.high");
        assert_eq!(high.notes.len(), 1);
    }
}
