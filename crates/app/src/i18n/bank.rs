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
pub const EMPTY_NONE_MARKED: &str =
    "مفيش أسئلة متعلّم عليها في النطاق ده. علّم على أي سؤال وإنت بتحل («علّم» فوق السؤال) وهتلاقيه هنا.";
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
/// Questions he marked on a paper and kept marked (`bank.marked`).
pub const ONLY_MARKED: &str = "المتعلّم عليه";
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
/// «سؤال 7 من 20» in the paper's top bar.
pub fn question_of(n: usize, of: usize) -> String {
    format!("{QUESTION} {n} {OF} {of}")
}
/// The paper's kind in its top bar and on its result: «بلوك · بالوقت»,
/// «تدريب · مع الشرح».
pub fn kind(block: bool, timed: bool) -> String {
    let what = if block { "بلوك" } else { "تدريب" };
    let how = if timed { "بالوقت" } else { "مع الشرح" };
    format!("{what} · {how}")
}
/// «N / M»: opens the navigator under 840dp.
pub const NAV_OPEN: &str = "لستة أسئلة البلوك";
pub const NAV_LABEL: &str = "أسئلة البلوك";
pub const NAV_TITLE: &str = "الأسئلة";
pub const LEGEND_ANSWERED: &str = "اتجاوب";
pub const LEGEND_BLANK: &str = "لسه";
pub const LEGEND_RIGHT: &str = "صح";
pub const LEGEND_WRONG: &str = "غلط";
pub const NEXT: &str = "السؤال اللي بعده";
pub const PREV: &str = "السؤال اللي فات";
pub const CHECK: &str = "تأكيد الإجابة";
/// Read to screen readers while «تأكيد الإجابة» waits for a pick.
pub const PICK_FIRST: &str = "اختار إجابتك الأول، وبعدين أكّدها.";
/// Beside «تأكيد الإجابة»: what a long press and a selection do.
pub const TOOLS_HINT: &str =
    "اضغط على الاختيار مطولًا (أو كليك يمين) عشان تشطب عليه، وحدّد أي كلام في السؤال عشان تعلّم عليه.";
pub const MARK: &str = "علّم";
pub const MARKED: &str = "متعلّم عليه";
/// Ends the paper: its result comes next.
pub const END: &str = "إنهاء البلوك";
pub const LEAVE: &str = "اخرج";
pub const TOOLS: &str = "أدوات: القيم المعملية، ملاحظة، إنهاء البلوك";
/// The tools menu of a finished paper's review, which has no end.
pub const TOOLS_REVIEW: &str = "أدوات: القيم المعملية، ملاحظة";
pub const TOOLS_TITLE: &str = "أدوات";
pub const LEFT_ABOUT: &str = "فاضل حوالي";
/// The clock of a timed paper: «الوقت الباقي 18 دقيقة و42 ثانية».
pub const TIME_LEFT: &str = "الوقت الباقي";
/// The tutor clock: «على السؤال 1:12».
pub const ON_QUESTION: &str = "على السؤال";
/// «18 دقيقة و42 ثانية».
pub fn minutes_seconds(ms: u64) -> String {
    use super::count::{MINUTE, SECOND, count};
    let s = ms / 1000;
    match (s / 60, s % 60) {
        (0, s) => count(s, &SECOND),
        (m, 0) => count(m, &MINUTE),
        (m, s) => format!("{} و{}", count(m, &MINUTE), count(s, &SECOND)),
    }
}
pub fn time_left(ms: u64) -> String {
    format!("{TIME_LEFT} {}", minutes_seconds(ms))
}
pub fn time_on(ms: u64) -> String {
    format!("الوقت على السؤال ده {}", minutes_seconds(ms))
}
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
/// Said on an option he struck out (it is drawn crossed out).
pub const STRUCK: &str = "مشطوب عليه";
/// Read out once a tutor answer is checked (the Check button is gone).
pub fn checked(right: bool, letter: char) -> String {
    if right {
        "إجابتك صح.".to_string()
    } else {
        format!("إجابتك غلط. الإجابة الصح {letter}.")
    }
}
/// The banner over an answered question.
pub const VERDICT_RIGHT: &str = "إجابة صح";
pub const VERDICT_WRONG: &str = "إجابة غلط";
pub const VERDICT_BLANK: &str = "ما اتجاوبش";
/// «اخترت C، والإجابة الصح B»; «اخترت B» when it is right; «الإجابة الصح B» for a blank.
pub fn picked(you: Option<char>, right: char) -> String {
    match you {
        Some(you) if you == right => format!("اخترت {you}"),
        Some(you) => format!("اخترت {you}، والإجابة الصح {right}"),
        None => format!("الإجابة الصح {right}"),
    }
}
pub const EXPLANATION: &str = "الشرح";
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
/// «الوقت 24:10 من 30:00».
pub fn used_of(used: &str, allowed: Option<&str>) -> String {
    match allowed {
        Some(allowed) => format!("{TOOK} {used} {OF} {allowed}"),
        None => format!("{TOOK} {used}"),
    }
}
pub const PER_QUESTION: &str = "متوسط السؤال";
pub const TILE_BLANK: &str = "ما اتجاوبش";
pub const TABLE: &str = "الأسئلة";
pub const COL_NO: &str = "#";
pub const COL_ID: &str = "الـ ID";
pub const COL_LECTURE: &str = "المادة والـ system";
pub const COL_RESULT: &str = "النتيجة";
pub const COL_TIME: &str = "الوقت";
pub const TABLE_HINT: &str = "دوس على أي سؤال يفتحلك بالشرح.";
/// After [`TABLE_HINT`] when the paper has VQ questions (quick search finds `VQ-0001`).
pub const TABLE_HINT_VQ: &str = "وتقدر تكتب أي ID في البحث يطلعلك السؤال.";
/// A row's button, for screen readers: «افتح سؤال 3 بالشرح».
pub fn open_question(n: usize) -> String {
    format!("افتح {QUESTION} {n} بالشرح")
}
pub const BY_COURSE: &str = "حسب الـ system";
pub const WEAK: &str = "ابدأ مراجعتك بدول";
pub const ALL_STRONG: &str = "مفيش موضوع تحت 60٪. شغل حلو.";
pub const REVIEW_WRONG: &str = "راجع الأسئلة الغلط";
pub const RETAKE: &str = "بلوك جديد من الغلط والمتعلّم عليه";
/// The review's top bar: what it goes through.
pub const REVIEWING: &str = "مراجعة";
pub const SHOW_ALL: &str = "الكل";
pub const SHOW_RIGHT: &str = "الصح";
pub const SHOW_WRONG: &str = "الغلط";
pub const SHOW_BLANK: &str = "اللي ما اتجاوبش";
pub const SHOW_MARKED: &str = "المتعلّم عليه";
pub const BACK_TO_RESULT: &str = "رجوع للنتيجة";
pub const BACK: &str = "رجوع للبنك";
pub const NEW_PAPER: &str = "حل ورقة جديدة";
pub const SENT_TO_REVIEW: &str = "الأسئلة اللي غلطت فيها هترجعلك في المراجعة قريب.";
pub const REPORT: &str = "بلّغ عن مشكلة";
pub const CLOSE: &str = "اقفل";
pub const LEAVE_TIMED_HINT: &str = "الامتحان هيستناك، والوقت شغّال";
pub const PAUSED_TITLE: &str = "عندك امتحان لسه مفتوح";
pub const PAUSED_BODY: &str = "جاوبت";
pub const PAUSED_LEFT: &str = "والوقت اللي فاضل حوالي";
pub const PAUSED_OVER: &str = "وقته خلص، فهيتسلّم زي ما هو.";
pub const RESUME: &str = "كمّل";
pub const SUBMIT_NOW: &str = "سلّمه دلوقتي";
pub const CONFIRM_TITLE: &str = "تنهي البلوك دلوقتي؟";
/// «لسه 3 أسئلة من غير إجابة، والفاضي مالوش درجة.»
pub fn confirm_blank(n: usize) -> String {
    format!("لسه {} من غير إجابة، والفاضي مالوش درجة.", questions(n))
}
/// «وفيه سؤالين متعلّم عليهم كنت عايز ترجعلهم.»
pub fn confirm_marked(n: usize) -> String {
    if n == 1 {
        "وفيه سؤال متعلّم عليه كنت عايز ترجعله.".to_string()
    } else {
        format!("وفيه {} متعلّم عليهم كنت عايز ترجعلهم.", questions(n))
    }
}
pub const CONFIRM_BACK_TO: &str = "ارجع للسؤال";
pub const CONFIRM_BACK: &str = "ارجع للامتحان";
pub const CONFIRM_END: &str = "أنهي البلوك كده";

/// A question count the Egyptian way: سؤال واحد, سؤالين, 3–10 أسئلة,
/// 11 and up سؤال.
pub fn questions(n: usize) -> String {
    super::count::count(n, &super::count::QUESTION)
}
