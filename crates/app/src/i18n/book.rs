pub const TITLE: &str = "صفحة الكتاب";
pub const NO_LECTURE: &str = "الصفحة دي برّه المحاضرات اللي في Steps.";
pub const QUOTE_HELD: &str = "كلام الكتاب هيظهر هنا بعد ما تجاوب، عشان مايكشفش الإجابة.";
/// Above the تحشيشة shown under a quote.
pub const HOOK: &str = "تحشيشة";
pub const MADE: &str = "اللي في Steps من الصفحة دي:";
pub const NOTHING_YET: &str = "لسه مفيش كروت أو أسئلة من الصفحة دي.";
pub const STUDY: &str = "ذاكر الصفحة دي";
pub const OPEN_LECTURE: &str = "افتح المحاضرة";
pub const CHIP_HINT: &str = "افتح صفحة الكتاب";
pub const SLIDE_TITLE: &str = "سلايد";
pub const SLIDE_HINT: &str = "افتح سلايد المحاضرة";
/// The page-read switch (A2): new cards from a page start once it's read.
pub const MARK_READ: &str = "قريت الصفحة دي";
pub const IS_READ: &str = "قريتها ✓";
pub const READ_HINT: &str = "الكروت الجديدة بتبدأ من الصفحات اللي قريتها. دوس تاني لو عايز تشيل العلامة.";

/// Where a lecture or lesson sits in its source: the word, and the numbers
/// apart so the caller can isolate them (`bdi dir="ltr"`). One page is
/// "صفحة 287", never "صفحات 287–287" (UX-39); a slide deck says "سلايد".
/// The one formatter for a page range in the UI.
pub fn pages_span(first: u32, last: u32, slides: bool) -> (&'static str, String) {
    let one = first >= last;
    let word = match (slides, one) {
        (false, true) => PAGE_ONE,
        (false, false) => PAGE_MANY,
        (true, true) => SLIDE_TITLE,
        (true, false) => SLIDE_MANY,
    };
    (word, page_range(first, last))
}

/// "287" for one page, "287–290" for more.
pub fn page_range(first: u32, last: u32) -> String {
    if first >= last { first.to_string() } else { format!("{first}–{last}") }
}

pub const PAGE_ONE: &str = "صفحة";
pub const PAGE_MANY: &str = "صفحات";
pub const SLIDE_MANY: &str = "سلايدز";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_on_one_page_names_one_page() {
        assert_eq!(pages_span(287, 287, false), (PAGE_ONE, "287".to_string()));
        assert_eq!(pages_span(287, 290, false), (PAGE_MANY, "287–290".to_string()));
        assert_eq!((PAGE_ONE, PAGE_MANY), ("صفحة", "صفحات"));
        assert_eq!(pages_span(5, 5, true), (SLIDE_TITLE, "5".to_string()));
        assert_eq!(pages_span(1, 53, true), (SLIDE_MANY, "1–53".to_string()));
        assert_eq!(page_range(32, 32), "32");
        assert_eq!(page_range(32, 43), "32–43");
    }
}
