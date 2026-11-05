pub const TITLE: &str = "ذاكر في Anki";
pub const BUTTON: &str = "صدّر لـ Anki";
pub const WORKING: &str = "بجهّز الملف…";
pub const DONE: &str = "الملف جاهز";
pub const SAVED_IN: &str = "اتحفظ في";
pub const FAILED: &str = "معرفتش أكمّل التصدير:";
/// "12 محاضرة · 340 سؤال وكارت".
pub fn contents(lectures: usize, items: usize) -> String {
    use super::count::{ITEM, LECTURE, count};
    format!("{} · {}", count(lectures, &LECTURE), count(items, &ITEM))
}
/// "الملف جاهز (340 كارت)."
pub fn done(notes: usize) -> String {
    format!("{DONE} ({}).", super::count::count(notes, &super::count::CARD))
}
