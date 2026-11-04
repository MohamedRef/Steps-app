//! A book quote's layout. Quotes have no length cap (Mohamed, 2026-10-29):
//! a quote is as much of the page as proves the fact, often a whole table,
//! and a table is drawn as a table (Mohamed, 2026-11-02). The quote stays
//! one YAML string, so the layout lives in plain-text conventions:
//!
//! - `Table 2: Child-Pugh classification` (or `Box`/`Figure`) on its own
//!   line is the table's title.
//! - Lines starting with `|` are grid rows (`| Ascites | Absent | Severe |`);
//!   the first is the header, and a `|---|` line is skipped.
//! - Other lines of a multi-line quote are list rows; two leading spaces per
//!   level nest them (`Metabolic disease:` then `  Hemochromatosis`).
//! - A one-line quote is a paragraph.

/// One piece of a quote, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Title(String),
    Para(String),
    List(Vec<Row>),
    Grid { head: Vec<String>, rows: Vec<Vec<String>> },
}

/// A list row and how deep it is nested (0 is the top).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub depth: usize,
    pub text: String,
    /// It has nested rows under it (drawn as a group heading).
    pub group: bool,
}

fn is_title(line: &str) -> bool {
    let mut w = line.split_whitespace();
    matches!(w.next(), Some("Table" | "Box" | "Figure"))
        && w.next().is_some_and(|n| {
            let n = n.trim_end_matches([':', '.']);
            !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        })
        && line.contains(':')
}

fn cells(line: &str) -> Vec<String> {
    let t = line.trim().trim_start_matches('|').trim_end_matches('|');
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn is_rule(line: &str) -> bool {
    line.trim().chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Splits a quote into its title, paragraphs, lists and grids.
pub fn parse(quote: &str) -> Vec<Block> {
    let lines: Vec<&str> = quote.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() <= 1 {
        return lines.first().map(|l| vec![Block::Para(l.trim().to_string())]).unwrap_or_default();
    }
    let mut out = Vec::new();
    let mut list: Vec<Row> = Vec::new();
    let mut grid: Vec<Vec<String>> = Vec::new();
    let flush_list = |list: &mut Vec<Row>, out: &mut Vec<Block>| {
        if list.is_empty() {
            return;
        }
        // A lone line (a table's "Points scored" over its grid) is text.
        if let [r] = list.as_slice()
            && r.depth == 0
        {
            out.push(Block::Para(std::mem::take(list).remove(0).text));
            return;
        }
        for i in 0..list.len() {
            list[i].group = list.get(i + 1).is_some_and(|n| n.depth > list[i].depth);
        }
        out.push(Block::List(std::mem::take(list)));
    };
    let flush_grid = |grid: &mut Vec<Vec<String>>, out: &mut Vec<Block>| {
        if grid.is_empty() {
            return;
        }
        let mut g = std::mem::take(grid).into_iter();
        let head = g.next().unwrap_or_default();
        out.push(Block::Grid { head, rows: g.collect() });
    };
    for line in lines {
        let t = line.trim();
        if t.starts_with('|') {
            flush_list(&mut list, &mut out);
            if !is_rule(t) {
                grid.push(cells(t));
            }
            continue;
        }
        flush_grid(&mut grid, &mut out);
        if is_title(t) {
            flush_list(&mut list, &mut out);
            out.push(Block::Title(t.to_string()));
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        list.push(Row { depth: indent / 2, text: t.to_string(), group: false });
    }
    flush_list(&mut list, &mut out);
    flush_grid(&mut grid, &mut out);
    out
}

/// The quote as plain lines, for a hover card that can't draw a table:
/// grid cells joined with " · ", nesting kept as indentation.
pub fn plain(quote: &str) -> String {
    quote
        .lines()
        .filter(|l| !l.trim().is_empty() && !(l.trim().starts_with('|') && is_rule(l)))
        .map(|l| if l.trim().starts_with('|') { cells(l).join(" · ") } else { l.trim_end().to_string() })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_line_is_a_paragraph() {
        assert_eq!(
            parse("Hepatic fibrosis follows hepatocellular necrosis."),
            vec![Block::Para("Hepatic fibrosis follows hepatocellular necrosis.".into())]
        );
        assert!(parse("  ").is_empty());
    }

    #[test]
    fn grouped_table_nests_and_marks_groups() {
        let q = "Table 1: Causes of liver cirrhosis\nAlcohol\nMetabolic disease:\n  Hemochromatosis (iron overload)\n  Galactosemia\nSyphilis (tertiary)";
        let b = parse(q);
        assert_eq!(b[0], Block::Title("Table 1: Causes of liver cirrhosis".into()));
        let Block::List(rows) = &b[1] else { panic!("{b:?}") };
        let shape: Vec<(usize, bool)> = rows.iter().map(|r| (r.depth, r.group)).collect();
        assert_eq!(shape, [(0, false), (0, true), (1, false), (1, false), (0, false)]);
        assert_eq!(rows[2].text, "Hemochromatosis (iron overload)");
    }

    #[test]
    fn grid_has_head_and_skips_the_rule() {
        let q = "Table 2: Child-Pugh classification\n| Variable | 1 | 2 | 3 |\n|---|---|---|---|\n| Ascites | Absent | Mild to moderate | Severe |";
        let b = parse(q);
        assert_eq!(b.len(), 2);
        let Block::Grid { head, rows } = &b[1] else { panic!("{b:?}") };
        assert_eq!(head, &["Variable", "1", "2", "3"]);
        assert_eq!(rows, &[vec!["Ascites", "Absent", "Mild to moderate", "Severe"]]);
        assert_eq!(
            plain(q),
            "Table 2: Child-Pugh classification\nVariable · 1 · 2 · 3\nAscites · Absent · Mild to moderate · Severe"
        );
    }

    #[test]
    fn a_sentence_naming_a_table_is_not_a_title() {
        let b = parse("Causes are summarized in Table 1.\nAlcohol");
        assert!(matches!(&b[0], Block::List(r) if r.len() == 2));
    }
}
