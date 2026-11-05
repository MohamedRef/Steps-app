pub const TAB_PROGRESS: &str = "تقدّمي";
pub const TAB_MISTAKES: &str = "الغلطات";
pub const TAB_PLAN: &str = "خطة الامتحان";
pub const TABS_LABEL: &str = "أقسام التقدّم";

pub const SUMMARY: &str = "لحد النهارده";
pub const SEEN: &str = "شفتها";
pub const SEEN_TWICE: &str = "شفتها مرتين";
pub const APPROVED: &str = "وافقت عليها";
pub const OF: &str = "من";
pub const NOTHING_YET: &str = "لسه ما ذاكرتش حاجة هنا. أول جلسة هتملا الأرقام دي.";

pub const COL_SEEN: &str = "اتشافت";
pub const COL_RETENTION: &str = "بتفتكر";
pub const COL_MCQ: &str = "MCQ صح";
pub const COL_APPROVED: &str = "متراجعة";
pub const NO_DATA: &str = "—";
pub const TOPICS: &str = "المواضيع";
pub const UNTAGGED: &str = "من غير موضوع";

pub const WEAK_TITLE: &str = "أضعف 3 مواضيع دلوقتي";
pub const WEAK_EMPTY: &str = "لسه بدري نحكم. بعد كام مراجعة هنعرف إيه اللي محتاج شغل زيادة.";
pub const WEAK_START: &str = "ذاكر دول";

pub const MISTAKES_TITLE: &str = "اللي غلطت فيه آخر 14 يوم";
pub const MISTAKES_EMPTY: &str = "مفيش غلطات مفتوحة. كمّل كده.";
pub const MISTAKES_START: &str = "جلسة من الغلطات";
pub const MISSED: &str = "غلطت";

pub const SESSION_COUNT: &str = "في الجلسة";
pub const STUDY_HINT: &str = "افتح المراجعة";
pub const APPROVED_HINT: &str = "علاماتك وتصديرها في الإعدادات";
pub const TOPIC_HINT: &str = "ذاكر الموضوع ده لوحده";
pub const OPEN_LECTURE: &str = "افتح المحاضرة";
pub const MISTAKE_HINT: &str = "جرّب السؤال ده تاني";
pub const MISTAKES_TOPIC_HINT: &str = "جرّب أخطاء الموضوع ده";
pub const DEMO: &str = "أرقام تجريبية للعرض بس، مش مذاكرتك.";

// The 2026-10-29 redesign: a period switch with retention, activity and a next step.
pub const HEAD: &str = "تقدّم له معنى";
pub const PERIOD_LABEL: &str = "الفترة";
pub const WEEK: &str = "الأسبوع ده";
pub const MONTH: &str = "الشهر ده";
pub const RECALL: &str = "تذكّر المعلومات";
pub const RECALL_SUB: &str = "من الأسئلة اللي رجعتلك بعد يوم أو أكتر.";
pub const RECALL_NONE: &str = "لسه مفيش أسئلة رجعتلك تاني في الفترة دي.";
pub const RECALL_HINT: &str = "شوف اللي غلطت فيه";
pub fn better(week: bool, pts: u32) -> String {
    format!("أحسن من {} بـ {pts}%", if week { "الأسبوع اللي فات" } else { "الشهر اللي فات" })
}
pub fn worse(week: bool, pts: u32) -> String {
    format!("أقل من {} بـ {pts}%، عادي يحصل", if week { "الأسبوع اللي فات" } else { "الشهر اللي فات" })
}
pub fn same(week: bool) -> String {
    format!("زي {} تقريبًا", if week { "الأسبوع اللي فات" } else { "الشهر اللي فات" })
}
pub const ACTIVITY: &str = "جلساتك على مدار الفترة";
pub const ACTIVITY_UNIT: &str = "دقيقة / يوم";
pub const ACTIVITY_HINT: &str = "افتح خطة الامتحان والأيام الجاية";
pub const ACTIVITY_EMPTY: &str = "مفيش مذاكرة متسجّلة في الفترة دي لسه.";
pub const DAYS_STUDIED: &str = "أيام ذاكرت فيهم";
pub const MINUTES: &str = "دقيقة تركيز";
pub const ANSWERS: &str = "سؤال وكارت جاوبتهم";
pub const MONTH_START: &str = "من 30 يوم";
pub const TODAY: &str = "النهارده";
pub fn day_bar(minutes: u32, answers: u32) -> String {
    use super::count::{ANSWER, MINUTE, count};
    format!("{}، {}", count(minutes, &MINUTE), count(answers, &ANSWER))
}
pub const NEXT_STEP: &str = "خطوة تفيدك دلوقتي";
