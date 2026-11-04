//! UWorld question IDs (QIDs) per First Aid lecture, from the public
//! "Boards and Beyond Videos and UWorld Question IDs; no duplicates" doc:
//! each B&B video lists the Step 1 QIDs on its topic, and a hand-kept map
//! (`content/qbank/bb-map.yaml`) files each video under a lecture. Only
//! numbers are kept, never question text.
//!
//! `steps-tools qbank sync` reads the doc's plain-text export with
//! [`parse_doc`], applies the map with [`build`] and writes
//! `content/qbank/uworld.yaml`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "steps.qbank/v1";
pub const MAP_SCHEMA: &str = "steps.bbmap/v1";

/// One video of the doc, in its order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    /// The B&B course heading: `Cardiology`.
    pub course: String,
    /// `II. Cardiac Ischemia`.
    pub section: String,
    pub video: String,
    pub qids: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Doc {
    pub videos: Vec<Video>,
    /// QIDs the doc lists as retired by UWorld.
    pub retired: BTreeSet<u32>,
}

fn numbers(s: &str) -> impl Iterator<Item = u32> + '_ {
    s.split(|c: char| !c.is_ascii_digit()).filter(|w| !w.is_empty()).filter_map(|w| w.parse().ok())
}

fn is_numbers(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    l == "none"
        || l == "none found"
        || (s.chars().any(|c| c.is_ascii_digit())
            && s.chars().all(|c| c.is_ascii_digit() || c == ',' || c.is_whitespace()))
}

/// Reads the doc's plain-text export (`…/export?format=txt`). The list
/// starts at the first course heading followed by a `* I.` section, and
/// stops at the closing note; the retired QIDs follow "QIDs retired".
pub fn parse_doc(text: &str) -> Result<Doc, String> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let section_line = |s: &str| {
        s.strip_prefix('*').map(str::trim).is_some_and(|r| {
            r.split_once('.').is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| "IVXL".contains(c)))
        })
    };
    let start = (0..lines.len())
        .find(|&i| {
            !lines[i].is_empty()
                && !lines[i].starts_with('*')
                && lines[i + 1..].iter().find(|l| !l.is_empty()).is_some_and(|l| section_line(l))
        })
        .ok_or("no course heading followed by a `* I.` section")?;
    let mut doc = Doc::default();
    let (mut course, mut section) = (String::new(), String::new());
    let mut open: Option<usize> = None;
    let mut retired = false;
    for &l in &lines[start..] {
        if l.is_empty() {
            continue;
        }
        if l.starts_with("QIDs retired") {
            retired = true;
            open = None;
            doc.retired.extend(numbers(l.split_once(':').map_or("", |(_, r)| r)));
            continue;
        }
        if retired {
            if is_numbers(l) {
                doc.retired.extend(numbers(l));
            }
            continue;
        }
        if l.starts_with("Made it all the way") || l.starts_with("____") {
            open = None;
            continue;
        }
        if section_line(l) {
            section = l.trim_start_matches('*').trim().trim_end_matches(':').trim().to_string();
            open = None;
        } else if let Some(rest) = l.strip_prefix('*') {
            let (name, tail) = rest.split_once(':').unwrap_or((rest, ""));
            doc.videos.push(Video {
                course: course.clone(),
                section: section.clone(),
                video: name.trim().to_string(),
                qids: numbers(tail).collect(),
            });
            open = Some(doc.videos.len() - 1);
        } else if is_numbers(l) {
            match open {
                Some(i) => doc.videos[i].qids.extend(numbers(l)),
                None => return Err(format!("numbers outside any video: {l}")),
            }
        } else {
            course = l.to_string();
            section.clear();
            open = None;
        }
    }
    if doc.videos.is_empty() {
        return Err("no videos found".into());
    }
    Ok(doc)
}

/// `content/qbank/bb-map.yaml`: where each B&B video goes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Map {
    pub schema: String,
    /// A whole B&B section to one lecture.
    pub sections: Vec<SectionRule>,
    /// A video that belongs elsewhere than its section.
    #[serde(default)]
    pub videos: Vec<VideoRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionRule {
    pub course: String,
    pub section: String,
    pub lecture: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoRule {
    pub course: String,
    pub video: String,
    pub lecture: String,
}

impl Map {
    /// The lecture `v` goes to: its own rule, else its section's.
    pub fn lecture_of(&self, v: &Video) -> Option<&str> {
        self.videos
            .iter()
            .find(|r| r.course == v.course && r.video == v.video)
            .map(|r| r.lecture.as_str())
            .or_else(|| {
                self.sections
                    .iter()
                    .find(|r| r.course == v.course && r.section == v.section)
                    .map(|r| r.lecture.as_str())
            })
    }

    /// Every lecture id the map names.
    pub fn lectures(&self) -> BTreeSet<&str> {
        self.sections
            .iter()
            .map(|r| r.lecture.as_str())
            .chain(self.videos.iter().map(|r| r.lecture.as_str()))
            .collect()
    }
}

/// `content/qbank/uworld.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QBank {
    pub schema: String,
    /// The source in `content/sources.yaml`.
    pub source: String,
    /// Where the doc lives.
    pub url: String,
    /// Lectures in the map's order of first use.
    pub lectures: Vec<LectureSets>,
    /// QIDs UWorld retired; left out of every set.
    #[serde(default)]
    pub retired: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LectureSets {
    pub id: String,
    pub sets: Vec<Set>,
}

/// One B&B video and its QIDs (none when every one is filed under an
/// earlier video).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Set {
    /// `Cardiology › Atherosclerosis`.
    pub video: String,
    pub qids: Vec<u32>,
}

impl QBank {
    pub fn lecture(&self, id: &str) -> Option<&LectureSets> {
        self.lectures.iter().find(|l| l.id == id)
    }

    /// The lecture and video a QID is filed under.
    pub fn find(&self, qid: u32) -> Option<(&str, &Set)> {
        self.lectures
            .iter()
            .find_map(|l| l.sets.iter().find(|s| s.qids.contains(&qid)).map(|s| (l.id.as_str(), s)))
    }
}

impl LectureSets {
    pub fn count(&self) -> usize {
        self.sets.iter().map(|s| s.qids.len()).sum()
    }
}

/// Files every video, with its QIDs, under its lecture, leaving out retired QIDs and
/// any QID an earlier video already has. Lectures come in `order` (the
/// book's). Fails, naming each one, when a video has no rule.
pub fn build(doc: &Doc, map: &Map, source: &str, url: &str, order: &[&str]) -> Result<QBank, Vec<String>> {
    let mut by_lecture: BTreeMap<&str, Vec<Set>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut missing = Vec::new();
    for v in &doc.videos {
        let Some(lecture) = map.lecture_of(v) else {
            missing.push(format!(
                "no lecture for B&B {} › {} › {}: add it to content/qbank/bb-map.yaml",
                v.course, v.section, v.video
            ));
            continue;
        };
        let qids: Vec<u32> =
            v.qids.iter().copied().filter(|q| !doc.retired.contains(q) && seen.insert(*q)).collect();
        // A video with no QIDs left is still one to watch.
        by_lecture
            .entry(lecture)
            .or_default()
            .push(Set { video: format!("{} › {}", v.course, v.video), qids });
    }
    if !missing.is_empty() {
        return Err(missing);
    }
    let rank = |id: &str| order.iter().position(|o| *o == id).unwrap_or(usize::MAX);
    let mut lectures: Vec<LectureSets> =
        by_lecture.into_iter().map(|(id, sets)| LectureSets { id: id.to_string(), sets }).collect();
    lectures.sort_by_key(|l| (rank(&l.id), l.id.clone()));
    Ok(QBank {
        schema: SCHEMA.into(),
        source: source.into(),
        url: url.into(),
        lectures,
        retired: doc.retired.iter().copied().collect(),
    })
}

/// Everything wrong with the map and the QIDs, given the lecture ids that
/// exist.
pub fn problems(map: &Map, qb: &QBank, lectures: &BTreeSet<&str>) -> Vec<String> {
    let mut out = Vec::new();
    if map.schema != MAP_SCHEMA {
        out.push(format!("bb-map.yaml: schema must be {MAP_SCHEMA}"));
    }
    if qb.schema != SCHEMA {
        out.push(format!("uworld.yaml: schema must be {SCHEMA}"));
    }
    for id in map.lectures() {
        if !lectures.contains(id) {
            out.push(format!("bb-map.yaml: {id} is not a lecture"));
        }
    }
    let mut seen = BTreeSet::new();
    let retired: BTreeSet<u32> = qb.retired.iter().copied().collect();
    for l in &qb.lectures {
        if !lectures.contains(l.id.as_str()) {
            out.push(format!("uworld.yaml: {} is not a lecture", l.id));
        }
        for s in &l.sets {
            for q in &s.qids {
                if !seen.insert(*q) {
                    out.push(format!("uworld.yaml: QID {q} is listed twice"));
                }
                if retired.contains(q) {
                    out.push(format!("uworld.yaml: QID {q} is retired"));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "Intro text.\nHow do I use it? copy 2127, 8425\n\n\nBasic Pharmacology\n* I. General Topics:\n   * Enzymes:\n 18021, 12066\n   * Dose-Response:\n 1707, 1708, 172\nCardiology\n   * II. Cardiac Ischemia:\n   * Atherosclerosis:\n 500, 501,\n 502\n   * STEMI:\n none\n   * Stable Angina: 501, 503\nMade it all the way through?\n________________\nQIDs retired by UWorld after the doc was created (Invalid QIDs):\n\n502, 999 7710\n";

    fn map() -> Map {
        serde_yaml_ng::from_str(
            "schema: steps.bbmap/v1\nsections:\n  - { course: Basic Pharmacology, section: I. General Topics, lecture: pharm.L01 }\n  - { course: Cardiology, section: II. Cardiac Ischemia, lecture: cvs.L04 }\nvideos:\n  - { course: Basic Pharmacology, video: Enzymes, lecture: bio.L06 }\n",
        )
        .unwrap()
    }

    #[test]
    fn the_export_reads_as_courses_sections_and_videos() {
        let d = parse_doc(DOC).unwrap();
        let names: Vec<_> =
            d.videos.iter().map(|v| (v.course.as_str(), v.video.as_str(), v.qids.clone())).collect();
        assert_eq!(
            names,
            [
                ("Basic Pharmacology", "Enzymes", vec![18021, 12066]),
                ("Basic Pharmacology", "Dose-Response", vec![1707, 1708, 172]),
                ("Cardiology", "Atherosclerosis", vec![500, 501, 502]),
                ("Cardiology", "STEMI", vec![]),
                ("Cardiology", "Stable Angina", vec![501, 503]),
            ]
        );
        assert_eq!(d.videos[2].section, "II. Cardiac Ischemia");
        assert_eq!(d.retired, BTreeSet::from([502, 999, 7710]));
    }

    #[test]
    fn videos_go_to_their_lectures_once_without_retired_qids() {
        let qb =
            build(&parse_doc(DOC).unwrap(), &map(), "bb-uworld", "u", &["bio.L06", "pharm.L01", "cvs.L04"])
                .unwrap();
        let ids: Vec<_> = qb.lectures.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(ids, ["bio.L06", "pharm.L01", "cvs.L04"], "book order");
        let cvs = qb.lecture("cvs.L04").unwrap();
        assert_eq!(cvs.sets[0].qids, [500, 501], "502 is retired");
        assert!(cvs.sets[1].qids.is_empty(), "STEMI has none but stays a video to watch");
        assert_eq!(cvs.sets[2].qids, [503], "501 was already filed");
        assert_eq!(cvs.count(), 3);
        assert_eq!(
            qb.find(503).map(|(l, s)| (l, s.video.as_str())),
            Some(("cvs.L04", "Cardiology › Stable Angina"))
        );
        let lectures = BTreeSet::from(["bio.L06", "pharm.L01", "cvs.L04"]);
        assert_eq!(problems(&map(), &qb, &lectures), Vec::<String>::new());
    }

    #[test]
    fn a_new_video_without_a_rule_is_named() {
        let doc = DOC.replace("Cardiology\n   * II. Cardiac Ischemia:", "Cardiology\n   * III. EKGs:");
        let err = build(&parse_doc(&doc).unwrap(), &map(), "s", "u", &[]).unwrap_err();
        assert_eq!(err.len(), 3, "{err:?}");
        assert!(err[0].contains("Cardiology › III. EKGs › Atherosclerosis"), "{err:?}");
    }

    #[test]
    fn checks_catch_unknown_lectures_and_repeats() {
        let mut qb = build(&parse_doc(DOC).unwrap(), &map(), "s", "u", &[]).unwrap();
        qb.lectures[0].sets[0].qids.push(500);
        qb.lectures[0].sets[0].qids.push(999);
        let p = problems(&map(), &qb, &BTreeSet::from(["pharm.L01", "cvs.L04"])).join("\n");
        for want in ["bio.L06 is not a lecture", "QID 500 is listed twice", "QID 999 is retired"] {
            assert!(p.contains(want), "{want}: {p}");
        }
    }
}
