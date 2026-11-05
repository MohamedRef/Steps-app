use super::count::{LECTURE, QUESTION, TEST, VIDEO, count};

pub const TITLE: &str = "فيديوهات B&B وأسئلة UWorld";
pub const HOW: &str = "انسخ الأرقام والزقها في UWorld › Create Test › Custom Test. علّم على الفيديو لما تخلصه.";
pub const DONE: &str = "خلصته";
pub const COPIED: &str = "اتنسخ ✓";
pub const NONE: &str = "أسئلته تحت فيديو تاني";
pub const FIND: &str = "فهرس أسئلة UWorld";
pub const PAGE_TITLE: &str = "أسئلة UWorld";
pub const PAGE_SUB: &str = "كل فيديوهات B&B وأسئلتها، مترتبة على First Aid. انسخ أي مادة أو محاضرة أو فيديو، أو علّم عليهم واعمل امتحان مخلوط.";
pub const PLACEHOLDER: &str = "دوّر على مادة أو محاضرة أو فيديو، أو اكتب رقم سؤال";
pub const FOUND: &str = "السؤال ده من";
pub const FROM_VIDEO: &str = "فيديو B&B:";
pub const OPEN_LECTURE: &str = "افتح المحاضرة";
pub const RETIRED: &str = "UWorld شالت السؤال ده.";
pub const NOT_FOUND: &str = "الرقم ده مش في القايمة. مش كل أسئلة UWorld فيها.";
pub const NO_MATCH: &str = "مفيش حاجة بالاسم ده.";
pub const SOURCE: &str =
    "الأرقام من ملف عام بيربط فيديوهات B&B بأسئلة UWorld (حوالي 86% من البنك)، وبتتحدث كل أسبوع.";
pub const CLOSE: &str = "اقفل";
pub const PICK: &str = "حطه في السلة";
pub const ALL_DONE: &str = "خلّصته كله ✓";
pub const BASKET: &str = "السلة";
pub const SHOW_TESTS: &str = "اعرض الامتحانات";
pub const SHOW_BASKET: &str = "الصق أرقام";
pub const BASKET_EMPTY: &str =
    "علّم ✓ على أي مادة أو محاضرة أو فيديو فوق، أو الصق أرقام، وأنا أقسّمهم امتحانات.";
pub const PASTE: &str = "الصق أرقام عندك (غلطاتك مثلاً)";
pub const PASTE_HINT: &str = "1707, 2033 أو 1707 2033";
pub const CLEAR: &str = "فضّي السلة";
pub const SIZE: &str = "أسئلة الامتحان";
pub const EVEN: &str = "قسّم بالتساوي";
pub const SHUFFLE: &str = "لخبط الترتيب";
pub const RESHUFFLE: &str = "لخبط تاني";
pub const SKIP_DONE: &str = "شيل فيديوهات خلّصتها";
pub const VIDEOS: &str = "الفيديوهات";
/// "12 فيديو · 85 سؤال"
pub fn summary(videos: usize, qids: usize) -> String {
    format!("{} · {}", count(videos, &VIDEO), count(qids, &QUESTION))
}
/// "6 محاضرات · 309 سؤال"
pub fn course_line(lectures: usize, qids: usize) -> String {
    format!("{} · {}", count(lectures, &LECTURE), count(qids, &QUESTION))
}
/// The lesson strip's parts, each followed by its source's name:
/// "8 فيديوهات" B&B · "54 سؤال" UWorld.
pub fn videos(n: usize) -> String {
    count(n, &VIDEO)
}
pub fn questions(n: usize) -> String {
    count(n, &QUESTION)
}
pub fn copy(n: usize) -> String {
    format!("انسخ {n}")
}
/// "40 + الباقي"
pub fn rest(size: usize) -> String {
    format!("{size} + الباقي")
}
/// "امتحان 2 · 37"
pub fn test(i: usize, n: usize) -> String {
    format!("امتحان {i} · {n}")
}
/// "363 سؤال ← 10 امتحانات"
pub fn tests_line(qids: usize, tests: usize) -> String {
    format!("{} ← {}", count(qids, &QUESTION), count(tests, &TEST))
}
/// "في السلة: 183 سؤال"
pub fn in_basket(n: usize) -> String {
    format!("في السلة: {}", count(n, &QUESTION))
}
/// "في السلة: 183 سؤال · 5 امتحانات"
pub fn in_basket_tests(n: usize, tests: usize) -> String {
    format!("{} · {}", in_basket(n), count(tests, &TEST))
}
