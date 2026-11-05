pub const CHIP_ADD: &str = "＋ ملاحظة";
pub const CHIP_HAS: &str = "ملاحظتك";
pub const ADD_HINT: &str = "اكتب ملاحظة";
pub const EDIT_HINT: &str = "افتح ملاحظتك";
pub const TITLE_NEW: &str = "ملاحظة جديدة";
pub const TITLE_EDIT: &str = "ملاحظتك";
pub const FIELD: &str = "الملاحظة";
pub const HINT: &str = "بالعربي أو بالإنجليزي: اللي عايز تفتكره هنا…";
pub const PRIVATE: &str = "ملاحظاتك بتتحفظ على أجهزتك بس، وبتروح معاك في النسخة الاحتياطية وفي Anki.";
pub const SAVE: &str = "احفظ";
pub const DELETE: &str = "امسحها";
pub const CANCEL: &str = "إلغاء";
pub const TOO_LONG: &str = "الملاحظة طويلة، خليها 2000 حرف بالكتير. لو محتاج أكتر، اعمل كارت.";
pub const FAILED: &str = "ما عرفتش أحفظ الملاحظة، وكلامك لسه هنا:";
pub const ON_LESSON: &str = "درس";
pub const PAGE_NONE: &str = "مفيش ملاحظات على الصفحة دي لسه.";
pub const ADD_PAGE: &str = "اكتب ملاحظة على الصفحة دي";
pub const LIST_TITLE: &str = "ملاحظاتك";
pub const LIST_EMPTY: &str =
    "لسه مكتبتش ملاحظات. اضغط «＋ ملاحظة» تحت أي كارت أو سؤال أو خطوة في درس، أو من صفحة الكتاب.";
pub const COURSES: &str = "المواد";
pub const ALL: &str = "الكل";
pub const SEARCH: &str = "دوّر في ملاحظاتك";
pub const NO_MATCH: &str = "مفيش ملاحظة فيها الكلام ده.";
/// "ملاحظة على ص 284".
pub fn on_page(page: u32) -> String {
    format!("ملاحظة على {}", super::cite::page(page))
}
/// "ملاحظاتك على ص 284".
pub fn on_page_title(page: u32) -> String {
    format!("ملاحظاتك على {}", super::cite::page(page))
}
pub fn count(n: usize) -> String {
    super::count::count(n, &super::count::NOTE)
}
