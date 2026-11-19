//! Reads a file of questions: a spreadsheet (`.xlsx`), JSON or YAML, alone
//! or in a `.zip` with the figures its rows name. Turns it into the rows
//! and figures `steps_core::ownq::read` takes.

use std::collections::BTreeMap;
use std::io::{Cursor, Read as _};

use serde_json::Value;
use sha2::{Digest, Sha256};
use steps_core::ownq::{self, FileImage, Image, Row};

/// What went wrong with the file itself (a row's problems come from `ownq::read`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileProblem {
    /// Not `.zip`, `.xlsx`, `.json`, `.yaml` or `.yml`.
    Kind(String),
    /// A zip with no spreadsheet, JSON or YAML in it.
    NoQuestions,
    /// The file couldn't be read, with the reader's reason.
    Unreadable(String),
    ImageTooBig(String),
    NotAnImage(String),
}

/// How its questions are counted, for messages: a sheet's rows by line, a
/// list's entries by number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Sheet,
    List,
}

/// A file read: its rows, the figures it brings by name, and their bytes.
#[derive(Debug, Clone, Default)]
pub struct Read {
    pub rows: Vec<Row>,
    pub images: BTreeMap<String, FileImage>,
    pub bytes: BTreeMap<String, Vec<u8>>,
    pub problems: Vec<FileProblem>,
    pub shape: Option<Shape>,
}

impl Read {
    /// The figure bytes a set of questions uses.
    pub fn figures(&self, questions: &[ownq::OwnQuestion]) -> Vec<(Image, Vec<u8>)> {
        let mut seen = std::collections::BTreeSet::new();
        questions
            .iter()
            .filter_map(|q| q.image.clone())
            .filter(|i| seen.insert(i.sha256.clone()))
            .filter_map(|i| self.bytes.get(&i.sha256).map(|b| (i, b.clone())))
            .collect()
    }
}

const IMAGE_EXT: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

fn ext(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

/// Reads `bytes`, named `name`.
pub fn read(name: &str, bytes: &[u8]) -> Read {
    let mut out = Read::default();
    match ext(name).as_str() {
        "zip" => read_zip(bytes, &mut out),
        e @ ("xlsx" | "json" | "yaml" | "yml") => read_questions(e, bytes, &mut out),
        _ => out.problems.push(FileProblem::Kind(name.to_string())),
    }
    out
}

fn read_zip(bytes: &[u8], out: &mut Read) {
    let mut zip = match zip::ZipArchive::new(Cursor::new(bytes)) {
        Ok(z) => z,
        Err(e) => return out.problems.push(FileProblem::Unreadable(e.to_string())),
    };
    let mut questions: Option<(String, Vec<u8>)> = None;
    for i in 0..zip.len() {
        let Ok(mut entry) = zip.by_index(i) else { continue };
        let name = entry.name().replace('\\', "/");
        let base = name.rsplit('/').next().unwrap_or(&name).to_string();
        if entry.is_dir() || name.starts_with("__MACOSX/") || base.starts_with('.') || base.starts_with("~$")
        {
            continue;
        }
        let e = ext(&base);
        let wanted =
            IMAGE_EXT.contains(&e.as_str()) || matches!(e.as_str(), "xlsx" | "json" | "yaml" | "yml");
        if !wanted {
            continue;
        }
        if entry.size() > (ownq::MAX_IMAGE_BYTES as u64) * 20 {
            out.problems.push(FileProblem::ImageTooBig(name));
            continue;
        }
        let mut data = Vec::new();
        if let Err(err) = entry.read_to_end(&mut data) {
            out.problems.push(FileProblem::Unreadable(format!("{name}: {err}")));
            continue;
        }
        if IMAGE_EXT.contains(&e.as_str()) {
            add_image(out, &name, data);
        } else {
            // The first sheet, else the first JSON, else the first YAML.
            let rank = |e: &str| match e {
                "xlsx" => 0,
                "json" => 1,
                _ => 2,
            };
            if questions.as_ref().is_none_or(|(n, _)| rank(&e) < rank(&ext(n))) {
                questions = Some((name, data));
            }
        }
    }
    match questions {
        Some((name, data)) => read_questions(&ext(&name), &data, out),
        None => out.problems.push(FileProblem::NoQuestions),
    }
}

fn add_image(out: &mut Read, name: &str, data: Vec<u8>) {
    if data.len() > ownq::MAX_IMAGE_BYTES {
        return out.problems.push(FileProblem::ImageTooBig(name.to_string()));
    }
    let Some(kind) = ownq::image_kind(&data) else {
        return out.problems.push(FileProblem::NotAnImage(name.to_string()));
    };
    let sha256 = hex(&Sha256::digest(&data));
    let chunks = data.len().div_ceil(ownq::CHUNK_BYTES).max(1) as u32;
    let image = Image { sha256: sha256.clone(), kind: kind.to_string(), bytes: data.len() as u32, chunks };
    out.images.insert(name.to_string(), FileImage { image });
    out.bytes.insert(sha256, data);
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn read_questions(kind: &str, bytes: &[u8], out: &mut Read) {
    let parsed = match kind {
        "xlsx" => sheet(bytes).map(|rows| (rows, Shape::Sheet)),
        "json" => serde_json::from_slice::<Value>(bytes).map_err(|e| e.to_string()).and_then(list),
        _ => serde_yaml_ng::from_slice::<Value>(bytes).map_err(|e| e.to_string()).and_then(list),
    };
    match parsed {
        Ok((rows, shape)) => {
            out.rows = rows;
            out.shape = Some(shape);
        }
        Err(e) => out.problems.push(FileProblem::Unreadable(e)),
    }
}

/// A column heading as the reader knows it: `A`…`J` and `why_A`…`why_J` in
/// capitals, the rest in small letters.
fn heading(h: &str) -> String {
    let h = h.trim();
    let lower = h.to_lowercase();
    if h.len() == 1 && ownq::LETTERS.contains(&h.to_ascii_uppercase()) {
        h.to_ascii_uppercase()
    } else if let Some(c) = lower.strip_prefix("why_").or_else(|| lower.strip_prefix("why ")) {
        format!("why_{}", c.to_ascii_uppercase())
    } else {
        lower.replace(' ', "_")
    }
}

/// The sheet named Questions (else the first), headings on its first row.
fn sheet(bytes: &[u8]) -> Result<Vec<Row>, String> {
    use calamine::{Data, Reader, Xlsx};
    let mut book: Xlsx<_> = Xlsx::new(Cursor::new(bytes.to_vec())).map_err(|e| e.to_string())?;
    let names = book.sheet_names();
    let wanted = names.iter().find(|n| n.trim().eq_ignore_ascii_case("questions")).or(names.first()).cloned();
    let Some(wanted) = wanted else { return Ok(Vec::new()) };
    let range = book.worksheet_range(&wanted).map_err(|e| e.to_string())?;
    let text = |c: &Data| -> Option<String> {
        let s = match c {
            Data::Empty | Data::Error(_) => return None,
            Data::String(s) => s.clone(),
            Data::Float(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", *f as i64),
            Data::Float(f) => f.to_string(),
            Data::Int(i) => i.to_string(),
            Data::Bool(b) => b.to_string(),
            other => other.to_string(),
        };
        let s = s.trim().to_string();
        (!s.is_empty()).then_some(s)
    };
    let mut rows = range.rows();
    let Some(head) = rows.next() else { return Ok(Vec::new()) };
    let heads: Vec<Option<String>> = head.iter().map(|c| text(c).map(|h| heading(&h))).collect();
    let start = range.start().map_or(0, |(r, _)| r as usize);
    let mut out = Vec::new();
    for (i, cells) in rows.enumerate() {
        let mut fields = BTreeMap::new();
        for (h, c) in heads.iter().zip(cells) {
            if let (Some(h), Some(v)) = (h, text(c)) {
                fields.insert(h.clone(), v);
            }
        }
        if !fields.is_empty() {
            out.push(Row { line: start + i + 2, fields });
        }
    }
    Ok(out)
}

/// `{"questions": [...]}` or a bare list; `options` and `why` as lists, or
/// the sheet's own columns.
fn list(doc: Value) -> Result<(Vec<Row>, Shape), String> {
    let items = match doc {
        Value::Array(a) => a,
        Value::Object(mut o) => match o.remove("questions") {
            Some(Value::Array(a)) => a,
            _ => return Err("expected a list of questions, or {\"questions\": [...]}".into()),
        },
        _ => return Err("expected a list of questions, or {\"questions\": [...]}".into()),
    };
    let scalar = |v: &Value| -> Option<String> {
        let s = match v {
            Value::String(s) => s.trim().to_string(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => return None,
        };
        (!s.is_empty()).then_some(s)
    };
    let mut rows = Vec::new();
    for (i, item) in items.into_iter().enumerate() {
        let Value::Object(o) = item else {
            rows.push(Row { line: i + 1, fields: BTreeMap::new() });
            continue;
        };
        let mut fields = BTreeMap::new();
        for (k, v) in o {
            match (heading(&k).as_str(), v) {
                ("options", Value::Array(a)) => {
                    for (c, v) in ownq::LETTERS.chars().zip(&a) {
                        if let Some(s) = scalar(v) {
                            fields.insert(c.to_string(), s);
                        }
                    }
                    if a.len() > ownq::LETTERS.len() {
                        fields.insert("K".into(), "(more than 10 options)".into());
                    }
                }
                ("why", Value::Array(a)) => {
                    for (c, v) in ownq::LETTERS.chars().zip(&a) {
                        if let Some(s) = scalar(v) {
                            fields.insert(format!("why_{c}"), s);
                        }
                    }
                }
                ("tags", Value::Array(a)) => {
                    let t: Vec<String> = a.iter().filter_map(scalar).collect();
                    if !t.is_empty() {
                        fields.insert("tags".into(), t.join(", "));
                    }
                }
                ("video", Value::Object(v)) => {
                    let get = |k: &str| v.get(k).and_then(scalar);
                    if let Some(id) = get("youtube").or_else(|| get("url")).or_else(|| get("id")) {
                        fields.insert("video".into(), id);
                    }
                    if let Some(t) = get("title") {
                        fields.insert("video_title".into(), t);
                    }
                }
                (h, v) => {
                    if let Some(s) = scalar(&v) {
                        fields.insert(h.to_string(), s);
                    }
                }
            }
        }
        rows.push(Row { line: i + 1, fields });
    }
    Ok((rows, Shape::List))
}
