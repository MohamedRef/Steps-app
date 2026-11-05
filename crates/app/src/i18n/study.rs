pub const ELSEWHERE_ALL: &str = "اعرض كل المواد";

pub fn due_elsewhere(n: usize) -> String {
    match n {
        1 => "فيه كارت واحد مستحق في مادة تانية.".to_string(),
        2 => "فيه كارتين مستحقين في مواد تانية.".to_string(),
        3..=10 => format!("فيه {n} كروت مستحقة في مواد تانية."),
        _ => format!("فيه {n} كارت مستحق في مواد تانية."),
    }
}
pub const TITLE: &str = "جلسة مذاكرة";
pub const EMPTY_LIBRARY: &str = "لسه مفيش محاضرة جاهزة للمذاكرة هنا. أول ما محاضرة تتكتب هتلاقيها في القايمة، ولحد ده ما يحصل تقدر تذاكر من Anki.";
pub const LECTURES: &str = "المحاضرات";
pub const PICK_ALL: &str = "اختار الكل";
pub const CLEAR_ALL: &str = "امسح الاختيار";
pub const TOPICS: &str = "المواضيع";
pub const TIME: &str = "الوقت";
pub const NEW: &str = "جديد";
pub const ABOUT: &str = "حوالي";
pub const START: &str = "يلا نبدأ";
pub const NOTHING_NOW: &str = "مفيش حاجة عليها الدور في اللي اخترته دلوقتي. جرّب محاضرة تانية أو ارجع بعدين.";
pub const END: &str = "إنهاء الجلسة";
pub const PENDING_TITLE: &str = "عندك جلسة شغّالة";
pub const PENDING_BODY: &str = "تحب تضيف الأسئلة دي بعد اللي فاضل في جلستك، ولا تبدأ جلسة جديدة بيها؟";
pub const PENDING_ADD: &str = "ضيفهم بعد جلستي";
pub const PENDING_REPLACE: &str = "ابدأ جلسة جديدة";
pub const PENDING_KEEP: &str = "لا، كمّل جلستي";
pub const OF: &str = "من";

pub const BADGE_NEW: &str = "جديد";
pub const BADGE_REVIEW: &str = "مراجعة";
pub const BADGE_RETRY: &str = "تاني مرة";
pub const UNVERIFIED: &str = "لسه ما اتراجعش";
pub const UNVERIFIED_HINT: &str = "اتكتب من الكتاب ولسه إنت ما أكدتوش.";
pub const KIND_CARD: &str = "كارت";
pub const KIND_CLOZE: &str = "كمّل الفراغ";
pub const KIND_ENUMERATE: &str = "عدّد";
pub const KIND_ESSAY: &str = "مقالي";
pub const KIND_MCQ: &str = "اختيار";
pub const KIND_CASE: &str = "حالة";
pub const KIND_TABLE: &str = "جدول";
pub const KIND_ORDER: &str = "رتّب";
pub const BOOK_QUOTE: &str = "من الكتاب";
pub const UPDATE_TITLE: &str = "تحديث عن الكتاب";
pub const UPDATE_AFTER: &str = "افتحه بعد ما تجاوب";
pub const UPDATE_BOOK_WINS: &str = "في الامتحان، إجابة الكتاب هي اللي بتتحسب.";
pub const NOT_HERE_YET: &str = "النوع ده من الأسئلة ليه شاشة لسه بتتبني.";
pub const SKIP: &str = "عدّي";

pub const THINK_FIRST: &str = "جاوب في دماغك الأول، وبعدين اكشف.";
pub const REVEAL: &str = "اكشف الإجابة";
pub const BLANK: &str = "الفراغ";
pub const CHECK: &str = "صحّح";
pub const REVEAL_BLANKS: &str = "مش فاكر، وريني";
pub const CLOZE_TIP: &str = "الأرقام بتتصحح من غير الوحدات والمسافات.";
pub const CLOZE_RIGHT: &str = "مظبوط.";
pub const CLOZE_CLOSE: &str = "قريب جدًا. بص على الإجابة كاملة.";
pub const CLOZE_WRONG: &str = "الإجابة الصح مكتوبة فوق. هترجعلك تاني قريب.";

pub const ENUM_GOAL_A: &str = "فيه";
pub const ENUM_GOAL_C: &str = "منهم تعدّي.";
/// "فيه 8 نقط، ولو افتكرت 5 منهم تعدّي."
pub fn enum_goal(total: usize, pass: usize) -> String {
    let total = super::count::count(total, &super::count::POINT);
    format!("{ENUM_GOAL_A} {total}، ولو افتكرت {pass} {ENUM_GOAL_C}")
}
pub const ENUM_PLACEHOLDER: &str = "اكتب اللي فاكره، أو قوله لنفسك بصوت عالي (اختياري)";
pub const SHOW_POINTS: &str = "خلصت، وريني النقط";
pub const TICK_WHAT_YOU_GOT: &str = "علّم على اللي افتكرته.";
pub const PRE_TICKED: &str = "علّمت لك على اللي لقيته في كلامك، وغيّر براحتك.";
pub const ENUM_PASS: &str = "عدّيت.";
pub const ENUM_SHORT_A: &str = "فاضلك";
pub const ENUM_SHORT_B: &str = "عشان تعدّي.";
pub const WHAT_I_WROTE: &str = "اللي كتبته";
/// The memory hook under an enumerate list once its points are open.
pub const ENUM_HOOK: &str = "تحشيشة تحفّظها";

pub const ESSAY_HOW: &str = "اكتب العناوين بس، كل عنوان في سطر، زي ما هتكتبهم في الامتحان.";
pub const ESSAY_PLACEHOLDER: &str = "Definition\nPathogenesis\nClinical picture\n…";
pub const COMPARE: &str = "قارن بالإجابة النموذجية";
pub const MODEL_OUTLINE: &str = "الإجابة النموذجية";
pub const TICK_HEADINGS: &str = "علّم على العناوين اللي جت في بالك.";
pub const HEADINGS_GOT: &str = "العناوين اللي جبتها:";

pub const GRADE_LABEL: &str = "افتكرتها قد إيه؟";
pub const GRADE_HINT: &str = "قيّم اللي افتكرته، مش صعوبة السؤال. ده اللي بيحدد إمتى يرجعلك.";
pub const UNDO: &str = "↶ تراجع";
pub const UNDO_HINT: &str = "رجّع آخر تقييم";
pub const UNDONE: &str = "رجّعت تقييمك، وكأنك ما قيّمتش. جاوب وقيّم من جديد.";
pub const UNDO_FAILED: &str = "ما عرفتش أرجّع التقييم. جرّب تاني بعد شوية.";
pub const SAVE_FAILED: &str = "ما عرفتش أحفظ تقييمك، فلسه على نفس السؤال وإجابتك زي ما هي. قيّم تاني.";
pub const UNDO_LAST: &str = "↶ رجّع آخر تقييم";
pub const SUGGESTED: &str = "اقتراحي متعلّم عليه، والقرار ليك.";
pub const SUGGESTED_SHORT: &str = "مقترح";
pub const AGAIN: &str = "تاني";
pub const HARD: &str = "صعب";
pub const GOOD: &str = "تمام";
pub const EASY: &str = "سهل";

pub const SUMMARY_TITLE: &str = "خلصت الجلسة، برافو عليك";
pub const SUMMARY_EMPTY: &str = "الجلسة خلصت";
pub const SUM_ITEMS: &str = "سؤال";
pub const SUM_RIGHT: &str = "صح من أول مرة";
pub const SUM_NEW: &str = "جديد";
pub const SUM_ALL_SOLID: &str = "كله كان ثابت. المراجعة الجاية هتيجي بعد فترة أطول.";
pub const SUM_BACK_SOON: &str = "هيرجعولك قريب";
pub const NEXT_SESSION: &str = "الجلسة الجاية:";
pub const NEXT_TOMORROW: &str = "ارجع بكرة لجلسة قصيرة، وهتلاقي المراجعات مستنياك.";
pub const ANOTHER: &str = "جلسة كمان";
pub const NOW: &str = "دلوقتي";
pub const AFTER: &str = "بعد";
pub const TOMORROW: &str = "بكرة";
