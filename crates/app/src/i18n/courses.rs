pub const TITLE: &str = "المواد";
pub const ACTIVE: &str = "بذاكرها";
pub const MAINTENANCE: &str = "مراجعة خفيفة";
pub const ARCHIVED: &str = "مستخبية";
pub const LATER: &str = "بعدين";
pub const ACTIVE_HINT: &str = "بتظهر في اليوم والمراجعة والخطة.";
pub const MAINTENANCE_HINT: &str = "خلصت قرايتها. مراجعات خفيفة بس عشان تفضل حاضرة ليوم الامتحان.";
pub const ARCHIVED_HINT: &str = "مستخبية من اليوم، بس موجودة في المراجع وفي ملف Anki.";
pub const LATER_HINT: &str = "لسه ما بدأتهاش. محاضراتها موجودة أول ما تحب تبدأ.";
pub const G_GENERAL: &str = "General Principles";
pub const G_SYSTEMS: &str = "Organ Systems";
pub const G_GENERAL_HINT: &str = "Section II في الكتاب: الأساسيات اللي بتتسأل جوه كل جهاز.";
pub const G_SYSTEMS_HINT: &str = "Section III: كل جهاز لوحده، من الـ anatomy للـ pharmacology.";
pub const STATUS: &str = "حالة المادة";
pub const OPEN_LECTURES: &str = "افتح المحاضرات";
pub const NO_CONTENT: &str = "لسه مفيش محاضرات للمادة دي.";
pub const ALL: &str = "كل المواد";
pub const PICK_TITLE: &str = "تذاكر إيه دلوقتي؟";
pub const MAP_LINK: &str = "خريطة المواد وتقدّمك";

pub fn chip_label(name: &str) -> String {
    format!("المادة المعروضة: {name}. دوس عشان تبدّل")
}
use super::count::{CARD, COURSE, LECTURE, LESSON, PAGE, count};

pub fn all_sub(courses: usize) -> String {
    format!("{} مع بعض", count(courses, &COURSE))
}
/// "3 من 12 درس", "1 من درسين".
pub fn lessons_done(done: usize, all: usize) -> String {
    format!("{done} من {}", count(all, &LESSON))
}
pub fn lectures_only(n: usize) -> String {
    format!("{}، الدروس جاية", count(n, &LECTURE))
}

/// "614 صفحة من First Aid في 16 مادة".
pub fn pages_line(pages: u32, courses: usize) -> String {
    format!("{} من First Aid في {}", count(pages, &PAGE), count(courses, &COURSE))
}
/// "بدأت 12 من 40 كارت".
pub fn started(seen: usize, total: usize) -> String {
    format!("بدأت {seen} من {}", count(total, &CARD))
}
/// The tile's foot: "قلب · ص 284–328".
pub fn tile_line(short: &str, from: u32, to: u32) -> String {
    format!("{short} · ص {from}–{to}")
}
/// "First Aid ص 284–328 (45 صفحة)".
pub fn book_pages(from: u32, to: u32, pages: u32) -> String {
    format!("First Aid ص {from}–{to} ({})", count(pages, &PAGE))
}
pub fn content_line(written: usize, lectures: usize, items: usize, lessons: usize) -> String {
    format!("{} · {} · {}", ready_lectures(written, lectures), count(items, &CARD), count(lessons, &LESSON))
}
/// "مكتوب 3 من 12 محاضرة".
pub fn ready_lectures(written: usize, total: usize) -> String {
    format!("مكتوب {written} من {}", count(total, &LECTURE))
}
