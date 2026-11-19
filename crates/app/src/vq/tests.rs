use std::collections::BTreeMap;

use steps_core::sealed::{Opened, SealedItem, SealedLecture, seal_item};

use super::testing::{FIGURE, ID, LECTURE, key, other_key, question, video};
use super::*;

/// The fixture lecture alone, and its sealed file.
fn parts() -> (BTreeMap<String, Lecture>, SealedLecture) {
    let lib = Library::get();
    let lectures = BTreeMap::from([(LECTURE.to_string(), lib.lectures[LECTURE].clone())]);
    let mut second = question();
    second.id = "cvs.L01.vq0002".into();
    second.src.page = 2;
    let items = vec![
        seal_item(
            &key(),
            &Opened::with_image(question(), Some(FIGURE))
                .with_objective(Some("Taught.".into()))
                .with_video(Some(video())),
        ),
        seal_item(&key(), &Opened::with_image(second, None)),
    ];
    (lectures, SealedLecture::new(LECTURE, &key(), items))
}

#[test]
fn the_right_key_opens_the_questions_into_their_lecture() {
    let (mut lectures, file) = parts();
    let before = lectures[LECTURE].items.len();
    let bank = open_into(std::slice::from_ref(&file), Some(&key()), &mut lectures);
    assert_eq!((bank.sealed, bank.opened()), (2, 2));
    assert_eq!(bank.key_id.as_deref(), Some(key().id().as_str()));
    let items = &lectures[LECTURE].items;
    assert_eq!(items.len(), before + 2);
    assert_eq!(items[before], question(), "an ordinary item, appended after the lecture's own");
    assert_eq!(bank.question(1), Some(ID));
    assert!(bank.has(ID) && !bank.has("cvs.L01.001"));
    let figure = bank.image(ID).unwrap();
    assert!(figure.starts_with("data:image/webp;base64,UklGR"), "{figure}");
    assert_eq!(bank.image("cvs.L01.vq0002"), None);
    assert_eq!(bank.objective(ID), Some("Taught."));
    assert_eq!(bank.objective("cvs.L01.vq0002"), None);
    assert_eq!(bank.video(ID), Some(&video()));
    assert_eq!(
        bank.video(ID).map(|v| v.url()).as_deref(),
        Some("https://www.youtube.com/watch?v=Fixture_0-1")
    );
    assert_eq!(bank.video("cvs.L01.vq0002"), None);
}

#[test]
fn a_video_without_a_youtube_id_s_shape_is_left_out() {
    let (mut lectures, _) = parts();
    let odd = steps_core::sealed::Video { youtube: "javascript:alert(1)".into(), title: "T".into() };
    let file = SealedLecture::new(
        LECTURE,
        &key(),
        vec![seal_item(&key(), &Opened::with_image(question(), None).with_video(Some(odd)))],
    );
    let bank = open_into(std::slice::from_ref(&file), Some(&key()), &mut lectures);
    assert!(bank.has(ID) && bank.video(ID).is_none(), "the question opens; its link doesn't");
}

#[test]
fn another_key_or_none_opens_nothing_and_says_nothing() {
    for k in [Some(other_key()), None] {
        let (mut lectures, file) = parts();
        let before = lectures[LECTURE].items.clone();
        let bank = open_into(std::slice::from_ref(&file), k.as_ref(), &mut lectures);
        assert_eq!((bank.sealed, bank.opened()), (2, 0));
        assert_eq!(lectures[LECTURE].items, before);
        assert!(bank.fits(&key()) && !bank.fits(&other_key()));
    }
    // A pack with no questions takes any key, and opens nothing.
    let bank = open_into(&[], Some(&key()), &mut BTreeMap::new());
    assert!(bank.fits(&other_key()) && bank.key_id.is_none() && bank.opened() == 0);
}

#[test]
fn a_question_that_doesnt_open_is_left_out() {
    let (mut lectures, mut file) = parts();
    file.items[1] = SealedItem { sealed: file.items[0].sealed.clone(), ..file.items[1].clone() };
    // A file for a lecture this library doesn't hold.
    let stray = SealedLecture { lecture: "zz.L01".into(), ..file.clone() };
    let bank = open_into(&[file, stray], Some(&key()), &mut lectures);
    assert_eq!((bank.sealed, bank.opened()), (4, 1));
    assert!(bank.has(ID) && !bank.has("cvs.L01.vq0002"));
}

#[test]
fn a_question_bank_item_is_named_by_its_number() {
    assert_eq!(source_label(&question()).as_deref(), Some("VQ-0001"));
    assert_eq!(name(ID).as_deref(), Some("VQ-0001"));
    assert_eq!(name("cvs.L01.001"), None);
    let lib = Library::get();
    let first_aid = lib.lectures[LECTURE].items.first().unwrap();
    assert_eq!(source_label(first_aid), None);
}

#[test]
fn search_reads_every_way_of_writing_a_number() {
    for (q, n) in [("VQ-0001", 1), ("vq0001", 1), ("vq 1", 1), ("  Vq-1314 ", 1314), ("vq #7", 7)] {
        assert_eq!(search_number(q), Some(n), "{q}");
    }
    for q in ["vq", "vq-0", "vq 12345", "vqx1", "1", "questions vq 1", "vq 1 2"] {
        assert_eq!(search_number(q), None, "{q}");
    }
}

#[test]
fn the_vault_keeps_the_key_for_this_user_only_and_forgets_it() {
    let dir = crate::prefs::test_dir();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(vault::load(), None);
    assert!(vault::save(&key()));
    assert_eq!(vault::load(), Some(key()));
    let file = dir.join("vq-key.txt");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), key().text());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600);
    }
    // Text that isn't a key reads as none.
    std::fs::write(&file, "not a key").unwrap();
    assert_eq!(vault::load(), None);
    vault::clear();
    assert!(!file.exists());
    assert_eq!(vault::load(), None);
}
