pub const TITLE: &str = "اختصارات الكيبورد";
pub const OPEN: &str = "اعرض الاختصارات";
pub const CLOSE: &str = "اقفل";
pub const NOTE: &str = "بتشتغل على كيبورد التابلت والماك. على الماك استخدم ⌘ بدل Ctrl.";
pub const SCREENS: &str = "تروح لشاشة";
pub const SETTINGS: &str = "الإعدادات";
pub const REVEAL: &str = "تكشف الإجابة";
pub const GOOD: &str = "بعد ما تكشف الإجابة: تختار «تمام» وتكمّل";
pub const UNDO: &str = "ترجع في آخر تقييم";
pub const RATE: &str = "تقيّم نفسك (من نسيت لسهل)";
pub const OPTIONS: &str = "تختار إجابة في سؤال الاختيارات";
pub const NEXT: &str = "تأكّد إجابتك، أو اللي بعده";
pub const MOVE: &str = "الخطوة اللي بعدها أو اللي قبلها في الدرس، والسؤال في الامتحان";
pub const FLAG: &str = "تعلّم السؤال عشان ترجعله";
pub const ESC: &str = "تقفل أي حاجة مفتوحة";
pub const HELP: &str = "القايمة دي";
pub const PAGE: &str = "تروح لصفحة في First Aid";
pub const QUICK: &str = "دوّر بسرعة: شاشة، محاضرة، درس، صفحة، إعداد أو أسئلة محاضرة";
pub const FOCUS: &str = "وضع التركيز في الدرس والمراجعة";
pub const SPLIT: &str = "الفاصل بين السؤال والشرح: الأسهم تحركه";
// Quick search (shortcuts/palette.rs).
pub const QS_TITLE: &str = "دوّر بسرعة";
pub const QS_PLACEHOLDER: &str = "شاشة، محاضرة، درس، رقم صفحة، إعداد، أو «أسئلة» واسم المحاضرة…";
pub const QS_NONE: &str = "مفيش حاجة بالاسم ده.";
pub const QS_HINT: &str = "↑ ↓ تتنقّل · Enter تفتح · Esc تقفل";
pub const KIND_SCREEN: &str = "شاشة";
pub const KIND_TOOL: &str = "أداة";
pub const KIND_PAGE: &str = "صفحة";
pub const KIND_LECTURE: &str = "محاضرة";
pub const KIND_LESSON: &str = "درس";
pub const KIND_SECTION: &str = "قسم في الإعدادات";
pub const KIND_QUESTIONS: &str = "أسئلة المحاضرة";
/// A lecture's topic (UX-15).
pub const KIND_TOPIC: &str = "موضوع";
/// An abbreviation from the dictionary.
pub const KIND_TERM: &str = "اختصار";
/// Nothing found: the same words in the Lessons search…
pub fn search_lessons(q: &str) -> String {
    format!("دوّر في الدروس عن «{q}»")
}
pub const KIND_SCREEN_LESSONS: &str = "الدروس";
/// …or in the abbreviations dictionary.
pub const SEARCH_TERMS: &str = "دوّر في القاموس";
