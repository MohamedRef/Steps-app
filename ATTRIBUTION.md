# Attribution

- Study content is written from the Alexandria Faculty of Medicine, Department of Internal Medicine, "Postgraduate Lectures" e-book (2025), for Mohamed's personal study. The book itself is not included in this repository.
- Lesson figures in `content/lessons/img/` marked `book: true` are cropped from that e-book's own figures, with Mohamed's permission, for his personal study (2026-10-28). Each is cited by its printed page in the lesson. Some of the book's figures credit their own sources (for example a WebPathology watermark on the L01 histology). Open-licence images, if any, are listed below with author, licence and source.
- Some code is adapted from Kabar (same repository, branch `main`); each adapted file says so.
- Rust crates: see `Cargo.lock` (Dioxus, serde, serde_yaml_ng and others, under MIT or Apache-2.0).

## Fonts

The UI fonts are bundled from the [Fontsource](https://fontsource.org) npm packages, version 5.3.0 (the same files Kabar ships), and each license is copied next to its files. `crates/app/src/fonts.rs` embeds them.
- **IBM Plex Sans Arabic** (400, 600 and 700, Arabic subset; `@fontsource/ibm-plex-sans-arabic` 5.3.0): © 2019 IBM Corp., SIL Open Font License 1.1. Used for the Arabic UI. Files are in `assets/fonts/ibm-plex-sans-arabic/` with `OFL.txt`.
- **Plus Jakarta Sans** (variable, Latin and Latin Extended; `@fontsource-variable/plus-jakarta-sans` 5.3.0): © 2020 The Plus Jakarta Sans Project Authors ([tokotype/PlusJakartaSans](https://github.com/tokotype/PlusJakartaSans)), SIL Open Font License 1.1. Used for medical terms and other Latin text. Files are in `assets/fonts/plus-jakarta-sans/` with `OFL.txt`.
