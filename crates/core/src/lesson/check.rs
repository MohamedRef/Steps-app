//! The lesson validator: what `steps-tools validate` checks in every
//! lesson file (keys, markup, steps, pictures and SVG figures).

use super::*;

/// What the validator needs to know about the lecture a lesson file teaches.
pub struct LessonContext<'a> {
    pub lecture: &'a Lecture,
    /// Ids Mohamed approved (`content/approved.txt`).
    pub approved: Option<&'a BTreeSet<String>>,
    /// The keys written on each step, by `(lesson index, step index)`.
    pub raw_step_keys: Vec<((usize, usize), Vec<String>)>,
}

/// An image file in git can't be bigger than this (WebP, about 800 px wide).
pub const MAX_IMAGE_BYTES: usize = 150 * 1024;

/// Image files a figure may use, by extension, with their MIME type.
pub const IMAGE_TYPES: [(&str, &str); 4] =
    [("webp", "image/webp"), ("jpg", "image/jpeg"), ("jpeg", "image/jpeg"), ("png", "image/png")];

/// Problems with a figure's picture (the file itself is checked by [`embed_images`]).
pub fn picture_problems(p: &Picture) -> Vec<String> {
    let mut out = Vec::new();
    if !p.data.is_empty() {
        out.push("image.data is filled in by the pack build; don't write it".into());
    }
    let file = &p.file;
    let ext = file.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    if !IMAGE_TYPES.iter().any(|(e, _)| *e == ext) {
        out.push(format!("image file {file:?} must be .webp, .jpg or .png"));
    }
    if file.is_empty() || file.contains("..") || file.starts_with('/') || file.contains('\\') {
        out.push(format!("image file {file:?} must be a path under content/lessons/img/"));
    }
    let has = |o: &Option<String>| o.as_deref().is_some_and(|s| !s.trim().is_empty());
    if p.book {
        if has(&p.credit) || has(&p.license) || has(&p.source) {
            out.push("a book figure takes no credit, license or source (the step's page cites it)".into());
        }
        return out;
    }
    if !has(&p.credit) || !has(&p.license) || !has(&p.source) {
        out.push("an image needs credit, license and source (or book: true for a book figure)".into());
    }
    if let Some(l) = &p.license
        && !license_ok(l)
    {
        out.push(format!(
            "license {l:?}: use public domain, CC0 or a CC BY licence without ND (no-derivatives)"
        ));
    }
    if let Some(s) = &p.source
        && !s.starts_with("https://")
    {
        out.push("source must be the https:// page the image came from".into());
    }
    out
}

/// Public domain, CC0, or CC BY (also -SA and -NC: Steps is personal and
/// free), but never -ND, since images get resized and cropped.
pub(super) fn license_ok(l: &str) -> bool {
    let u = l.to_ascii_uppercase().replace('-', " ");
    let open = u.starts_with("PUBLIC DOMAIN")
        || u.starts_with("PD")
        || u.starts_with("CC0")
        || u.starts_with("CC BY");
    open && !u.split_whitespace().any(|w| w == "ND")
}

/// Fills in every open-licence picture's `data` from its file under
/// `content/lessons/img/` (`read` is handed that relative path). Returns
/// the problems: missing files and files that are too big.
pub fn embed_images(files: &mut [LessonFile], mut read: impl FnMut(&str) -> Option<Vec<u8>>) -> Vec<String> {
    use base64::Engine;
    let mut out = Vec::new();
    for f in files.iter_mut() {
        for l in &mut f.lessons {
            for (k, step) in l.steps.iter_mut().enumerate() {
                let StepBody::Figure { image: Some(p), .. } = &mut step.body else { continue };
                let file = p.file.clone();
                let at = format!("{} step {}", l.id, k + 1);
                let Some(bytes) = read(&file) else {
                    out.push(format!("{at}: content/lessons/img/{file} is missing"));
                    continue;
                };
                if bytes.len() > MAX_IMAGE_BYTES {
                    out.push(format!(
                        "{at}: {file} is {} KB; keep images under {} KB (WebP, about 800 px wide)",
                        bytes.len() / 1024,
                        MAX_IMAGE_BYTES / 1024
                    ));
                }
                let ext = file.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
                let mime = IMAGE_TYPES.iter().find(|(e, _)| *e == ext).map_or("image/webp", |(_, m)| m);
                p.data = format!(
                    "data:{mime};base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(&bytes)
                );
            }
        }
    }
    out
}

/// Key terms (`==term==`) per step: the marker only helps when it's rare.
pub const MAX_KEY_TERMS: usize = 3;

/// Figures can't be bigger than this (they are drawn, not pasted).
pub const MAX_SVG_BYTES: usize = 24 * 1024;

/// Checks a lesson file against the rules in `docs/LESSONS.md`.
pub fn validate(file: &LessonFile, cx: &LessonContext<'_>) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut bad = |at: &str, message: String| out.push(Problem { at: at.to_string(), message });
    let lec = cx.lecture;
    let lid = file.lecture.as_str();
    if file.schema != SCHEMA {
        bad(lid, format!("schema is {:?}, expected {SCHEMA:?}", file.schema));
    }
    if file.lecture != lec.lecture {
        bad(lid, format!("lessons are for {lid} but the lecture is {}", lec.lecture));
    }
    if file.lessons.is_empty() {
        bad(lid, "no lessons".into());
    }
    let items: BTreeSet<&str> = lec.items.iter().map(|i| i.id.as_str()).collect();
    let topics: BTreeSet<&str> = lec.topics.iter().map(|t| t.id.as_str()).collect();
    let [first, last] = lec.book.pages;
    let mut ids = BTreeSet::new();
    for (li, lesson) in file.lessons.iter().enumerate() {
        let at = lesson.id.as_str();
        if !ids.insert(at) {
            bad(at, "lesson id is used twice".into());
        }
        match at.strip_prefix(lid).and_then(|r| r.strip_prefix(".lesson")) {
            Some(n) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && !n.starts_with('0') => {}
            _ => bad(at, format!("lesson id must be {lid}.lessonN")),
        }
        if lesson.title.trim().is_empty() || lesson.hook.trim().is_empty() {
            bad(at, "title and hook are required".into());
        }
        if !(2..=15).contains(&lesson.minutes) {
            bad(at, format!("minutes is {}: keep a lesson between 2 and 15 minutes", lesson.minutes));
        }
        let [a, b] = lesson.pages;
        if a > b || a < first || b > last {
            bad(at, format!("pages {a}-{b} are not inside the lecture ({first}-{last})"));
        }
        for t in &lesson.topics {
            if !topics.contains(t.as_str()) {
                bad(at, format!("topic `{t}` is not one of the lecture's topics"));
            }
        }
        if lesson.recap.is_empty() {
            bad(at, "recap is empty: end with the take-aways".into());
        }
        if lesson.steps.len() < 3 {
            bad(at, "a lesson needs at least 3 steps".into());
        }
        if !lesson.steps.iter().any(|s| s.body.is_question()) {
            bad(at, "a lesson needs at least one check, order, match, sort or case step".into());
        }
        match lesson.status {
            Status::Approved if !cx.approved.is_some_and(|a| a.contains(at)) => {
                bad(at, "status approved, but Mohamed hasn't approved it (content/approved.txt)".into());
            }
            _ => {}
        }
        for id in &lesson.practice {
            if !items.contains(id.as_str()) {
                bad(at, format!("practice item {id} is not in {lid}"));
            }
        }
        // LESSONS.md rule 10: a picture in every lesson.
        if !lesson.steps.iter().any(|s| matches!(s.body, StepBody::Figure { .. })) {
            bad(at, "a lesson needs at least one figure step (LESSONS.md rule 10)".into());
        }
        for (si, step) in lesson.steps.iter().enumerate() {
            let sat = format!("{at} step {}", si + 1);
            if let Some((_, keys)) = cx.raw_step_keys.iter().find(|(k, _)| *k == (li, si)) {
                let own = allowed_keys(step.body.kind()).unwrap_or(&[]);
                for k in keys {
                    if !own.contains(&k.as_str()) && !SHARED_STEP_KEYS.contains(&k.as_str()) {
                        bad(&sat, format!("unknown key `{k}` for a {} step", step.body.kind()));
                    }
                }
            }
            if let Some(p) = step.page
                && (p < a || p > b)
            {
                bad(&sat, format!("page {p} is outside the lesson's pages {a}-{b}"));
            }
            if step.quote.is_some() && step.page.is_none() {
                bad(&sat, "a quote needs its page".into());
            }
            if let StepBody::Figure { image: Some(p), .. } = &step.body
                && p.book
                && step.page.is_none()
            {
                bad(&sat, "a book figure needs its page".into());
            }
            // LESSONS.md rule 9: a try step plays every item it lists, so
            // each must be a kind the player shows (an MCQ is dropped).
            if let StepBody::Try { items: ids } = &step.body {
                for id in ids {
                    if let Some(i) = lec.items.iter().find(|i| &i.id == id)
                        && !PLAYABLE.contains(&i.body.kind())
                    {
                        bad(
                            &sat,
                            format!(
                                "try item {id} is a {}; a try step plays only {}",
                                i.body.kind(),
                                PLAYABLE.join(", ")
                            ),
                        );
                    }
                }
            }
            for m in step_problems(&step.body, &items) {
                bad(&sat, m);
            }
        }
    }
    // LESSONS.md rule 9: every item of the lecture is practised from the
    // lesson that teaches it.
    let practised: BTreeSet<&str> =
        file.lessons.iter().flat_map(|l| l.practice.iter().map(String::as_str)).collect();
    let missing: Vec<&str> = items.iter().copied().filter(|id| !practised.contains(id)).collect();
    if !file.lessons.is_empty() && !missing.is_empty() {
        let shown: Vec<&str> = missing.iter().copied().take(8).collect();
        let more = if missing.len() > 8 { format!(" and {} more", missing.len() - 8) } else { String::new() };
        bad(
            lid,
            format!("{} items are in no lesson's practice list: {}{more}", missing.len(), shown.join(", ")),
        );
    }
    out
}

/// `**` and `==` in every text a step shows, checked line by line the way
/// [`spans`] reads them, and the step's key terms counted together (#321).
pub(super) fn markup_problems(body: &StepBody) -> Vec<String> {
    fn strings<'a>(v: &'a serde_json::Value, key: &str, out: &mut Vec<(String, &'a str)>) {
        match v {
            // Drawings and pictures aren't lesson text.
            serde_json::Value::String(s) if !matches!(key, "svg" | "image" | "alt" | "kind") => {
                out.push((key.to_string(), s))
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| strings(x, key, out)),
            serde_json::Value::Object(o) => o
                .iter()
                .filter(|(k, _)| !matches!(k.as_str(), "svg" | "image"))
                .for_each(|(k, x)| strings(x, k, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    let Ok(v) = serde_json::to_value(body) else { return out };
    let mut texts = Vec::new();
    strings(&v, "", &mut texts);
    let mut keys = 0;
    for (field, text) in texts {
        for line in text.lines() {
            if line.contains("===") {
                out.push(format!("{field}: `===` (write `==term==` with no extra =)"));
            }
            if line.matches("**").count() % 2 == 1 {
                out.push(format!("{field}: `**` is not closed on its line"));
            }
            let marks = line.matches("==").count();
            if marks % 2 == 1 {
                out.push(format!("{field}: `==` is not closed on its line"));
            }
            keys += marks / 2;
        }
    }
    if keys > MAX_KEY_TERMS {
        out.push(format!(
            "{keys} key terms (`==`): keep the marker for the {MAX_KEY_TERMS} that matter most"
        ));
    }
    out
}

pub(super) fn step_problems(body: &StepBody, items: &BTreeSet<&str>) -> Vec<String> {
    let mut out = markup_problems(body);
    let empty = |s: &str| s.trim().is_empty();
    match body {
        StepBody::Explain { text, .. } | StepBody::Aside { text, .. } => {
            if empty(text) {
                out.push("text is empty".into());
            }
        }
        StepBody::Figure { svg, image, caption, alt, .. } => {
            match image {
                None => out.extend(svg_problems(svg)),
                Some(_) if !svg.trim().is_empty() => {
                    out.push("a figure has either svg or image, not both".into())
                }
                Some(p) => out.extend(picture_problems(p)),
            }
            if empty(caption) || empty(alt) {
                out.push("a figure needs a caption and an alt text".into());
            }
        }
        StepBody::Reveal { prompt, answer } => {
            if empty(prompt) || empty(answer) {
                out.push("prompt and answer are required".into());
            }
        }
        StepBody::Mnemonic { word, parts, .. } => {
            if parts.len() < 2 {
                out.push("a mnemonic needs at least 2 parts".into());
            }
            // The keys spell the word, in order (ignoring spaces and case).
            let norm = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
            let spelled: String = parts.iter().map(|p| norm(&p.key)).collect();
            if spelled != norm(word) {
                out.push(format!("the keys spell {spelled:?}, not the word {word:?}"));
            }
        }
        StepBody::Update { text, reference } => {
            if empty(text) || empty(reference) {
                out.push("an update needs text and ref (guideline and year)".into());
            } else if !reference.chars().any(|c| c.is_ascii_digit()) {
                out.push("ref must name the guideline's year".into());
            }
        }
        StepBody::Check { options, answer, why, .. } => {
            if !(3..=5).contains(&options.len()) {
                out.push(format!("a check has {} options; use 3 to 5", options.len()));
            }
            if *answer >= options.len() {
                out.push("answer is not one of the options".into());
            }
            if why.len() != options.len() {
                out.push("why needs one line per option".into());
            }
            if options.iter().collect::<BTreeSet<_>>().len() != options.len() {
                out.push("two options are the same".into());
            }
        }
        StepBody::Order { steps, .. } => {
            if !(3..=8).contains(&steps.len()) {
                out.push("order needs 3 to 8 steps".into());
            }
        }
        StepBody::Match { pairs, .. } => {
            if !(3..=6).contains(&pairs.len()) {
                out.push("match needs 3 to 6 pairs".into());
            }
            let rights: BTreeSet<&String> = pairs.iter().map(|p| &p[1]).collect();
            if rights.len() != pairs.len() {
                out.push("two pairs have the same right side".into());
            }
        }
        StepBody::Sort { bins, cards, .. } => {
            if !(2..=4).contains(&bins.len()) {
                out.push("sort needs 2 to 4 bins".into());
            }
            if !(3..=10).contains(&cards.len()) {
                out.push("sort needs 3 to 10 cards".into());
            }
            if cards.iter().any(|c| c.bin >= bins.len()) {
                out.push("a card's bin is out of range".into());
            }
            if (0..bins.len()).any(|b| !cards.iter().any(|c| c.bin == b)) {
                out.push("every bin needs a card".into());
            }
        }
        StepBody::Case { steps, .. } => {
            if steps.is_empty() {
                out.push("a case needs steps".into());
            }
            for (i, s) in steps.iter().enumerate() {
                match s.answer {
                    Some(a) if a >= s.options.len() => {
                        out.push(format!("case step {}: answer out of range", i + 1))
                    }
                    None if !s.options.is_empty() => {
                        out.push(format!("case step {}: options without an answer", i + 1))
                    }
                    _ => {}
                }
            }
        }
        StepBody::Table { columns, rows, hide, .. } => {
            if columns.len() < 2 || rows.is_empty() {
                out.push("a table needs 2+ columns and a row".into());
            }
            if rows.iter().any(|r| r.len() != columns.len()) {
                out.push("every row needs one cell per column".into());
            }
            if hide.iter().any(|[r, c]| *r >= rows.len() || *c >= columns.len()) {
                out.push("a hidden cell is outside the table".into());
            }
        }
        StepBody::Try { items: ids } => {
            if ids.is_empty() || ids.len() > 5 {
                out.push("try 1 to 5 items".into());
            }
            for id in ids {
                if !items.contains(id.as_str()) {
                    out.push(format!("item {id} is not in this lecture"));
                }
            }
        }
    }
    out
}

/// A figure must be a plain, self-contained drawing in theme colours: no
/// scripts, event handlers, links, embedded images or fixed colours (so it
/// reads in Day, Night and true black alike).
pub fn svg_problems(svg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let s = svg.trim();
    if !s.starts_with("<svg") || !s.ends_with("</svg>") {
        out.push("svg must be one <svg>…</svg> element".into());
    }
    if !s.contains("viewBox") {
        out.push("svg needs a viewBox (it scales to the screen)".into());
    }
    if s.len() > MAX_SVG_BYTES {
        out.push(format!("svg is {} KB; keep figures under {} KB", s.len() / 1024, MAX_SVG_BYTES / 1024));
    }
    let lower = s.to_ascii_lowercase();
    for banned in
        ["<script", "<image", "<foreignobject", "<iframe", "javascript:", "href=\"http", "@import", "<style"]
    {
        if lower.contains(banned) {
            out.push(format!("svg may not contain `{banned}`"));
        }
    }
    // on…= event handler attributes.
    for (i, w) in lower.match_indices(" on") {
        let rest = &lower[i + w.len()..];
        let name: String = rest.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        if !name.is_empty() && rest[name.len()..].trim_start().starts_with('=') {
            out.push(format!("svg may not have event handlers (`on{name}`)"));
            break;
        }
    }
    for attr in ["fill=\"", "stroke=\"", "stop-color=\"", "color=\""] {
        for (i, _) in s.match_indices(attr) {
            // `stroke="` also matches inside `-stroke="`… only whole attributes.
            if i > 0 && !s.as_bytes()[i - 1].is_ascii_whitespace() {
                continue;
            }
            let value: String = s[i + attr.len()..].chars().take_while(|c| *c != '"').collect();
            let ok = value.starts_with("var(--")
                || value.starts_with("url(#")
                || matches!(value.as_str(), "none" | "currentColor" | "transparent");
            if !ok {
                out.push(format!("svg colour `{attr}{value}\"`: use var(--token), currentColor or none"));
            }
        }
    }
    for fixed in ["rgb(", "rgba(", "hsl(", "#"] {
        if fixed == "#" {
            // `#` is fine in url(#id) and href="#id"; a hex colour isn't.
            let hex = s.match_indices('#').any(|(i, _)| {
                let tail: String = s[i + 1..].chars().take_while(|c| c.is_ascii_hexdigit()).collect();
                let before = &s[..i];
                matches!(tail.len(), 3 | 6 | 8) && !before.ends_with("url(") && !before.ends_with("href=\"")
            });
            if hex {
                out.push("svg has a hex colour: use var(--token) or a class".into());
            }
        } else if lower.contains(fixed) {
            out.push(format!("svg has a fixed colour `{fixed}…)`: use var(--token)"));
        }
    }
    out
}
