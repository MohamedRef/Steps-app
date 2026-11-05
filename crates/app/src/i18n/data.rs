pub const EXPORT_TITLE: &str = "نسخة احتياطية من كل حاجة";
pub const EXPORT_BUTTON: &str = "احفظ نسخة";
pub const EXPORT_SAVED_1: &str = "اتحفظت";
/// "اتحفظت 1200 حاجة في:".
pub fn export_saved(rows: u64) -> String {
    format!("{EXPORT_SAVED_1} {} في:", super::count::count(rows, &super::count::THING))
}
/// "ادمج 12 تغيير".
pub fn import_apply(n: usize) -> String {
    format!("{IMPORT_APPLY_1} {}", super::count::count(n, &super::count::CHANGE))
}
/// "تمام، اتدمج 12 تغيير. الجدول اتحدّث."
pub fn import_done(n: usize) -> String {
    format!("{IMPORT_DONE_1} {}. الجدول اتحدّث.", super::count::count(n, &super::count::CHANGE))
}
/// "بس فيه 3 صفوف ما اتدمجوش (…)".
pub fn import_partial(n: usize) -> String {
    let rest = IMPORT_PARTIAL_2.trim_start_matches("صف ");
    format!("{IMPORT_PARTIAL_1} {} {rest}", super::count::count(n, &super::count::ROW))
}
pub const EXPORT_ANDROID_HINT: &str =
    "تلاقيه من تطبيق الملفات أو من الكمبيوتر بالكابل، في Android/data/app.steps/files.";
pub const EXPORT_FAILED: &str = "ما عرفتش أحفظ النسخة:";
pub const EXPORT_SAVE_TO: &str = "احفظها في Downloads أو Drive";
pub const EXPORT_SAVING: &str = "اختار مكان النسخة…";
pub const EXPORT_SAVED_AS: &str = "اتحفظت برّه التطبيق باسم";
pub const EXPORT_SAVED_OUT: &str = "اتحفظت في المكان اللي اخترته.";
pub const EXPORT_SAVE_CANCELLED: &str = "ما اخترتش مكان، النسخة لسه في فولدر التطبيق.";
pub const EXPORT_SAVE_FAILED: &str = "ما عرفتش أنقلها، النسخة لسه في فولدر التطبيق.";
pub const IMPORT_TITLE: &str = "استرجع نسخة";
pub const IMPORT_FOUND: &str = "نسخ لقيتها على الجهاز:";
pub const IMPORT_PICK: &str = "أو اختار ملف";
pub const IMPORT_PICK_FIRST: &str = "اختار ملف النسخة";
pub const IMPORT_PICKING: &str = "اختار الملف…";
pub const IMPORT_PICK_FAILED: &str = "ما عرفتش أفتح اختيار الملفات";
pub const IMPORT_COPY_FAILED: &str =
    "ما عرفتش أفتح الملف ده، أو هو أكبر من نسخة Steps. اختار ملف steps-export.";
pub const IMPORT_TOO_BIG: &str = "الملف ده كبير أوي على إنه نسخة من Steps. اختار ملف steps-export.";
pub const IMPORT_PASTE: &str = "أو الزق النص";
pub const IMPORT_PASTED: &str = "نص ملزوق";
pub const IMPORT_READ: &str = "اقرا النص";
pub const IMPORT_FILE: &str = "الملف:";
pub const IMPORT_FROM: &str = "من جهاز";
pub const IMPORT_AT: &str = "اتعمل";
pub const IMPORT_NOTHING: &str = "كل اللي في النسخة دي موجود هنا بالفعل.";
pub const IMPORT_SAFE: &str = "الجديد هيتضاف، والأحدث هيحل محل القديم. مفيش حاجة هتتمسح.";
pub const IMPORT_APPLY_1: &str = "ادمج";
pub const IMPORT_CANCEL: &str = "إلغاء";
pub const IMPORT_BACK: &str = "رجوع";
pub const IMPORT_AGAIN: &str = "استرجع نسخة تانية";
pub const IMPORT_DONE_1: &str = "تمام، اتدمج";
pub const IMPORT_FAILED: &str = "ما عرفتش أقرا النسخة دي:";
pub const IMPORT_PARTIAL_1: &str = "بس فيه";
pub const IMPORT_PARTIAL_2: &str = "صف ما اتدمجوش (ناقصهم بيانات أو من نسخة أحدث من Steps). الباقي اتحفظ، وتقدر تجرب تاني بعد ما تحدّث التطبيق.";
pub const COL_WHAT: &str = "إيه";
pub const COL_IN_FILE: &str = "في الملف";
pub const COL_NEW: &str = "جديد";
pub const COL_NEWER: &str = "أحدث";
pub const T_REVIEWS: &str = "المراجعات";
pub const T_CHECKS: &str = "علاماتك على المحتوى";
pub const T_SETTINGS: &str = "الإعدادات";
pub const T_OWN: &str = "كروتك";
pub const T_NOTES: &str = "ملاحظاتك";
pub const T_SCORES: &str = "اختباراتك";
