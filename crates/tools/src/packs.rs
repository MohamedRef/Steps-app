//! `steps-tools packs build [--out DIR] [--check]`: the offline content pack
//! (`steps_core::pack`): the content `crates/app/build.rs` embeds, plus the
//! app's stylesheets (`Pack::look`) so the update feed can carry the look.
//!
//! - `build` writes `dist/packs/steps-content.pack` and prints what is in it.
//! - `--check` (CI) writes nothing. It fails when a lecture file doesn't
//!   parse (the app would leave it out) or the pack doesn't round-trip.
//!
//! Also home of [`load`], which the Anki exporter shares.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use steps_core::content::{Index, Lecture};
use steps_core::pack::Pack;

/// `content/index.yaml` and every written lecture, in index order.
pub fn load(root: &Path) -> Result<(Index, Vec<Lecture>), Vec<String>> {
    let content = root.join("content");
    let index: Index = crate::read_yaml(&content.join("index.yaml")).map_err(|e| vec![e])?;
    let mut lectures = Vec::new();
    let mut problems = Vec::new();
    for entry in &index.lectures {
        let path = crate::lecture_path(&content, &entry.id);
        if path.exists() {
            match crate::read_yaml::<Lecture>(&path) {
                Ok(l) => lectures.push(l),
                Err(e) => problems.push(e),
            }
        }
    }
    if problems.is_empty() { Ok((index, lectures)) } else { Err(problems) }
}

pub fn run(root: &Path, args: &[String]) -> Result<String, Vec<String>> {
    if args.first().map(String::as_str) != Some("build") {
        return Err(vec!["usage: steps-tools packs build [--out DIR] [--check]".into()]);
    }
    let check = args.iter().any(|a| a == "--check");
    let out = crate::flag(args, "--out").map(PathBuf::from).unwrap_or_else(|| root.join("dist/packs"));

    let (index, lectures) = load(root)?;
    let mut lessons: Vec<_> = crate::lessons::load(root)?.into_iter().map(|(_, f, _)| f).collect();
    let missing = steps_core::lesson::embed_images(&mut lessons, crate::lessons::read_image(root));
    if !missing.is_empty() {
        return Err(missing);
    }
    let program = crate::read_yaml(&root.join("content/program.yaml")).map_err(|e| vec![e])?;
    let pack = Pack::new(index, lectures).with_lessons(lessons).with_look(look(root)?).with_program(program);
    let problems = pack.problems();
    if !problems.is_empty() {
        // Devices refuse such a pack; never build one.
        return Err(problems.into_iter().map(|p| format!("pack: {p}")).collect());
    }
    let bytes = pack.encode();
    match Pack::decode(&bytes) {
        Ok(back) if back == pack => {}
        Ok(_) => return Err(vec!["the pack doesn't round-trip: some content changes when packed".into()]),
        Err(e) => return Err(vec![format!("the pack doesn't decode: {e}")]),
    }
    let summary = format!(
        "{} lecture(s), {} items, {} lesson(s), {} stylesheet(s), {:.1} KB packed",
        pack.lectures.len(),
        pack.item_count(),
        pack.lessons.iter().map(|f| f.lessons.len()).sum::<usize>(),
        pack.look.len(),
        bytes.len() as f64 / 1024.0
    );
    if check {
        return Ok(format!("pack OK: {summary}"));
    }
    let path = out.join("steps-content.pack");
    std::fs::create_dir_all(&out).map_err(|e| vec![format!("{}: {e}", out.display())])?;
    std::fs::write(&path, &bytes).map_err(|e| vec![format!("{}: {e}", path.display())])?;
    Ok(format!("{}: {summary}", path.display()))
}

/// Every stylesheet of the app, by path under `crates/app`, so the pack can
/// carry the look (`Pack::look`); none outside a full checkout.
fn look(root: &Path) -> Result<BTreeMap<String, String>, Vec<String>> {
    let app = root.join("crates/app");
    let mut out = BTreeMap::new();
    let mut dirs = vec![app.join("src"), app.join("assets")];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries {
            let path = entry.map_err(|e| vec![format!("{}: {e}", dir.display())])?.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "css") {
                let text =
                    std::fs::read_to_string(&path).map_err(|e| vec![format!("{}: {e}", path.display())])?;
                let key =
                    path.strip_prefix(&app).expect("under crates/app").to_string_lossy().replace('\\', "/");
                out.insert(key, text);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_content_packs() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let report = run(&root, &["build".into(), "--check".into()]).unwrap();
        assert!(report.starts_with("pack OK"), "{report}");
        let look = look(&root).unwrap();
        assert!(
            look.contains_key("assets/main.css") && look.contains_key("src/lecture/lecture.css"),
            "{look:?}"
        );
    }

    #[test]
    fn a_broken_lecture_fails_the_check() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("content/hb")).unwrap();
        std::fs::write(
            dir.path().join("content/index.yaml"),
            "schema: steps.content/v1\nlectures:\n  - { id: hb.L01, title: Liver Cirrhosis, pages: [1, 22], chunk: 10 }\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("content/hb/L01.yaml"), "schema: steps.content/v1\nlecture: [")
            .unwrap();
        let problems = run(dir.path(), &["build".into(), "--check".into()]).unwrap_err();
        assert!(problems[0].contains("L01.yaml"), "{problems:?}");
    }
}
