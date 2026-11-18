use super::*;

/// A key made here, never a real one.
fn key(fill: u8) -> Key {
    Key::from_bytes([fill; 32])
}

/// The bytes 0, 1, … 31: the key the reference values below were worked
/// out for, apart from this code (Python's hashlib BLAKE2b, HChaCha20 from
/// its draft's test vector, and the `cryptography` package's
/// ChaCha20-Poly1305).
fn counting() -> Key {
    Key::from_bytes(std::array::from_fn(|i| i as u8))
}

fn question(id: &str) -> Item {
    let yaml = format!(
        "id: {id}\nkind: mcq\ntopic: oncogenes\nstem: \"A 49-year-old man has a mass. Biopsy is shown. Which is most likely?\"\noptions: [Transcription factor, Tyrosine kinase, GTPase]\nanswer: 0\nwhy: [\"Right. c-MYC.\", BCR-ABL., KRAS.]\nsrc: {{ book: vq, page: 1 }}\nstatus: unverified\ntags: [vq]\n"
    );
    serde_yaml_ng::from_str(&yaml).unwrap()
}

#[test]
fn a_question_round_trips_with_its_figure() {
    let k = key(7);
    let figure = b"RIFF\x04\x00\x00\x00WEBPVP8L".to_vec();
    let opened = Opened::with_image(question("path.L03.vq0001"), Some(&figure));
    let sealed = seal_item(&k, &opened);
    assert_eq!((sealed.id.as_str(), sealed.kind.as_str()), ("path.L03.vq0001", "mcq"));
    // Nothing of the question is in clear.
    assert!(!sealed.sealed.contains("Transcription") && !sealed.sealed.contains('='), "{}", sealed.sealed);
    let back = open_item(&k, &sealed).unwrap();
    assert_eq!(back, opened);
    assert_eq!(back.image_bytes(), Some(figure));
    assert_eq!(back.image_type.as_deref(), Some(IMAGE_TYPE));
    // A question without a figure has neither field.
    let plain = Opened::with_image(question("path.L03.vq0002"), None);
    assert_eq!(open_item(&k, &seal_item(&k, &plain)).unwrap(), plain);
    assert_eq!(plain.image_bytes(), None);
    // Its educational objective travels with it, and only when it has one.
    let taught =
        plain.clone().with_objective(Some("Reflex syncope is the most common cause of fainting.".into()));
    assert_eq!(open_item(&k, &seal_item(&k, &taught)).unwrap(), taught);
    let json = |o: &Opened| serde_json::to_string(o).unwrap();
    assert!(json(&taught).contains(r#""objective":"Reflex syncope"#) && !json(&plain).contains("objective"));
    assert_ne!(seal_item(&k, &taught).sealed, seal_item(&k, &plain).sealed);
}

#[test]
fn sealing_is_deterministic() {
    let k = key(7);
    let opened = Opened::with_image(question("path.L03.vq0001"), None);
    assert_eq!(seal_item(&k, &opened), seal_item(&k, &opened));
    // Any change in the question, or another key, gives other bytes.
    let mut edited = opened.clone();
    edited.item.topic = Some("tumor-suppressors".into());
    assert_ne!(seal_item(&k, &edited).sealed, seal_item(&k, &opened).sealed);
    assert_ne!(seal_item(&key(8), &opened).sealed, seal_item(&k, &opened).sealed);
}

#[test]
fn the_format_matches_the_reference_values() {
    let k = counting();
    assert_eq!(k.text(), "AAAQ-EAYE-AUDA-OCAJ-BIFQ-YDIO-B4IB-CEQT-CQKR-MFYY-DENB-WHA5-DYPQ");
    assert_eq!(k.id(), "f0745f75");
    let blob = seal(&k, "path.L03.vq0001", br#"{"x":1}"#);
    assert_eq!(blob, "AVvBzx-iy6k-6bvkpsH-L7HknMNOR7GZv_2Q-I2cmzjAddkt4d6PpPZlBJDK7Lap");
    assert_eq!(open(&k, "path.L03.vq0001", &blob).unwrap(), br#"{"x":1}"#);
}

#[test]
fn a_wrong_key_opens_nothing() {
    let sealed = seal_item(&key(7), &Opened::with_image(question("path.L03.vq0001"), None));
    assert_eq!(open_item(&key(8), &sealed), Err(SealError::Open));
    assert_ne!(key(7).id(), key(8).id());
}

#[test]
fn a_changed_byte_opens_nothing() {
    let k = key(7);
    let blob = seal(&k, "path.L03.vq0001", b"a question");
    let mut bytes = URL_SAFE_NO_PAD.decode(&blob).unwrap();
    for at in [1, 1 + NONCE_LEN, bytes.len() - 1] {
        bytes[at] ^= 1;
        assert_eq!(
            open(&k, "path.L03.vq0001", &URL_SAFE_NO_PAD.encode(&bytes)),
            Err(SealError::Open),
            "{at}"
        );
        bytes[at] ^= 1;
    }
    // Another layout, a cut blob, and text that isn't base64url.
    bytes[0] = 2;
    assert_eq!(open(&k, "path.L03.vq0001", &URL_SAFE_NO_PAD.encode(&bytes)), Err(SealError::Blob));
    assert_eq!(open(&k, "path.L03.vq0001", &blob[..20]), Err(SealError::Blob));
    assert_eq!(open(&k, "path.L03.vq0001", "not base64!"), Err(SealError::Blob));
}

#[test]
fn a_blob_under_another_id_opens_nothing() {
    let k = key(7);
    let one = seal_item(&k, &Opened::with_image(question("path.L03.vq0001"), None));
    let two = seal_item(&k, &Opened::with_image(question("path.L03.vq0002"), None));
    // The blobs swapped between the two ids.
    let swapped = SealedItem { sealed: two.sealed.clone(), ..one.clone() };
    assert_eq!(open_item(&k, &swapped), Err(SealError::Open));
    // A question sealed under the clear id, though it names another inside.
    let mut other = question("path.L03.vq0002");
    let json = serde_json::to_vec(&Opened::with_image(other.clone(), None)).unwrap();
    let forged = SealedItem { sealed: seal(&k, "path.L03.vq0001", &json), ..one.clone() };
    assert!(matches!(open_item(&k, &forged), Err(SealError::Id { found, .. }) if found == "path.L03.vq0002"));
    // And the kind in clear must be the question's.
    other.id = "path.L03.vq0001".into();
    let json = serde_json::to_vec(&Opened::with_image(other, None)).unwrap();
    let card = SealedItem { kind: "card".into(), sealed: seal(&k, "path.L03.vq0001", &json), ..one };
    assert!(matches!(open_item(&k, &card), Err(SealError::Kind { .. })));
    // Plaintext that isn't a question.
    let junk = SealedItem { sealed: seal(&k, "path.L03.vq0001", b"[1, 2]"), ..two };
    let junk = SealedItem { id: "path.L03.vq0001".into(), ..junk };
    assert!(matches!(open_item(&k, &junk), Err(SealError::Json(_))));
}

#[test]
fn the_key_text_reads_back_whatever_its_spacing_and_case() {
    for fill in [0, 7, 255] {
        let k = key(fill);
        let text = k.text();
        assert_eq!(text.len(), 52 + 12, "{text}");
        assert!(text.split('-').all(|g| g.len() == 4), "{text}");
        assert_eq!(Key::parse(&text).unwrap(), k);
        assert_eq!(Key::parse(&text.to_lowercase().replace('-', " ")).unwrap(), k);
        assert_eq!(Key::parse(&format!("  {}\n", text.replace('-', ""))).unwrap(), k);
    }
    let text = counting().text();
    // A letter short, one too many, a letter base32 doesn't have (1, 8, 0),
    // and spare bits that aren't zero.
    assert_eq!(Key::parse(&text[..text.len() - 1]), Err(SealError::KeyText));
    assert_eq!(Key::parse(&format!("{text}A")), Err(SealError::KeyText));
    assert_eq!(Key::parse(&text.replacen('A', "1", 1)), Err(SealError::KeyText));
    assert_eq!(Key::parse(&format!("{}R", &text[..text.len() - 1])), Err(SealError::KeyText));
    assert_eq!(Key::parse(""), Err(SealError::KeyText));
}

#[test]
fn the_key_id_names_the_key_without_showing_it() {
    let k = key(7);
    let id = k.id();
    assert_eq!(id.len(), 8);
    assert!(id.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()), "{id}");
    assert_eq!(id, key(7).id(), "the same key, the same id");
    // Debug output shows the id, never the key.
    let debug = format!("{k:?}");
    assert!(debug.contains(&id) && !debug.contains(&k.text()[..4]), "{debug}");
}

#[test]
fn a_file_is_sorted_and_names_its_key() {
    let k = key(7);
    let items = ["path.L03.vq0010", "path.L03.vq0002"]
        .map(|id| seal_item(&k, &Opened::with_image(question(id), None)))
        .to_vec();
    let file = SealedLecture::new("path.L03", &k, items);
    assert_eq!(file.schema, SCHEMA);
    assert_eq!(file.key, k.id());
    assert_eq!(
        file.items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
        ["path.L03.vq0002", "path.L03.vq0010"]
    );
    let yaml = serde_yaml_ng::to_string(&file).unwrap();
    assert_eq!(serde_yaml_ng::from_str::<SealedLecture>(&yaml).unwrap(), file);
    // The file holds nothing but the four fields.
    assert!(serde_yaml_ng::from_str::<SealedLecture>(&format!("{yaml}title: x\n")).is_err());
}

#[test]
fn vq_ids_numbers_and_labels() {
    assert_eq!(number("path.L03.vq0001"), Some(1));
    assert_eq!(number("heme.L04.vq1314"), Some(1314));
    assert_eq!(under("path.L03.vq0001", "path.L03"), Some(1));
    assert_eq!(under("path.L03.vq0001", "path.L04"), None);
    for bad in
        ["path.L03.001", "path.L03.vq001", "path.L03.vq00001", "path.L03.vq0000", "path.L03.vq+001", "vq0001"]
    {
        assert_eq!(number(bad), None, "{bad}");
    }
    assert_eq!(id_for("path.L03", 7), "path.L03.vq0007");
    assert_eq!(label("VQ", 1), "VQ-0001");
    assert_eq!(label("VQ", 1314), "VQ-1314");
    assert!(is_webp(b"RIFF\x10\x00\x00\x00WEBPVP8 "));
    assert!(!is_webp(b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR"));
    assert!(!is_webp(b"RIFF"));
}
