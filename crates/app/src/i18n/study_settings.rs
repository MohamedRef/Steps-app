pub const STORE_MEMORY: &str =
    "مذاكرتك مش بتتحفظ: قاعدة البيانات ما اتفتحتش، فاللي بتذاكره هيروح لما تقفل التطبيق.";
pub const SECTION: &str = "المراجعة";
pub const SECTION_SUB: &str = "نسبة التذكّر والجديد كل يوم";
pub const RETENTION: &str = "نسبة التذكّر المطلوبة";
pub const R85: &str = "أخف";
pub const R90: &str = "متوازن";
pub const R95: &str = "قبل الامتحان";
pub const PERSONAL_1: &str = "الجدول متظبط على مذاكرتك أنت، من";
/// "الجدول متظبط على مذاكرتك أنت، من 520 مراجعة. …"
pub fn personal(n: u32) -> String {
    format!("{PERSONAL_1} {}. بيتحدّث كل أسبوع.", super::count::count(n, &super::count::REVIEW))
}
pub fn default_params(need: usize, so_far: usize) -> String {
    use super::count::{REVIEW, count};
    format!("{DEFAULT_PARAMS_1} {}. لحد دلوقتي: {so_far}.", count(need, &REVIEW))
}
pub const DEFAULT_PARAMS_1: &str = "الجدول هيتظبط على مذاكرتك لما توصل";
pub const NEW_PER_DAY: &str = "أسئلة جديدة في اليوم";
pub const NEW_FROM_PLAN_1: &str = "عندك ميعاد امتحان، فالخطة هي اللي بتحدد الجديد كل يوم. النهارده:";
pub const NEW_FROM_PLAN_2: &str = "جديد. تقدر تغيّر أيام المذاكرة وحد المراجعات من تقدّمي ← خطة الامتحان.";
pub const PAGE_GATE: &str = "الجديد من الصفحات اللي قريتها بس";
pub const PAGE_GATE_NOTE: &str = "كارت جديد مايبدأش غير لما تعلّم صفحته في First Aid إنك قريتها، من صفحة الكتاب أو صفحة المحاضرة. المراجعات مش بتتأثر.";
