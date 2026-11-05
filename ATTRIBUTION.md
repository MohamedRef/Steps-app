# Attribution

- Study content is written from *First Aid for the USMLE Step 1 2026* (McGraw Hill), for Mohamed's personal study, citing its printed pages. Other sources, when used, are listed in `content/sources.yaml`. The book itself is not included in this repository.
- Lesson figures marked `book: true` are cropped from the book's own figures for Mohamed's personal study, each cited by its printed page in the lesson. Open-licence images, if any, are listed below with author, licence and source.
- Steps started as a copy of Batna (`MohamedRef/Batna-app`, branch `main`), which itself grew from Kabar; adapted files say so ("adapted from batna-app main: <path>").
- Rust crates: see `Cargo.lock` (Dioxus, serde, serde_yaml_ng and others, under MIT or Apache-2.0).

## Fonts

The UI fonts are bundled from the [Fontsource](https://fontsource.org) npm packages, version 5.3.0, and each license is copied next to its files. `crates/app/src/fonts.rs` embeds them.
- **Readex Pro** (variable, Arabic, Latin and Latin Extended; `@fontsource-variable/readex-pro` 5.3.0): © 2019 The Readex Pro Project Authors ([ThomasJockin/readexpro](https://github.com/ThomasJockin/readexpro)), SIL Open Font License 1.1. Used for the whole UI, Arabic and English. Files are in `assets/fonts/readex-pro/` with `OFL.txt`.
