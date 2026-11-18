use super::*;
use crate::content::{Item, Lecture};
use crate::sealed::{Key, seal_item};

fn key() -> Key {
    Key::from_bytes([7; 32])
}

fn lecture() -> Lecture {
    serde_yaml_ng::from_str(include_str!("../../../../fixtures/content/sample.yaml")).unwrap()
}

/// A VQ question filed under the fixture lecture (`cvs.L04`, topic `shock`).
fn question(id: &str, extra: &str) -> Item {
    let n = crate::sealed::number(id).unwrap_or(1);
    serde_yaml_ng::from_str(&format!(
        "id: {id}\nkind: mcq\ntopic: shock\nstem: A 70-year-old woman faints. Which is most likely?\noptions: [Tamponade, Sepsis, Anaphylaxis, Hemorrhage, Heart block, Vagal syncope]\nanswer: 0\nwhy: [Right., a, b, c, d, e]\nsrc: {{ book: vq, page: {n} }}\nstatus: unverified\ntags: [vq]\n{extra}"
    ))
    .unwrap()
}

fn qbank() -> std::collections::BTreeSet<String> {
    ["vq".to_string()].into()
}

fn messages(o: &Opened) -> Vec<String> {
    let q = qbank();
    let cx = Context { qbank: Some(&q), ..Context::default() };
    opened(&lecture(), o, &cx).iter().map(|p| p.to_string()).collect()
}

#[test]
fn a_sound_file_and_question_pass() {
    let o = Opened::with_image(question("cvs.L04.vq0001", ""), None);
    let file = SealedLecture::new("cvs.L04", &key(), vec![seal_item(&key(), &o)]);
    assert!(super::file(&file).is_empty(), "{:?}", super::file(&file));
    assert!(messages(&o).is_empty(), "{:#?}", messages(&o));
    // Its figure, a WebP file.
    let o = Opened::with_image(question("cvs.L04.vq0001", ""), Some(b"RIFF\x04\x00\x00\x00WEBPVP8L"));
    assert!(messages(&o).is_empty(), "{:#?}", messages(&o));
}

#[test]
fn file_problems_are_named_without_the_key() {
    let o = Opened::with_image(question("cvs.L04.vq0002", ""), None);
    let mut file = SealedLecture::new("cvs.L04", &key(), vec![seal_item(&key(), &o)]);
    file.schema = "steps.sealed/v0".into();
    file.key = "Key-1".into();
    let mut second = file.items[0].clone();
    second.id = "cvs.L04.vq0001".into();
    second.kind = "card".into();
    file.items.push(second);
    let mut stray = file.items[0].clone();
    stray.id = "cvs.L05.vq0003".into();
    stray.sealed = " ".into();
    file.items.push(stray);
    file.items.push(file.items[0].clone());
    let msgs: Vec<String> = super::file(&file).iter().map(|p| p.to_string()).collect();
    for want in [
        "cvs.L04: schema is \"steps.sealed/v0\", expected \"steps.sealed/v1\"",
        "cvs.L04: key \"Key-1\" is not a key id",
        "cvs.L04.vq0001: ids are out of order",
        "cvs.L04.vq0001: kind is \"card\"; VQ questions are mcq",
        "cvs.L05.vq0003: id must be cvs.L04.vqNNNN",
        "cvs.L05.vq0003: sealed is empty",
        "cvs.L04.vq0002: id is used twice",
    ] {
        assert!(msgs.iter().any(|m| m.starts_with(want)), "{want}: {msgs:#?}");
    }
    let empty = SealedLecture::new("cvs.L04", &key(), Vec::new());
    assert!(super::file(&empty)[0].message.contains("no questions"));
}

#[test]
fn the_banks_own_rules_on_an_opened_question() {
    // Another source, another page, no tag.
    let mut item = question("cvs.L04.vq0003", "");
    item.src.book = None;
    item.src.page = 9;
    item.tags.clear();
    item.topic = Some("murmurs".into());
    let msgs = messages(&Opened::with_image(item, None));
    for want in [
        "cvs.L04.vq0003: topic `murmurs` is not in the lecture's topics",
        "cvs.L04.vq0003: src.book must be vq",
        "cvs.L04.vq0003: src.page must be 3, the id's VQ number",
        "cvs.L04.vq0003: tags need `vq`",
        // Not from the bank any more: First Aid's rules, page range included.
        "cvs.L04.vq0003: mcq needs 4 or 5 options, has 6",
        "cvs.L04.vq0003: src.page 9 is outside the lecture",
    ] {
        assert!(msgs.iter().any(|m| m.starts_with(want)), "{want}: {msgs:#?}");
    }
    // Approvals hold as for every item.
    let msgs = messages(&Opened::with_image(question("cvs.L04.vq0001", "").approved(), None));
    assert!(msgs.iter().any(|m| m.contains("only Mohamed approves")), "{msgs:#?}");
}

#[test]
fn a_figure_is_optional_and_a_small_webp() {
    // A stem that says something "is shown" needs no figure: lab values
    // are often shown as text.
    let shown = question("cvs.L04.vq0004", "").with_stem("Lab values are shown. Which is most likely?");
    assert!(messages(&Opened::with_image(shown.clone(), None)).is_empty());
    // A PNG, a figure over 200 KB, a type without a figure and the reverse.
    let png = Opened::with_image(shown.clone(), Some(b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR"));
    assert!(messages(&png).iter().any(|m| m.ends_with("image is not a WebP file")));
    let mut big = b"RIFF\x00\x00\x00\x00WEBP".to_vec();
    big.resize(sealed::MAX_IMAGE_BYTES + 1, 0);
    let big = Opened::with_image(shown.clone(), Some(&big));
    assert!(
        messages(&big).iter().any(|m| m.contains("image is 201 KB, over 200 KB")),
        "{:?}",
        messages(&big)
    );
    let mut gif = Opened::with_image(shown.clone(), Some(b"RIFF\x04\x00\x00\x00WEBPVP8L"));
    gif.image_type = Some("image/gif".into());
    assert!(messages(&gif).iter().any(|m| m.contains("image_type is Some(\"image/gif\")")));
    let typed =
        Opened { image: None, image_type: Some("image/webp".into()), item: shown.clone(), objective: None };
    assert!(messages(&typed).iter().any(|m| m.ends_with("image_type without an image")));
    let garbled = Opened {
        image: Some("***".into()),
        image_type: Some("image/webp".into()),
        item: shown,
        objective: None,
    };
    assert!(messages(&garbled).iter().any(|m| m.ends_with("image is not base64")));
}

#[test]
fn an_educational_objective_says_something() {
    let o = Opened::with_image(question("cvs.L04.vq0005", ""), None);
    let taught = o.clone().with_objective(Some("Tamponade: Beck triad and pulsus paradoxus.".into()));
    assert!(messages(&taught).is_empty(), "{:#?}", messages(&taught));
    for blank in ["", "  \n"] {
        let msgs = messages(&o.clone().with_objective(Some(blank.into())));
        assert_eq!(msgs, ["cvs.L04.vq0005: objective is empty; write it or leave it out"], "{blank:?}");
    }
}

trait Edit {
    fn approved(self) -> Self;
    fn with_stem(self, stem: &str) -> Self;
}

impl Edit for Item {
    fn approved(mut self) -> Self {
        self.status = crate::content::Status::Approved;
        self
    }

    fn with_stem(mut self, new: &str) -> Self {
        if let Body::Mcq { stem, .. } = &mut self.body {
            *stem = new.into();
        }
        self
    }
}
