use super::count::{CARD, QUESTION, count};

pub const MARK: &str = "راجعها تاني";
pub const WHY: &str = "فيه حاجة من الصفحة دي غلطت فيها آخر مرة. العلامة بتمشي أول ما تجاوبها صح.";
pub const SETTING_TITLE: &str = "علامات المراجعة";
pub const SETTING_LINE: &str = "علّم في الدروس وصفحات الكتاب الحاجات اللي غلطت فيها آخر مرة";
/// "ذاكر الـ 3 كروت دول"
pub fn study(n: usize) -> String {
    format!("ذاكر {} دلوقتي", count(n, &CARD))
}
/// "وفيه سؤال واحد من الصفحة دي محتاج تبص عليه تاني."
pub fn questions(n: usize) -> String {
    format!("وفيه {} من الصفحة دي محتاج تبص عليه تاني.", count(n, &QUESTION))
}
