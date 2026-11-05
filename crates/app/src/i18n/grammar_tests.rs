use super::*;

#[test]
fn course_map_lines_agree() {
    assert_eq!(courses::pages_line(614, 16), "614 صفحة من First Aid في 16 مادة");
    assert_eq!(courses::pages_line(2, 2), "صفحتين من First Aid في مادتين");
    assert_eq!(courses::started(12, 40), "بدأت 12 من 40 كارت");
    assert_eq!(courses::started(0, 5), "بدأت 0 من 5 كروت");
    assert_eq!(courses::tile_line("قلب", 284, 328), "قلب · ص 284–328");
    assert_eq!(courses::book_pages(284, 328, 45), "First Aid ص 284–328 (45 صفحة)");
    assert_eq!(courses::all_sub(9), "9 مواد مع بعض");
    assert_eq!(courses::all_sub(12), "12 مادة مع بعض");
    assert_eq!(courses::lessons_done(3, 7), "3 من 7 دروس");
    assert_eq!(courses::lessons_done(3, 30), "3 من 30 درس");
    assert_eq!(courses::lectures_only(5), "5 محاضرات، الدروس جاية");
    assert_eq!(courses::ready_lectures(1, 2), "مكتوب 1 من محاضرتين");
    assert_eq!(courses::content_line(3, 4, 120, 6), "مكتوب 3 من 4 محاضرات · 120 كارت · 6 دروس");
}

#[test]
fn other_counted_lines_agree() {
    assert_eq!(today::exam_in(5), "فاضل 5 أيام على الامتحان");
    assert_eq!(today::exam_in(40), "فاضل 40 يوم على الامتحان");
    assert_eq!(today::plan_line(3, 4), "3 جديد + 4 مراجعات");
    assert_eq!(stats::day_bar(5, 2), "5 دقايق، إجابتين");
    assert_eq!(table::mark_each(2), "فاضل خانتين: علّم على كل واحدة عرفتها ولا لأ.");
    assert_eq!(table::recalled(1, 12), "افتكرت 1 من 12 خانة.");
    assert_eq!(order::moves_to_fix(2), "الترتيب قريب: نقل خطوتين بس بيظبطه.");
    assert_eq!(learn::misses(2), "وصّلتهم كلهم، بغلطتين في السكة.");
    assert_eq!(learn::misses(4), "وصّلتهم كلهم، بـ 4 غلطات في السكة.");
    assert_eq!(update::notice_more(1), "وحاجة واحدة كمان");
    assert_eq!(update::notice_more(4), "و4 حاجات كمان");
    assert_eq!(cloud::pending(2), "تغييرين مستنيين يترفعوا");
    assert_eq!(cloud::pending(14), "14 تغيير مستني يترفع");
    assert_eq!(data::import_partial(3).split(" (").next().unwrap(), "بس فيه 3 صفوف ما اتدمجوش");
    assert_eq!(study::enum_goal(8, 5), "فيه 8 نقط، ولو افتكرت 5 منهم تعدّي.");
    assert_eq!(plan::days_left(40), "فاضل 40 يوم مذاكرة");
    assert_eq!(anki::contents(12, 2), "12 محاضرة · سؤالين");
}

#[test]
fn each_count_takes_the_right_form() {
    assert_eq!(today::due_cases(2), "حالتين مستنية مراجعة في شاشة الحالات");
    assert_eq!(today::exam_in(1), "الامتحان بكرة. إنت جاهز أكتر مما فاكر.");
    assert_eq!(today::exam_in(2), "فاضل يومين على الامتحان");
    assert!(today::tight_days(1).starts_with("الخطة محتاجة يوم مذاكرة زيادة في الأسبوع"));
    assert!(today::tight_days(3).starts_with("الخطة محتاجة 3 أيام مذاكرة زيادة في الأسبوع"));
    assert_eq!(today::headline(0, 5), "5 حاجات جديدة، على قد وقتك.");
    assert_eq!(today::headline(1, 0), "مراجعة واحدة، وبس.");
    assert_eq!(today::headline(2, 12), "مراجعتين، و12 حاجة جديدة.");
    assert_ne!(today::hero_body(0, 3), today::hero_body(3, 0));
    assert_ne!(today::hero_body(3, 0), today::hero_body(3, 3));
    assert_eq!(today::plan_reviews_only(7), "7 مراجعات بس");
    assert_eq!(
        study_settings::personal(40),
        format!("{} 40 مراجعة. بيتحدّث كل أسبوع.", study_settings::PERSONAL_1)
    );
    assert_eq!(study::due_elsewhere(1), "فيه كارت واحد مستحق في مادة تانية.");
    assert_eq!(study::due_elsewhere(2), "فيه كارتين مستحقين في مواد تانية.");
    assert_eq!(study::due_elsewhere(4), "فيه 4 كروت مستحقة في مواد تانية.");
    assert_eq!(study::due_elsewhere(11), "فيه 11 كارت مستحق في مواد تانية.");
    assert_eq!(table::mark_each(1), "فاضل خانة واحدة: عرفتها ولا لأ؟");
    assert_eq!(table::recalled(3, 3), "كل الخانات صح (3 من 3). حلو أوي!");
    assert_eq!(order::moves_to_fix(1), "الترتيب قريب: نقل خطوة واحدة بس بيظبطه.");
    assert_eq!(case::steps(5), "5 خطوات");
    assert_eq!(own::cards(1), "كارت واحد");
    assert_eq!(cloud::pending(1), "تغيير واحد مستني يترفع");
    assert_eq!(cloud::pending(5), "5 تغييرات مستنية تترفع");
    assert_eq!(cloud::pending(103), "103 تغييرات مستنية تترفع");
    assert_eq!(update::builds_count(1), "نسخة جديدة");
    assert_eq!(update::builds_count(2), "نسختين جداد");
    assert_eq!(update::builds_count(4), "4 نسخ جديدة");
    assert_eq!(update::builds_count(12), "12 نسخة جديدة");
    assert_eq!(update::content_updates_count(1), "تحديث محتوى");
    assert_eq!(update::content_updates_count(2), "تحديثين محتوى");
    assert_eq!(update::content_updates_count(3), "3 تحديثات محتوى");
    assert_eq!(update::content_updates_count(20), "20 تحديث محتوى");
    assert_eq!(remind::text(None), remind::text(Some(0)));
    assert_eq!(remind::text(Some(1)), "عندك مراجعة واحدة النهارده. دقيقة وتخلص.");
    assert_eq!(remind::text(Some(14)), "عندك 14 مراجعة النهارده. يلا نبدأ بجلسة قصيرة.");
    assert_eq!(onboard::days_sub(7), "كل أيام الأسبوع");
    assert_eq!(onboard::days_sub(1), "يوم واحد في الأسبوع");
    assert_eq!(onboard::days_sub(2), "يومين في الأسبوع");
    assert_eq!(onboard::days_sub(5), "5 أيام في الأسبوع");
    assert_eq!(onboard::studied_sub(0), "ولا محاضرة لسه");
    assert_eq!(onboard::studied_sub(1), "محاضرة واحدة");
    assert_eq!(onboard::studied_sub(2), "محاضرتين");
    assert_eq!(onboard::studied_sub(9), "9 محاضرات");
    assert_eq!(onboard::studied_sub(11), "11 محاضرة");
    assert_eq!(onboard::studied_sub(103), "103 محاضرات");
    assert_eq!(recap::exam_in(0), "الامتحان النهارده. بالتوفيق.");
    assert_eq!(recap::exam_in(30), "فاضل على الامتحان 30 يوم.");
    assert_eq!(learn::misses(1), "وصّلتهم كلهم، بغلطة واحدة في السكة.");
    assert_eq!(learn::score_line(5, 5), "ولا غلطة. الدرس ده بقى بتاعك.");
    assert_eq!(learn::score_line(3, 5), "كويس جدًا. اللي غلطت فيه هيرجعلك في المراجعة.");
    assert_eq!(learn::score_line(1, 5), "عادي جدًا من أول مرة، ده درس جديد. الكروت هتثبّته.");
}

#[test]
fn numbers_and_names_land_where_the_sentence_wants_them() {
    assert_eq!(cite::slide(4), "سلايد 4");
    assert_eq!(bank::all_of("قلب"), "كل محاضرات قلب");
    assert_eq!(bank::checked(true, 'C'), "إجابتك صح.");
    assert_eq!(bank::checked(false, 'C'), "إجابتك غلط. الإجابة الصح C.");
    assert_eq!(stats::better(true, 12), "أحسن من الأسبوع اللي فات بـ 12%");
    assert_eq!(stats::better(false, 3), "أحسن من الشهر اللي فات بـ 3%");
    assert!(today::tight_cap(150).starts_with("لو علّيت حد المراجعات اليومي لـ 150، الخطة تلحق كلها."));
    assert_eq!(plan::tight_cap(150), "لو علّيت حد المراجعات اليومي لـ 150 هتلحق كلها.");
    assert!(order::in_place(3, 5).starts_with("3 من 5 في مكانها."));
    assert_eq!(order::should_be(2), "مكانها 2");
    assert_eq!(learn::placed_of(2, 6), "اخترت 2 من 6");
    assert_eq!(learn::slot(3), "الخطوة رقم 3");
    assert_eq!(learn::should_be(1), "مكانها رقم 1");
    assert_eq!(learn::sort_left(4, 9), "فاضل 4 من 9");
    assert_eq!(learn::in_place(5, 7), "5 من 7 في مكانهم.");
    assert!(learn::no_match("VSD").starts_with("مفيش درس فيه «VSD»."));
    assert_eq!(learn::resume_at(3, 8), "هتكمّل من خطوة 3 من 8");
}
