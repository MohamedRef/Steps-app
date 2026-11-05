pub const OPEN_PROGRESS: &str = "افتح تقدّمي";
/// Between list items and clauses.
pub const COMMA: &str = "، ";
use super::count::{DAY, NEW_THING, QUESTION, REVIEW, count};

pub const DONE_REST: &str = "مراجعة النهارده خلصت، استريح.";
pub const WEEK_TITLE: &str = "آخر 7 أيام";
pub const WEEK_RETENTION: &str = "بتفتكر صح:";
pub const WEEK_WEAK: &str = "يستاهلوا رجوع:";

/// "النهارده جاوبت 12 سؤال".
pub fn answered(n: usize) -> String {
    format!("النهارده جاوبت {}", count(n, &QUESTION))
}

/// "واتعلمت حاجتين جداد".
pub fn learned(n: u32) -> String {
    format!("واتعلمت {}", count(n, &NEW_THING))
}

pub fn done_rest(tomorrow: u32) -> String {
    match tomorrow {
        0 => DONE_REST.to_string(),
        n => format!("{DONE_REST} بكرة عندك {}.", count(n, &REVIEW)),
    }
}

pub fn left(n: usize) -> String {
    format!("فاضل {} لو عندك وقت، ولو لأ بكرة موجودة.", count(n, &REVIEW))
}

/// "اتعلمت 10 حاجات جديدة في 3 أيام."
pub fn week_learned(learned: u32, days: usize) -> String {
    format!("اتعلمت {} في {}.", count(learned, &NEW_THING), count(days, &DAY))
}

pub fn exam_in(days: i32) -> String {
    match days {
        ..=0 => "الامتحان النهارده. بالتوفيق.".to_string(),
        n => format!("فاضل على الامتحان {}.", count(n, &DAY)),
    }
}
