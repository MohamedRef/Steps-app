pub const PRACTISE_TOPIC: &str = "اتمرّن على الموضوع ده";
pub const OPEN_LECTURE: &str = "افتح المحاضرة";
pub const TITLE: &str = "بنك الأسئلة";
pub const MODE_LABEL: &str = "طريقة الحل";
pub const MODE_TUTOR_NAME: &str = "تدريب بشرح";
pub const MODE_TUTOR_HINT: &str = "الشرح بعد كل سؤال";
pub const MODE_TIMED_NAME: &str = "تدريب بالوقت";
pub const MODE_TIMED_HINT: &str = "الشرح في الآخر";
pub const MODE_MOCK_NAME: &str = "بلوك زي الامتحان";
pub const MODE_MOCK_HINT: &str = "الأجهزة اللي تختارها";
pub const SCOPE_ALL: &str = "كل الأسئلة";
pub const SCOPE_CHOSEN: &str = "النطاق اللي اخترته";
pub const MOCK_SCOPE_CHIP: &str = "زي Step 1";
pub const MOCK_SCOPE_NOTE: &str = "البلوك بياخد من كل المحاضرات المكتوبة في الأجهزة اللي اخترتها، موزّعة على المحاضرات والمواضيع. الامتحان الحقيقي (من مايو 2026) 14 بلوك، كل بلوك نص ساعة ولحد 20 سؤال.";
pub const BLOCK_SYSTEMS: &str = "الأجهزة والمواد";
pub const BLOCK_ALL_SYSTEMS: &str = "كل الأجهزة";
pub fn block_systems(n: usize) -> String {
    format!("بلوك من {}", super::count::count(n, &super::count::COURSE))
}
pub const BLOCK_TIMED: &str = "بالوقت";
pub const BLOCK_TIMED_HINT: &str = "90 ثانية للسؤال، والشرح في الآخر";
pub const BLOCK_TUTOR: &str = "بشرح";
pub const BLOCK_TUTOR_HINT: &str = "من غير وقت، والشرح بعد كل سؤال";
pub const ALL_LECTURES: &str = "كل المحاضرات";
pub fn all_of(course: &str) -> String {
    format!("كل محاضرات {course}")
}
pub const CUSTOMISE: &str = "خصّص المحاضرات والمواضيع";
pub const CUSTOMISE_CLOSE: &str = "اقفل التخصيص";
pub const RESET_SCOPE: &str = "رجّع النطاق لكل الأسئلة";
pub const SCOPE_NOW: &str = "النطاق دلوقتي:";
pub const POOL_OF: &str = "من";
pub const ABOUT: &str = "حوالي";
pub const EMPTY_ALL_RIGHT: &str =
    "جاوبت كل أسئلة النطاق ده صح آخر مرة. شغل حلو! شيل «الغلط واللي ما اتشافش بس» لو عايز تعيدها.";
pub const EMPTY_TAGS: &str = "مفيش أسئلة من النوع اللي اخترته في المحاضرات والمواضيع دي.";
pub const EMPTY_TOPICS: &str = "المواضيع اللي اخترتها لسه مالهاش أسئلة اختيار من متعدد.";
pub const WIDEN_ALL_RIGHT: &str = "هات كل أسئلة النطاق";
pub const WIDEN_TAGS: &str = "شيل نوع السؤال";
pub const WIDEN_ALL: &str = "رجّع لكل الأسئلة";
pub const EMPTY_TITLE: &str = "لسه مفيش أسئلة اختيار من متعدد";
pub const EMPTY_BODY: &str = "أول ما محاضرة فيها أسئلة MCQ تخلص، هتلاقيها هنا على طول.";
pub const LECTURES: &str = "المحاضرات";
pub const TOPICS: &str = "المواضيع";
pub const PICK_LECTURE_FOR_TOPICS: &str = "اختار محاضرة أو أكتر عشان تظهر مواضيعها.";
pub const TAGS: &str = "نوع السؤال";
pub const TAG_HIGH_YIELD: &str = "مهم جدًا";
pub const TAG_BOOK: &str = "أسئلة الكتاب";
pub const TAG_NUMBERS: &str = "أرقام";
pub const ONLY_WRONG: &str = "الغلط واللي ما اتشافش بس";
pub const ALL: &str = "الكل";
pub const COUNT: &str = "عدد الأسئلة";
pub const NONE_MATCH: &str = "مفيش أسئلة بالاختيارات دي.";
pub const START_TUTOR: &str = "ابدأ التدريب";
pub const START_TIMED: &str = "ابدأ بالوقت";
pub const PRESET_TUTOR: &str = "تختار، وبعدين تصحّح، فيظهر الشرح وصفحة الكتاب، وتعدّي لما تفهم.";
pub const PRESET_TIMED: &str = "زي الامتحان: الوقت شغّال، تقدر تعلّم على سؤال وترجعله، والشرح كله في الآخر.";
pub const MOCK_TITLE: &str = "نتيجة البلوك";
pub const START_MOCK: &str = "ابدأ البلوك";
pub const HISTORY: &str = "امتحاناتك اللي فاتت";
pub const MODE_TIMED: &str = "بالوقت";
pub const MODE_MOCK: &str = "بلوك";

pub const QUESTION: &str = "سؤال";
pub const OF: &str = "من";
pub const NEXT: &str = "التالي";
pub const PREV: &str = "السابق";
pub const CHECK: &str = "صحّح الإجابة";
pub const PICK_FIRST: &str = "اختار إجابتك الأول، وبعدين صحّحها.";
pub const FLAG: &str = "علّم للمراجعة";
pub const FLAGGED: &str = "متعلّم";
pub const SUBMIT: &str = "سلّم";
pub const FINISH: &str = "خلّص";
pub const LEAVE: &str = "اخرج";
pub const LEFT_ABOUT: &str = "فاضل حوالي";
pub const UNANSWERED: &str = "من غير إجابة";
pub const UPDATE: &str = "تحديث من الإرشادات الحالية";
pub const UPDATE_NOTE: &str = "إجابة الامتحان هي إجابة الكتاب.";
pub const RIGHT: &str = "صح";
pub const YOUR_ANSWER: &str = "إجابتك";
pub const RIGHT_PICKED: &str = "إجابتك صح";
pub const RIGHT_ANSWER: &str = "دي الإجابة الصح";
pub const WRONG: &str = "غلط";
pub const ANSWERED: &str = "متجاوب";
pub const NOW_HERE: &str = "إنت هنا";
/// Read out once a tutor answer is checked (the Check button is gone).
pub fn checked(right: bool, letter: char) -> String {
    if right {
        "إجابتك صح.".to_string()
    } else {
        format!("إجابتك غلط. الإجابة الصح {letter}.")
    }
}
pub const NOT_FOUND: &str = "السؤال ده اتشال من المحتوى.";
pub const BEHIND: &str = "اللي ورا الغلطة";
pub const FACT_NEW: &str = "جديد";
pub const FACT_DUE: &str = "عليه الدور";
pub const FACT_LEARNED: &str = "اتذاكر";
pub fn behind_study(n: usize) -> String {
    format!("ذاكر {} دلوقتي", super::count::count(n, &super::count::CARD))
}

pub const RESULT: &str = "النتيجة";
pub const TOOK: &str = "الوقت";
pub const BY_LECTURE: &str = "حسب المحاضرة";
pub const BY_TOPIC: &str = "حسب الموضوع";
pub const WEAK: &str = "ابدأ مراجعتك بدول";
pub const ALL_STRONG: &str = "مفيش موضوع تحت 60٪. شغل حلو.";
pub const REVIEW: &str = "راجع الأسئلة";
pub const SHOW_ALL: &str = "الكل";
pub const SHOW_WRONG: &str = "الغلط";
pub const SHOW_FLAGGED: &str = "المتعلّم عليها";
pub const RETRY_WRONG: &str = "جاوب الغلط تاني بالشرح";
pub const BACK: &str = "رجوع للبنك";
pub const NEW_PAPER: &str = "حل ورقة جديدة";
pub const SENT_TO_REVIEW: &str = "الأسئلة اللي غلطت فيها هترجعلك في المراجعة قريب.";
pub const REPORT: &str = "بلّغ عن مشكلة في السؤال ده";
pub const CLOSE: &str = "اقفل";
pub const LEAVE_TIMED_HINT: &str = "الامتحان هيستناك، والوقت شغّال";
pub const PAUSED_TITLE: &str = "عندك امتحان لسه مفتوح";
pub const PAUSED_BODY: &str = "جاوبت";
pub const PAUSED_LEFT: &str = "والوقت اللي فاضل حوالي";
pub const PAUSED_OVER: &str = "وقته خلص، فهيتسلّم زي ما هو.";
pub const RESUME: &str = "كمّل";
pub const SUBMIT_NOW: &str = "سلّمه دلوقتي";
pub const CONFIRM_TITLE: &str = "تسلّم دلوقتي؟";
pub const CONFIRM_BLANK_A: &str = "لسه";
pub const CONFIRM_BLANK_B: &str = "سؤال من غير إجابة، والفاضي مالوش درجة.";
pub const CONFIRM_FLAGGED: &str = "متعلّم عليهم كنت عايز ترجعلهم.";
pub const CONFIRM_BACK_TO: &str = "ارجع للسؤال";
pub const CONFIRM_BACK: &str = "ارجع للامتحان";
pub const CONFIRM_SUBMIT: &str = "سلّم كده";

/// A question count the Egyptian way: سؤال واحد, سؤالين, 3–10 أسئلة,
/// 11 and up سؤال.
pub fn questions(n: usize) -> String {
    super::count::count(n, &super::count::QUESTION)
}
