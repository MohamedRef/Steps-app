use super::*;
use crate::store::Store;

fn question(id: &str, category: &str, image: Option<Image>) -> OwnQuestion {
    OwnQuestion {
        id: id.into(),
        category: category.into(),
        lecture: "cvs.L03".into(),
        topic: None,
        page: Some(295),
        stem: "Stem?".into(),
        options: vec!["A".into(), "B".into()],
        answer: 1,
        why: vec![],
        explanation: None,
        objective: None,
        image,
        video: None,
        tags: vec![],
    }
}

#[test]
fn questions_and_their_figures_are_kept_synced_and_deleted_by_category() {
    let s = Store::open_in_memory().unwrap();
    let bytes: Vec<u8> = (0..(ownq::CHUNK_BYTES + 5)).map(|i| (i % 7) as u8).collect();
    let image =
        Image { sha256: "beef".into(), kind: "image/png".into(), bytes: bytes.len() as u32, chunks: 2 };
    let qs = [question("P-1", "Pathoma", Some(image.clone())), question("U-1", "UWorld notes", None)];
    s.put_own_questions(&qs, &[(image.clone(), bytes.clone())]).unwrap();
    assert_eq!(s.own_questions(), [qs[0].clone(), qs[1].clone()]);
    assert_eq!(s.own_image(&image), Some(bytes.clone()));
    // Every row waits for Steps Cloud: two chunks and two questions.
    let ops = s.outbox(100).unwrap();
    let tables: Vec<&str> = ops.iter().map(|o| o.table.as_str()).collect();
    assert_eq!(tables, ["own_images", "own_images", "own_questions", "own_questions"]);
    assert!(
        ops.iter()
            .all(|o| serde_json::to_string(&o.payload).unwrap().len()
                < steps_protocol::sync::MAX_PAYLOAD_BYTES)
    );
    // The same figure again is not written again.
    s.put_own_questions(&qs[..1], &[(image.clone(), bytes)]).unwrap();
    assert_eq!(s.outbox_len(), 4, "the question's row is replaced in the outbox, no chunk added");
    assert_eq!(s.delete_own_category("Pathoma").unwrap(), 1);
    assert_eq!(s.own_questions(), [qs[1].clone()]);
    let missing = Image { sha256: "none".into(), ..image };
    assert_eq!(s.own_image(&missing), None);
}
