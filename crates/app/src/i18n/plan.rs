pub const START_HINT: &str = "ابدأ جلسة النهارده";
pub const PROGRESS_HINT: &str = "افتح التقدّم بالتفصيل";
pub const DAY_HINT: &str = "افتح محاضرة اليوم ده";
pub const SETUP_TITLE: &str = "امتحانك إمتى؟";
pub const EXAM_DAY: &str = "يوم الامتحان";
pub const DAYS_LABEL: &str = "أيام المذاكرة في الأسبوع";
pub const CAP_LABEL: &str = "أقصى عدد مراجعات في اليوم";
pub const SAVE: &str = "اعمل الخطة";
pub const EDIT: &str = "عدّل الخطة";
pub const CANCEL: &str = "إلغاء";
pub const PICK_DAY: &str = "اختار يوم بعد النهارده.";
pub const PICK_WEEKDAY: &str = "اختار يوم واحد على الأقل.";
pub const WEEKDAYS: [&str; 7] = ["السبت", "الحد", "الاتنين", "التلات", "الأربع", "الخميس", "الجمعة"];
pub const WEEKDAYS_SHORT: [&str; 7] = ["سبت", "حد", "اتنين", "تلات", "أربع", "خميس", "جمعة"];
pub const MONTHS: [&str; 12] = [
    "يناير",
    "فبراير",
    "مارس",
    "أبريل",
    "مايو",
    "يونيو",
    "يوليو",
    "أغسطس",
    "سبتمبر",
    "أكتوبر",
    "نوفمبر",
    "ديسمبر",
];

pub const EXAM_ON: &str = "الامتحان";
/// "فاضل 40 يوم مذاكرة".
pub fn days_left(n: usize) -> String {
    format!("فاضل {}", super::count::count(n, &super::count::STUDY_DAY))
}
pub const INTEGRATION_FROM: &str = "مرحلة الربط (لفّة تانية وأسئلة مخلوطة أكتر) تبدأ";
pub const DEDICATED_FROM: &str = "الفترة المركّزة (بنك أسئلة ومواضيع ضعيفة وتقييم كل أسبوع، من غير جديد) تبدأ";

pub const PHASE_SYSTEMS: &str = "مرحلة الأجهزة";
pub const PHASE_INTEGRATION: &str = "مرحلة الربط";
pub const PHASE_DEDICATED: &str = "الفترة المركّزة";
/// "بنك الأسئلة: 20 سؤال".
pub fn qbank(n: u32) -> String {
    format!("بنك الأسئلة: {}", super::count::count(n, &super::count::QUESTION))
}
pub const BASELINE: &str = "تقييم البداية (NBME)";
pub const CHECKPOINT: &str = "تقييم دوري (NBME)";
pub const WEEKLY: &str = "تقييم الأسبوع (NBME)";
pub const FREE_120: &str = "Free 120";
pub const NEXT_MILESTONE: &str = "التقييم الجاي";
pub const MILESTONES_TITLE: &str = "التقييمات الذاتية";
pub const MILESTONES_NOTE: &str = "نتيجة النهارده بتقولك فين تركّز، مش بتحكم عليك. السكور بيطلع مع الوقت.";

pub const TODAY: &str = "النهارده";
pub const NEW_ITEMS: &str = "جديد";
pub const REVIEWS: &str = "مراجعة تقريبًا";
pub const APPROX: &str = "تقريبًا";
pub const FROM: &str = "من";
pub const REVISIT: &str = "لفّة تانية على";
pub const REST_TODAY: &str = "النهارده راحة. المذاكرة الجاية:";

pub const FOCUS_LEARN: &str = "جديد ومراجعة";
pub const FOCUS_REVISIT: &str = "لفّة تانية";
pub const FOCUS_MOCK: &str = "تقييم ذاتي";
pub const FOCUS_WEAK: &str = "بنك أسئلة ومواضيع ضعيفة";
pub const FOCUS_CATCH_UP: &str = "يوم لحاق";
pub const FOCUS_REST: &str = "راحة";

pub const ON_TRACK: &str = "الخطة مريحة: كل المحاضرات هتخلص بدري وتلحق تلفّ عليها تاني.";
pub const SNUG: &str = "الخطة ماشية، بس الجديد هياخد جزء من وقت اللفّة التانية.";
pub const TIGHT: &str = "على الخطة دي، فيه أسئلة مش هتلحق قبل الفترة المركّزة:";
pub const TIGHT_ADD: &str = "لو زوّدت أيام مذاكرة في الأسبوع هتلحق كلها. عدد الأيام:";
pub fn tight_cap(cap: u32) -> String {
    format!("لو علّيت حد المراجعات اليومي لـ {cap} هتلحق كلها.")
}
pub const TIGHT_HIGH_YIELD: &str = "حتى بأيام أكتر وحد مراجعات أعلى مش هتلحق كلها، فركّز على high-yield الأول.";
pub const TIGHT_DEDICATED: &str =
    "إنت في الفترة المركّزة، فالوقت لبنك الأسئلة والمواضيع الضعيفة. في الجديد، ركّز على high-yield بس.";
/// The calm offer to move the exam day, never to cut dedicated short.
pub fn later_exam(date: &str) -> String {
    format!("أو أجّل الامتحان لـ {date} وتفضل الفترة المركّزة كاملة. ده تخطيط، مش فشل.")
}
pub const MOVE_EXAM: &str = "انقل الامتحان للميعاد ده";
/// When the last NBME forms aren't there yet, close to dedicated.
pub fn not_ready(date: &str) -> String {
    format!(
        "آخر تقييمات NBME لسه تحت المستوى اللي عايزه. ممكن تأجّل الامتحان لـ {date} وتدّي نفسك 4 أسابيع كمان. ده تخطيط، مش فشل."
    )
}
pub const PICK_DAY_FAR: &str = "اختار يوم في خلال 3 سنين من النهارده.";
pub const DEDICATED: &str = "إنت في الفترة المركّزة: بنك أسئلة ومواضيع ضعيفة وتقييم كل أسبوع. مفيش جديد.";
pub const EXAM_DONE: &str = "الامتحان عدّى. ربنا يوفقك! لو عندك امتحان تاني، حط ميعاده.";

pub const PROGRESS: &str = "وصلت لفين";
pub const FIRST_PASS: &str = "اللفّة الأولى";
pub const SECOND_PASS: &str = "اللفّة التانية";
pub const WEEK: &str = "الأيام الجاية";
pub const LOAD_TITLE: &str = "الشغل في الأسبوعين الجايين";
pub const LOAD_LABEL: &str = "مراجعات وجديد متوقعين كل يوم، النهارده على اليمين";
pub const LOAD_PEAK: &str = "أتقل يوم";
pub const LOAD_KEY_REVIEWS: &str = "مراجعة";
pub const LOAD_KEY_NEW: &str = "جديد";
pub const LOAD_KEY_CAP: &str = "الحد اليومي";
pub const PACE_TITLE: &str = "السرعة لحد الامتحان";
pub const PACE_LABEL: &str = "اللي شفته لحد النهارده، والخطة لحد الامتحان";
pub const PACE_KEY_PAST: &str = "اللي فات";
pub const PACE_KEY_PLAN: &str = "الخطة";
pub const PACE_KEY_FINAL: &str = "الفترة المركّزة";
pub const PACE_DONE_ON: &str = "على الخطة دي، هتكون شفت كل الأسئلة يوم";
pub const PACE_NOT_DONE: &str = "على الخطة دي، فيه أسئلة مش هتلحق تتشاف قبل الفترة المركّزة.";
pub const ALL_DAYS: &str = "الخطة كلها لحد الامتحان";

// Setup: phases, qbank, day loads, catch-up day, vacations.
pub const DEDICATED_WEEKS: &str = "الفترة المركّزة قبل الامتحان";
/// "6 أسابيع".
pub fn weeks(n: u32) -> String {
    super::count::count(n, &super::count::WEEK)
}
pub const QBANK_LABEL: &str = "أسئلة بنك الأسئلة في اليوم العادي";
pub const QBANK_HINT: &str = "ده في مرحلة الأجهزة. في الربط الضعف، وفي الفترة المركّزة 4 أضعاف.";
pub const DAY_LOADS: &str = "شغل كل يوم";
pub const LOADS: [&str; 3] = ["خفيف", "عادي", "تقيل"];
pub const CATCH_UP_DAY: &str = "يوم اللحاق (من غير جديد)";
pub const NO_CATCH_UP: &str = "مفيش";
pub const VACATIONS: &str = "أجازات";
pub const ADD_VACATION: &str = "زوّد أجازة";
pub const VACATION_FROM: &str = "من";
pub const VACATION_TO: &str = "لحد";
pub const REMOVE: &str = "شيلها";
pub const PICK_VACATION: &str = "اختار أول وآخر يوم في كل أجازة، وأولها قبل آخرها.";

// Rebalance after missed work: calm, and undoable.
/// "فاتك 12 جديد من امبارح. نوزّعهم على الأيام الجاية؟"
pub fn missed(n: u32) -> String {
    format!("فاتك {n} جديد من امبارح. نوزّعهم على الأيام الجاية؟")
}
pub const SPREAD: &str = "وزّعهم";
/// "حطّهم في يوم اللحاق (خميس 12 نوفمبر)".
pub fn to_catch_up(day: &str) -> String {
    format!("حطّهم في يوم اللحاق ({day})")
}
pub const SPREAD_DONE: &str = "اتوزّعوا على الأيام الجاية.";
/// "اتحطّوا في يوم اللحاق، خميس 12 نوفمبر."
pub fn moved(day: &str) -> String {
    format!("اتحطّوا في يوم اللحاق، {day}.")
}
pub const UNDO: &str = "رجّع";
pub const PREVIEW: &str = "الجديد في الأيام الجاية، قبل ← بعد";
