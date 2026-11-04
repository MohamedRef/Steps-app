//! Builds the offline content pack (`steps_core::pack`) from
//! `content/index.yaml`, `content/program.yaml` and every `content/<chapter>/LNN.yaml`, and embeds it
//! with `include_bytes!`, the same pattern as Kabar's build.rs (adapted from
//! kabar-app main: crates/app/build.rs). A new lecture file needs no code
//! change. A file that doesn't parse is left out with a warning; CI's
//! `steps-tools validate` and `packs build --check` fail on it.

use std::path::PathBuf;

use steps_core::content::{Index, Lecture};
use steps_core::lesson::LessonFile;
use steps_core::pack::Pack;
use steps_core::program::Program;

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let content = root.join("content");
    println!("cargo:rerun-if-changed={}", content.display());

    let index_path = content.join("index.yaml");
    println!("cargo:rerun-if-changed={}", index_path.display());
    let index: Index =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&index_path).expect("content/index.yaml"))
            .expect("content/index.yaml parses");

    let mut files = Vec::new();
    for dir in std::fs::read_dir(&content).expect("content/ exists").flatten() {
        // content/glossary/ holds the per-course glossaries and
        // content/qbank/ the UWorld QIDs (docs/QBANK.md), not lectures.
        if !dir.path().is_dir() || dir.file_name() == "glossary" || dir.file_name() == "qbank" {
            continue;
        }
        println!("cargo:rerun-if-changed={}", dir.path().display());
        for f in std::fs::read_dir(dir.path()).unwrap().flatten() {
            let p = f.path();
            if p.extension().is_some_and(|e| e == "yaml") {
                println!("cargo:rerun-if-changed={}", p.display());
                files.push(p);
            }
        }
    }
    files.sort();

    let mut lectures = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        match serde_yaml_ng::from_str::<Lecture>(&text) {
            Ok(l) => lectures.push(l),
            Err(e) => println!("cargo:warning={}: left out of the app: {e}", f.display()),
        }
    }

    // Lessons: content/lessons/<chapter>/LNN.yaml (steps.lesson/v1).
    let mut lessons = Vec::new();
    let lessons_dir = content.join("lessons");
    println!("cargo:rerun-if-changed={}", lessons_dir.display());
    for dir in std::fs::read_dir(&lessons_dir).into_iter().flatten().flatten() {
        println!("cargo:rerun-if-changed={}", dir.path().display());
        for f in std::fs::read_dir(dir.path()).into_iter().flatten().flatten() {
            let p = f.path();
            if p.extension().is_some_and(|e| e == "yaml") {
                println!("cargo:rerun-if-changed={}", p.display());
                match serde_yaml_ng::from_str::<LessonFile>(&std::fs::read_to_string(&p).unwrap()) {
                    Ok(l) => lessons.push(l),
                    Err(e) => println!("cargo:warning={}: left out of the app: {e}", p.display()),
                }
            }
        }
    }

    // Open-licence lesson images go into the pack as data URLs.
    let img = lessons_dir.join("img");
    println!("cargo:rerun-if-changed={}", img.display());
    for p in steps_core::lesson::embed_images(&mut lessons, |f| {
        let path = img.join(f);
        println!("cargo:rerun-if-changed={}", path.display());
        std::fs::read(path).ok()
    }) {
        println!("cargo:warning=lesson image: {p}");
    }

    let program_path = content.join("program.yaml");
    println!("cargo:rerun-if-changed={}", program_path.display());
    let program: Program =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&program_path).expect("content/program.yaml"))
            .expect("content/program.yaml parses");

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let pack = out.join("content.pack");
    std::fs::write(&pack, Pack::new(index, lectures).with_lessons(lessons).with_program(program).encode())
        .unwrap();
    // The per-course glossaries (content/glossary/<course>.yaml), embedded
    // beside content/glossary.yaml (crate::hover).
    let glossary_dir = content.join("glossary");
    println!("cargo:rerun-if-changed={}", glossary_dir.display());
    let mut parts: Vec<PathBuf> = std::fs::read_dir(&glossary_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yaml"))
        .collect();
    parts.sort();
    let mut glossary_parts = String::from("pub const GLOSSARY_PARTS: &[&str] = &[\n");
    for p in &parts {
        println!("cargo:rerun-if-changed={}", p.display());
        glossary_parts.push_str(&format!("    include_str!({:?}),\n", p.display().to_string()));
    }
    glossary_parts.push_str("];\n");
    // The web app fetches the pack as its own file (scripts/web/build.sh copies
    // it next to the wasm), so a content change doesn't re-send the code.
    let pack_const = if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        "pub const PACK: &[u8] = &[];\n".to_string()
    } else {
        format!("pub const PACK: &[u8] = include_bytes!({:?});\n", pack.display().to_string())
    };
    std::fs::write(out.join("embedded.rs"), format!("{pack_const}{glossary_parts}")).unwrap();
}
