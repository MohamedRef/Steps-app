pub const DUE: &str = "مستحق مراجعة";
pub const NEW: &str = "جديد متاح";
/// "جديد مستني تقرا صفحاته في First Aid: 12 سؤال وكارت" (A2).
pub fn locked(n: usize) -> String {
    format!("جديد مستني تقرا صفحاته في First Aid: {}", super::count::count(n, &super::count::ITEM))
}
/// "3 حالات مستنية مراجعة في شاشة الحالات".
pub fn due_cases(n: usize) -> String {
    format!("{} مستنية مراجعة في شاشة الحالات", super::count::count(n, &super::count::CASE))
}
pub const ALL_DONE: &str = "خلّصت المستحق النهارده. شغل حلو.";
pub const NEW_CAP_REACHED: &str = "وخلّصت الجديد اللي محدّده لليوم";
pub const TOMORROW: &str = "بكرة:";
pub const NOTHING_WRITTEN: &str = "المحتوى لسه بيتكتب. أول ما محاضرة تجهز هتلاقيها هنا.";
pub const STORE_FAILED: &str = "مش قادر أفتح قاعدة البيانات، فالمراجعات مش هتتحفظ بعد ما تقفل التطبيق:";
pub const WRITE_FAILED: &str = "في اختيار ما اتحفظش على الجهاز، جرّبه تاني:";
pub const IN_PROGRESS: &str = "محاضرات شغّال فيها";
pub const OF: &str = "من";
pub const STARTED: &str = "بدأتهم";
pub const DUE_CHIP: &str = "مستحق";
pub const LIBRARY: &str = "المحاضرات";
pub const READY: &str = "جاهزة";
pub const WRITING: &str = "بتتكتب";
pub const PAGES: &str = "صفحات";
pub const UNVERIFIED_NOTE: &str = "كل المحتوى لسه ما اتراجعش. أول ما تشوف أي سؤال، علّم عليه صح أو غلط.";
pub const UNVERIFIED_HINT: &str = "علاماتك بتتبعت من الإعدادات › البيانات";
pub const DUE_HINT: &str = "ابدأ مراجعة المستحق";
pub const NEW_HINT: &str = "ابدأ جلسة على الجديد";
pub const FORECAST_HINT: &str = "شوف الخطة والأيام الجاية";
pub const CHECKED_HINT: &str = "علاماتك وتصديرها في الإعدادات";
pub const PLAN_HINT: &str = "افتح خطة المذاكرة";

// The exam plan on Today (#99).
pub fn exam_in(days: i32) -> String {
    match days {
        1 => "الامتحان بكرة. إنت جاهز أكتر مما فاكر.".to_string(),
        2 => "فاضل يومين على الامتحان".to_string(),
        n => format!("فاضل {} على الامتحان", super::count::count(n, &super::count::DAY)),
    }
}
pub const PLAN_TODAY: &str = "خطة النهارده:";
pub const PLAN_NEW: &str = "جديد";
pub const ABOUT: &str = "حوالي";
pub const RING_LABEL: &str = "اللي خلّصته من خطة النهارده";
/// Under the ring: what the percent counts, "3 من 40 اتعملوا".
pub fn ring_caption(done: u32, total: u32) -> String {
    format!("{done} من {total} اتعملوا")
}
pub const NEW_LEFT_1: &str = "فاضل";
pub const NEW_LEFT_2: &str = "جديد من خطة النهارده. كل جلسة بتقرّبك.";
pub const NEW_DONE: &str = "خلّصت جديد النهارده. اللي فاضل مراجعة بس.";
pub const REVISIT: &str = "لفّة تانية على";
pub const WEAK: &str = "النهارده لبنك الأسئلة والمواضيع الضعيفة.";
pub const CATCH_UP: &str = "النهارده يوم لحاق: كمّل اللي فاتك، أو ارتاح لو مفيش.";
/// "النهارده: تقييم البداية (NBME). سجّل النتيجة بعده عشان الخطة تتابعك."
pub fn milestone_today(name: &str) -> String {
    format!("النهارده: {name}. سجّل النتيجة بعده عشان تتابع مستواك.")
}
pub const WEAK_START: &str = "ذاكر المواضيع الضعيفة";
pub const REST: &str = "النهارده راحة في الخطة.";
pub const REST_DUE: &str = "لو عندك مستحق، جلسة قصيرة تكفي.";
pub const AHEAD: &str = "اسبق الخطة بشوية جديد";
pub fn tight_days(days: u32) -> String {
    let days = if days == 1 {
        "يوم مذاكرة زيادة".to_string()
    } else {
        format!("{} زيادة", super::count::count(days, &super::count::STUDY_DAY))
    };
    format!(
        "الخطة محتاجة {days} في الأسبوع عشان كل الجديد يخلص قبل الفترة المركّزة. تقدر تعدّلها من تقدّمي ← خطة الامتحان."
    )
}
pub fn tight_cap(cap: u32) -> String {
    format!("لو علّيت حد المراجعات اليومي لـ {cap}، الخطة تلحق كلها. تقدر تعدّله من تقدّمي ← خطة الامتحان.")
}
pub const TIGHT_HIGH_YIELD: &str = "الوقت مش مكفّي كل الجديد قبل الفترة المركّزة. ركّز على high-yield الأول.";

// The 2026-10-29 redesign: a greeting, the session length, one start.
pub const EYEBROW: &str = "جلسة صغيرة، وفهم ثابت";
pub const READY_Q: &str = "جاهز لجلسة هادية؟";
pub const DONE_Q: &str = "خلّصت النهارده. عاش!";
/// The very first day, before anything is studied: never "done".
pub const WELCOME_Q: &str = "أهلاً بيك. نبدأ من الأول؟";
/// A day with nothing to start yet (a mock day, a rest day) and nothing done.
pub const DAY_Q: &str = "شوف خطة النهارده";
pub const TIME_Q: &str = "معاك قد إيه؟";
pub const TIME_HINT: &str = "طول الجلسة. بيتحفظ كاختيارك العادي.";
/// "5 د" on a session-length chip.
pub fn minutes_short(m: u32) -> String {
    format!("{m} د")
}
pub const HERO_KICKER: &str = "المراجعة المستحقة";
pub const HERO_KICKER_NEW: &str = "خطوة جديدة";
pub const HERO_KICKER_PLAN: &str = "خطة النهارده";
/// The hero's headline from the real counts.
pub fn headline(due: usize, new: usize) -> String {
    use super::count::{NEW_THING, REVIEW, count};
    match (due, new) {
        (0, n) => format!("{}، على قد وقتك.", count(n, &NEW_THING)),
        (d, 0) => format!("{}، وبس.", count(d, &REVIEW)),
        (d, n) => format!("{}، و{}.", count(d, &REVIEW), count(n, &NEW_THING)),
    }
}
pub fn hero_body(due: usize, new: usize) -> String {
    match (due, new) {
        (0, _) => "مفيش مستحق. نبدأ بالجديد على قد الوقت اللي اخترته.".to_string(),
        (_, 0) => "نبدأ بالأضعف، والجلسة بتقف لما وقتك يخلص.".to_string(),
        _ => "نبدأ باللي محتاج تثبيت، وبعده الجديد على قد وقتك.".to_string(),
    }
}
/// "ابدأ الجلسة · 10 دقايق".
pub fn start_session(minutes: u32) -> String {
    format!("ابدأ الجلسة · {}", super::count::count(minutes, &super::count::MINUTE))
}
/// The label beside the big minutes digit: "دقايق" or "دقيقة" to match it.
pub fn minutes_all(n: u32) -> String {
    format!("{} للكل تقريبًا", super::count::word(n, &super::count::MINUTE))
}
/// "حوالي 45 دقيقة للكل تقريبًا", read aloud.
pub fn minutes_all_label(n: u32) -> String {
    format!("{} للكل تقريبًا", super::count::count(n, &super::count::MINUTE))
}
/// "3 جديد + 12 مراجعة" on today's plan line.
pub fn plan_line(new: u32, due: usize) -> String {
    format!("{new} {PLAN_NEW} + {}", super::count::count(due, &super::count::REVIEW))
}
pub fn plan_reviews_only(due: usize) -> String {
    format!("{} بس", super::count::count(due, &super::count::REVIEW))
}
pub const MINUTES_HINT: &str = "شوف الحمل في الأيام الجاية";
pub const OPEN_NEXT_LESSON: &str = "افتح الدرس الجاي";
pub const CONTINUE_HEAD: &str = "كمّل من مكانك";
pub const ALL_LESSONS: &str = "كل الدروس";
pub const MORE_HEAD: &str = "حاجات تانية تفيدك";
pub const QUICK_BANK: &str = "تدريب أسئلة سريع";
pub const CHECKED_ROW: &str = "مراجعة صحة المحتوى";
pub fn checked_sub(n: usize) -> String {
    format!("راجعت صحة {} لحد دلوقتي", super::count::count(n, &super::count::ITEM))
}

// A Study session still open (this run, or restored after a restart).
pub const RESUME_TITLE: &str = "عندك جلسة مذاكرة لسه مفتوحة";
pub const RESUME_SESSION: &str = "كمّل الجلسة";
