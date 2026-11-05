//! The lesson text markup: paragraphs, lists and inline spans (bold,
//! terms, scripts) that the app renders.

/// A run of lesson text with one look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    /// One of the step's few key terms (`==term==`): drawn with a marker.
    pub key: bool,
    /// Latin script (English terms, numbers with units): isolated left-to-right.
    pub latin: bool,
}

/// A block of lesson text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Para(Vec<Span>),
    Bullets(Vec<Vec<Span>>),
    /// Lines written `1. `, `2. `…: drawn as a numbered list, counted from 1.
    Numbered(Vec<Vec<Span>>),
}

/// A list line: whether it is numbered, and its text. `- item`, or a number
/// then `. ` or `) ` (`1. item`, `2) item`).
fn list_item(line: &str) -> Option<(bool, &str)> {
    if let Some(rest) = line.strip_prefix("- ") {
        return Some((false, rest));
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || digits > 2 {
        return None;
    }
    let rest = &line[digits..];
    rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")).map(|r| (true, r))
}

/// Splits lesson text into paragraphs (blank lines) and lists (lines
/// starting `- `, or `1. ` for a numbered one), with `**bold**`,
/// `==key terms==` and Latin runs marked.
pub fn blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let flush = |out: &mut Vec<Block>, list: &mut Option<(bool, Vec<Vec<Span>>)>| {
        if let Some((numbered, items)) = list.take() {
            out.push(if numbered { Block::Numbered(items) } else { Block::Bullets(items) });
        }
    };
    for chunk in text.split("\n\n").map(str::trim).filter(|c| !c.is_empty()) {
        let mut para: Vec<Span> = Vec::new();
        let mut list: Option<(bool, Vec<Vec<Span>>)> = None;
        for l in chunk.lines().map(str::trim).filter(|l| !l.is_empty()) {
            match list_item(l) {
                Some((numbered, item)) => {
                    if !para.is_empty() {
                        out.push(Block::Para(std::mem::take(&mut para)));
                    }
                    if list.as_ref().is_some_and(|(n, _)| *n != numbered) {
                        flush(&mut out, &mut list);
                    }
                    list.get_or_insert_with(|| (numbered, Vec::new())).1.push(spans(item));
                }
                None => {
                    flush(&mut out, &mut list);
                    if !para.is_empty() {
                        para.push(Span { text: " ".into(), bold: false, key: false, latin: false });
                    }
                    para.extend(spans(l));
                }
            }
        }
        if !para.is_empty() {
            out.push(Block::Para(para));
        }
        flush(&mut out, &mut list);
    }
    out
}

/// One line of lesson text as spans: `**bold**`, `==key term==`, and runs
/// of Latin script apart from Arabic so the UI can isolate them
/// (`<bdi dir="ltr">`). Bold on punctuation alone (the colon after
/// "**(SECs)**:") is dropped, so it doesn't stand apart from its term.
pub fn spans(line: &str) -> Vec<Span> {
    // Script runs come from the whole line, so bold can't split an English
    // phrase ("**Genome:** single-stranded") into runs that flip.
    let mut plain = String::new();
    let mut look: Vec<(bool, bool)> = Vec::new();
    let (mut bold, mut key) = (false, false);
    let mut rest = line;
    while let Some(c) = rest.chars().next() {
        if let Some(r) = rest.strip_prefix("**") {
            bold = !bold;
            rest = r;
        } else if let Some(r) = rest.strip_prefix("==") {
            key = !key;
            rest = r;
        } else {
            plain.push(c);
            look.push((bold, key));
            rest = &rest[c.len_utf8()..];
        }
    }
    let mut out: Vec<Span> = Vec::new();
    let mut at = 0;
    for (text, latin) in script_runs(&plain) {
        for c in text.chars() {
            let (bold, key) = look[at];
            at += 1;
            match out.last_mut() {
                Some(last) if last.bold == bold && last.key == key && last.latin == latin => {
                    last.text.push(c)
                }
                _ => out.push(Span { text: c.to_string(), bold, key, latin }),
            }
        }
    }
    for s in &mut out {
        if s.text.chars().all(|c| !c.is_alphanumeric()) {
            s.bold = false;
            s.key = false;
        }
    }
    // Neighbours that now look the same join up again.
    let mut joined: Vec<Span> = Vec::with_capacity(out.len());
    for s in out {
        match joined.last_mut() {
            Some(l) if l.bold == s.bold && l.key == s.key && l.latin == s.latin => l.text.push_str(&s.text),
            _ => joined.push(s),
        }
    }
    joined
}

/// Runs of Latin and non-Latin text. Spaces, digits and punctuation between
/// two Latin words stay in the Latin run ("Stanford type A"), so a term
/// is isolated whole; elsewhere they go with the Arabic around them.
fn script_runs(text: &str) -> Vec<(String, bool)> {
    #[derive(Clone, Copy, PartialEq)]
    enum C {
        Latin,
        Arabic,
        Neutral,
    }
    let class = |c: char| {
        if c.is_ascii_alphabetic() || ('\u{00C0}'..='\u{024F}').contains(&c) || "µβαγκ".contains(c) {
            C::Latin
        } else if ('\u{0600}'..='\u{06FF}').contains(&c) || ('\u{0750}'..='\u{077F}').contains(&c) {
            C::Arabic
        } else {
            C::Neutral
        }
    };
    let chars: Vec<char> = text.chars().collect();
    let strong: Vec<C> = chars.iter().map(|&c| class(c)).collect();
    let mut resolved = strong.clone();
    let mixed = strong.contains(&C::Arabic);
    // Each run of neutral chars (spaces, digits, signs) between two strong
    // chars: between two Latin words it is Latin ("Stanford type A").
    // Right after a Latin word, the numbers, units and closing brackets that
    // continue it are Latin ("Zone 3", "PaO2 <80", "(LVEF)"), but not the
    // sentence's own punctuation. Right before one, an opening bracket or
    // quote is Latin ("(morphology").
    let mut i = 0;
    while i < chars.len() {
        if strong[i] != C::Neutral {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && strong[i] == C::Neutral {
            i += 1;
        }
        let before = start.checked_sub(1).map(|k| strong[k]);
        let after = strong.get(i).copied();
        let run = start..i;
        for k in run.clone() {
            resolved[k] = C::Arabic;
        }
        // In an Arabic line, "label: value" and "cause → effect" are two
        // terms the line reads in Arabic order ("الـ edge: sharp" reads edge,
        // then sharp); one isolate would put "sharp" next to "الـ" and
        // "edge" far off.
        let separator = || {
            let t: String = chars[run.clone()].iter().collect();
            mixed
                && t.ends_with(' ')
                && (matches!(t.trim(), ":" | ";")
                    || t.starts_with(' ') && matches!(t.trim(), "→" | "⇒" | "←"))
        };
        if before == Some(C::Latin) && after == Some(C::Latin) && !separator() {
            for k in run {
                resolved[k] = C::Latin;
            }
            continue;
        }
        if before == Some(C::Latin) {
            let continues = |c: char| c.is_ascii_digit() || c == ' ' || "%()[]+-−–≥≤<>/.×±°'\"·,".contains(c);
            let mut end = start;
            while end < i && continues(chars[end]) {
                end += 1;
            }
            // Brackets stay only in pairs: an opening one must close inside
            // the run ("Low risk (≤ 2%)"), a closing one must close a bracket
            // the Latin term opened ("(morphology)"). The others are Arabic's
            // ("(مع dextrose 20%)").
            let mut term = start;
            while term > 0 && resolved[term - 1] == C::Latin {
                term -= 1;
            }
            let mut opened = chars[term..start].iter().fold(0usize, |d, &c| match c {
                '(' | '[' => d + 1,
                ')' | ']' => d.saturating_sub(1),
                _ => d,
            });
            let mut open: Vec<usize> = Vec::new();
            for (k, &c) in chars.iter().enumerate().take(end).skip(start) {
                match c {
                    '(' | '[' => open.push(k),
                    ')' | ']' if open.pop().is_none() => {
                        if opened == 0 {
                            end = k;
                            break;
                        }
                        opened -= 1;
                    }
                    _ => {}
                }
            }
            if let Some(&first) = open.first() {
                end = first;
            }
            // Don't take the sentence's punctuation or trailing spaces.
            while end > start && " .,-–·".contains(chars[end - 1]) {
                end -= 1;
            }
            for r in &mut resolved[start..end] {
                *r = C::Latin;
            }
        }
        if after == Some(C::Latin) {
            // A number that starts the term is part of it too ("20 cm",
            // "1.5 ml/kg/h", "3–4 L").
            let mut from = i;
            while from > start
                && (chars[from - 1].is_ascii_digit() || " .,–-×/~<>≥≤±%([\"'".contains(chars[from - 1]))
            {
                from -= 1;
            }
            while from < i && !(chars[from].is_ascii_digit() || "<>≥≤±~([\"'".contains(chars[from])) {
                from += 1;
            }
            for r in &mut resolved[from..i] {
                *r = C::Latin;
            }
        }
    }
    // Numbers with a sign, a range or a ratio ("≤ 2%", "> 3", "5–6",
    // "1/3") read wrong in an Arabic line: bidi mirrors "≤" into "≥" and
    // turns "5–6" into "6–5". They are isolated left to right too.
    let mut k = 0;
    while k < chars.len() {
        if !chars[k].is_ascii_digit() || resolved[k] == C::Latin {
            k += 1;
            continue;
        }
        // A sign right before the number, maybe with one space ("≤ 2").
        let mut start = k;
        let mut signed = false;
        let mut j = k;
        if j > 0 && chars[j - 1] == ' ' {
            j -= 1;
        }
        while j > 0 && "<>≤≥±~=+−".contains(chars[j - 1]) && resolved[j - 1] != C::Latin {
            j -= 1;
            signed = true;
        }
        if signed {
            start = j;
        }
        let mut end = k;
        let mut joined = false;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
            if end + 1 < chars.len() && "–-/:×".contains(chars[end]) && chars[end + 1].is_ascii_digit() {
                joined = true;
                end += 1;
            } else if end + 1 < chars.len() && chars[end] == '.' && chars[end + 1].is_ascii_digit() {
                end += 1;
            } else if let Some(skip) = spaced_operator(&chars[end..]) {
                // A sum or product written with spaces: "45 + 55 = 100".
                joined = true;
                end += skip;
            }
        }
        let percent = chars.get(end) == Some(&'%');
        if percent {
            end += 1;
        }
        if signed || joined || percent {
            for r in &mut resolved[start..end] {
                *r = C::Latin;
            }
        }
        k = end;
    }
    // A bracket pair goes into an isolate only when all of it does: in
    // "(black أو brown)" the brackets stay Arabic, so they mirror right.
    let mut open: Vec<usize> = Vec::new();
    for k in 0..chars.len() {
        match chars[k] {
            '(' | '[' => open.push(k),
            ')' | ']' => {
                if let Some(o) = open.pop()
                    && resolved[o..=k].iter().any(|r| *r != C::Latin)
                {
                    resolved[o] = C::Arabic;
                    resolved[k] = C::Arabic;
                }
            }
            _ => {}
        }
    }
    let mut out: Vec<(String, bool)> = Vec::new();
    for (i, &c) in chars.iter().enumerate() {
        let latin = resolved[i] == C::Latin;
        match out.last_mut() {
            Some((s, l)) if *l == latin => s.push(c),
            _ => out.push((c.to_string(), latin)),
        }
    }
    out
}

/// The length of " + " (or "+", " = ", " × ", "−", "/") in front of a
/// digit at the start of `rest`, so "45 + 55 = 100" reads as one formula.
fn spaced_operator(rest: &[char]) -> Option<usize> {
    let mut k = 0;
    if rest.get(k) == Some(&' ') {
        k += 1;
    }
    if !rest.get(k).is_some_and(|c| "+=×−÷".contains(*c)) {
        return None;
    }
    k += 1;
    if rest.get(k) == Some(&' ') {
        k += 1;
    }
    rest.get(k).is_some_and(|c| c.is_ascii_digit()).then_some(k)
}
