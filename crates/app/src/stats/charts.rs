//! Inline SVG charts for the planner, built as strings so every mark can
//! carry an SVG `<title>` (the hover tooltip) and the whole chart a
//! `role="img"` with a label. (Adapted from kabar-app main:
//! crates/app/src/stats/charts.rs.) Colours come from theme tokens in
//! `stats.css`, never hex values here.
//!
//! Charts draw with `direction="ltr"` and mirror the time axis themselves,
//! because the UI is Arabic: today sits on the right and the exam on the left.
//! Every chart also has a text summary next to it on screen.

use std::fmt::Write as _;

/// Escapes text for SVG content and attributes.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

fn open(view_w: f32, view_h: f32, class: &str, label: &str, aspect: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" class="{class}" viewBox="0 0 {view_w} {view_h}" role="img" aria-label="{l}" direction="ltr" preserveAspectRatio="{aspect}"><title>{l}</title>"#,
        l = esc(label),
    )
}

/// One day of the load forecast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Load {
    pub reviews: u32,
    pub new: u32,
    pub rest: bool,
}

/// Expected load per day, today first (on the right): reviews as solid bars,
/// the day's new items stacked on top in a lighter tone, and the daily
/// review cap as a dashed line. Rest days get a small dot on the baseline.
pub fn forecast(days: &[Load], cap: u32, label: &str, tip: impl Fn(usize, &Load) -> String) -> String {
    let n = days.len().max(1);
    let (bw, gap, h) = (14.0f32, 4.0f32, 96.0f32);
    let w = n as f32 * (bw + gap);
    let peak = days.iter().map(|d| d.reviews + d.new).max().unwrap_or(0).max(cap).max(1) as f32;
    let scale = |v: u32| v as f32 / peak * (h - 6.0);
    let mut s = open(w, h + 2.0, "chart forecast-svg", label, "none");
    let _ = write!(s, r#"<line class="baseline" x1="0" x2="{w}" y1="{h}" y2="{h}"/>"#);
    for (i, d) in days.iter().enumerate() {
        let x = (n - 1 - i) as f32 * (bw + gap) + gap / 2.0;
        let class = if i == 0 { "fc today" } else { "fc" };
        let _ = write!(
            s,
            r#"<g class="{class}"><title>{}</title><rect class="hit" x="{}" y="0" width="{}" height="{h}"/>"#,
            esc(&tip(i, d)),
            x - gap / 2.0,
            bw + gap
        );
        if d.rest {
            let _ = write!(s, r#"<circle class="rest" cx="{}" cy="{}" r="2"/>"#, x + bw / 2.0, h - 4.0);
        } else {
            let rh = scale(d.reviews);
            let nh = scale(d.new);
            if d.reviews > 0 {
                let _ = write!(
                    s,
                    r#"<rect class="bar" x="{x}" y="{}" width="{bw}" height="{rh}" rx="2"/>"#,
                    h - rh
                );
            }
            if d.new > 0 {
                let _ = write!(
                    s,
                    r#"<rect class="bar-new" x="{x}" y="{}" width="{bw}" height="{nh}" rx="2"/>"#,
                    h - rh - nh
                );
            }
        }
        s.push_str("</g>");
    }
    let cy = h - scale(cap);
    let _ = write!(s, r#"<line class="cap" x1="0" x2="{w}" y1="{cy}" y2="{cy}"/>"#);
    s.push_str("</svg>");
    s
}

/// The pace line. `past` and `planned` are (day offset from today, items
/// seen by the end of that day); offsets run from the first past day to the
/// exam. The past is a solid line, the plan dashed, the goal (every item) a
/// faint horizontal line, and the final window a shaded band.
pub struct Pace<'a> {
    pub past: &'a [(i32, u32)],
    pub planned: &'a [(i32, u32)],
    pub total: u32,
    /// Offsets of the first and last day on the axis.
    pub from: i32,
    pub to: i32,
    /// Offset where the final window starts.
    pub final_from: Option<i32>,
}

pub fn pace(p: &Pace, label: &str) -> String {
    let (w, h, pad) = (320.0f32, 120.0f32, 6.0f32);
    let span = (p.to - p.from).max(1) as f32;
    // Mirrored: the earliest day on the right, the exam on the left.
    let x = |off: i32| w - pad - (off - p.from) as f32 / span * (w - 2.0 * pad);
    let top = p.total.max(1) as f32;
    let y = |v: u32| h - pad - v.min(p.total) as f32 / top * (h - 2.0 * pad);
    let mut s = open(w, h, "chart pace-svg", label, "none");
    if let Some(f) = p.final_from {
        let (x1, x2) = (x(p.to), x(f));
        let _ = write!(
            s,
            r#"<rect class="final-band" x="{x1}" y="0" width="{}" height="{h}"/>"#,
            (x2 - x1).max(0.0)
        );
    }
    let gy = y(p.total);
    let _ = write!(s, r#"<line class="goal" x1="{pad}" x2="{}" y1="{gy}" y2="{gy}"/>"#, w - pad);
    let _ = write!(
        s,
        r#"<line class="baseline" x1="{pad}" x2="{}" y1="{}" y2="{}"/>"#,
        w - pad,
        h - pad,
        h - pad
    );
    let tx = x(0);
    let _ = write!(s, r#"<line class="today-mark" x1="{tx}" x2="{tx}" y1="{pad}" y2="{}"/>"#, h - pad);
    for (class, points) in [("line-past", p.past), ("line-plan", p.planned)] {
        if points.len() < 2 {
            continue;
        }
        let d: Vec<String> = points
            .iter()
            .enumerate()
            .map(|(i, &(off, v))| format!("{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, x(off), y(v)))
            .collect();
        let _ = write!(s, r#"<path class="{class}" d="{}"/>"#, d.join(" "));
    }
    s.push_str("</svg>");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forecast_mirrors_and_labels() {
        let days = [
            Load { reviews: 10, new: 5, rest: false },
            Load { reviews: 0, new: 0, rest: true },
            Load { reviews: 40, new: 0, rest: false },
        ];
        let svg = forecast(&days, 150, "a < b", |i, d| format!("day {i}: {}", d.reviews));
        assert!(svg.contains(r#"aria-label="a &lt; b""#));
        assert_eq!(svg.matches("<rect class=\"bar\"").count(), 2);
        assert_eq!(svg.matches("bar-new").count(), 1);
        assert_eq!(svg.matches("class=\"rest\"").count(), 1);
        // Today is the right-most slot.
        let today = svg.find("fc today").unwrap();
        assert!(svg[today..].contains(r#"x="36""#));
    }

    #[test]
    fn pace_draws_past_and_plan() {
        let svg = pace(
            &Pace {
                past: &[(-10, 0), (-5, 40), (0, 80)],
                planned: &[(0, 80), (50, 400)],
                total: 400,
                from: -10,
                to: 60,
                final_from: Some(50),
            },
            "pace",
        );
        assert!(svg.contains("line-past"));
        assert!(svg.contains("line-plan"));
        assert!(svg.contains("final-band"));
        // The first past day is on the right edge.
        assert!(svg.contains("M314.0"));
    }
}
