use super::count::{QUESTION, count};

pub const TITLE: &str = "فيديوهات B&B وأسئلة UWorld";
pub const HOW: &str = "انسخ الأرقام والزقها في UWorld › Create Test › Custom Test. علّم على الفيديو لما تخلصه.";
pub const DONE: &str = "خلصته";
pub const COPIED: &str = "اتنسخ ✓";
pub const NONE: &str = "أسئلته تحت فيديو تاني";
pub const FIND: &str = "دوّر برقم سؤال UWorld";
pub const PAGE_TITLE: &str = "رقم سؤال UWorld";
pub const PLACEHOLDER: &str = "مثلاً 1707";
pub const FOUND: &str = "السؤال ده من";
pub const FROM_VIDEO: &str = "فيديو B&B:";
pub const OPEN_LECTURE: &str = "افتح المحاضرة";
pub const RETIRED: &str = "UWorld شالت السؤال ده.";
pub const NOT_FOUND: &str = "الرقم ده مش في القايمة. مش كل أسئلة UWorld فيها.";
pub const SOURCE: &str =
    "الأرقام من ملف عام بيربط فيديوهات B&B بأسئلة UWorld (حوالي 86% من البنك)، وبتتحدث كل أسبوع.";
/// "12 فيديو · 85 سؤال"
pub fn summary(videos: usize, qids: usize) -> String {
    format!("{videos} فيديو · {}", count(qids, &QUESTION))
}
pub fn copy(n: usize) -> String {
    format!("انسخ {n}")
}
pub fn copy_left(n: usize) -> String {
    format!("انسخ كل اللي فاضل ({})", count(n, &QUESTION))
}
pub fn page_sub(n: usize) -> String {
    format!("اكتب رقم سؤال غلطت فيه، وأقولك هو من أنهي محاضرة. فيه {n} رقم.")
}
