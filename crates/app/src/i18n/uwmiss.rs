// The UWorld mistake log on the mistakes tab (stats/uworld.rs).
use super::count::{Noun, count};

pub const MISTAKE: Noun =
    Noun { one: "غلطة واحدة", two: "غلطتين", few: "غلطات", many: "غلطة" };

pub const CARD_TITLE: &str = "غلطاتك في UWorld";
pub const CARD_HOW: &str = "اكتب رقم سؤال UWorld غلطت فيه واختار السبب. مع الوقت هتشوف أكتر سبب بتغلط بسببه وأكتر المحاضرات اللي محتاجة رجوع.";
pub const QID: &str = "رقم السؤال";
pub const QID_HINT: &str = "مثلاً 1707";
pub const FROM: &str = "السؤال ده من";
pub const LOG_TITLE: &str = "سجّلها في غلطاتي";
pub const LOGGED_AS: &str = "متسجّلة في غلطاتك:";
pub const REASON: &str = "غلطت فيه ليه؟";
pub const NOTE: &str = "ملاحظة قصيرة (اختياري)";
pub const NOTE_HINT: &str = "مثلاً: لخبطت بين MEN1 و MEN2";
pub const SAVE: &str = "سجّل الغلطة";
pub const UPDATE: &str = "حدّث السبب";
pub const PICK_REASON: &str = "اختار السبب الأول.";
pub const NOTE_TOO_LONG: &str = "الملاحظة طويلة، اختصرها.";
pub const SAVED: &str = "اتسجّلت ✓";

pub const TITLE: &str = "السجل";
pub const EMPTY: &str = "لسه مفيش غلطات متسجّلة.";
pub const PATTERNS: &str = "أنماط غلطك";
pub const TOO_FEW: &str = "سجّل 3 غلطات على الأقل عشان نشوف نمط.";
pub const TIE: &str = "مفيش سبب واحد غالب لسه: الأسباب متقاربة.";
pub const BY_LECTURE: &str = "أكتر المحاضرات اللي بتغلط فيها";
pub const COPY_QIDS: &str = "انسخ أرقامها";
pub const COPIED: &str = "اتنسخ ✓";
pub const OPEN: &str = "افتح المحاضرة";
pub const UNFILED: &str = "مش متربوطة بمحاضرة:";
pub const LIST: &str = "الغلطات";
pub const FILTER: &str = "اعرض الغلطات بالسبب ده بس";
pub const DELETE: &str = "امسح";
pub const CONFIRM_DELETE: &str = "امسحها فعلاً";
pub const CANCEL: &str = "إلغاء";
pub const PRIVACY: &str = "بيتحفظ رقم السؤال والسبب وملاحظتك بس، مش نص السؤال.";

/// "8 غلطات"
pub fn total(n: usize) -> String {
    count(n, &MISTAKE)
}

/// "أكتر سبب: X (5 من 8، 63%)"
pub fn top_reason(label: &str, n: usize, total: usize, percent: usize) -> String {
    format!("أكتر سبب: {label} ({n} من {total}، {percent}%)")
}

/// Advice that follows the top reason: what to change, kindly.
pub fn advice(reason: steps_core::review::ErrorReason) -> &'static str {
    use steps_core::review::ErrorReason as R;
    match reason {
        R::Knowledge => "الحل: ارجع للمحاضرة في First Aid وراجع الكروت بتاعتها قبل البلوك الجاي.",
        R::Misread => "الحل: اقرا آخر سطر في السؤال الأول، وبعدين الحالة كلها، قبل ما تبص على الاختيارات.",
        R::SecondGuessed => "الحل: متغيّرش إجابتك إلا لو لقيت معلومة في السؤال فاتتك، مش مجرد إحساس.",
        R::Reasoning => "الحل: اكتب الخطوات في دماغك: التشخيص، وبعدين الآلية، وبعدين المطلوب بالظبط.",
        R::Timing => "الحل: اتدرّب على بلوكات بالوقت (20 سؤال في 30 دقيقة) وعلّم على السؤال الصعب وارجعله.",
    }
}
