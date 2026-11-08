pub const NONE_YET: &str = "الدروس لسه بتتكتب. أول ما درس يجهز هتلاقيه هنا.";
pub const OF: &str = "من";
pub const START_FIRST: &str = "ابدأ أول درس";
pub const CONTINUE: &str = "كمّل";
pub const ALL_LESSONS_DONE: &str = "خلّصت كل الدروس المكتوبة. عاش!";
pub const UNVERIFIED_NOTE: &str = "الدروس لسه ما اتراجعتش: الكتاب هو المرجع، ولو لقيت حاجة مش مظبوطة قولّي.";
pub const WRITING: &str = "بتتكتب";
pub const PAGES: &str = "صفحات";
pub const NEW: &str = "جديد";
pub const EXPLAIN: &str = "اشرحهولي";
pub const EXPLAIN_HINT: &str = "افتح الدرس اللي بيشرح السؤال ده";
pub const OPEN_LECTURE: &str = "افتح المحاضرة";

pub const CLOSE: &str = "اقفل الدرس";
pub const PROGRESS_LABEL: &str = "موضعك في الدرس";
pub const NAV_REQUIRED: &str = "كمّل المطلوب في الخطوة دي أو اختار «عدّي الخطوة دي» عشان تكمل.";
pub const VIEW_TITLE: &str = "شكل الدرس";
pub const VIEW_STEPS: &str = "خطوات";
pub const VIEW_STEPS_HINT: &str = "زرار «كمّل» كبير تحت";
pub const VIEW_PAGES: &str = "صفحات";
pub const VIEW_PAGES_HINT: &str = "زي الكتاب، بتقلب الصفحة";
pub const TO_PAGES: &str = "اعرض الدرس صفحات زي الكتاب";
pub const TO_STEPS: &str = "اعرض الدرس خطوات";
pub const PAGE_OF: &str = "صفحة";
pub const OF_N: &str = "من";
pub const BACK: &str = "رجوع";
pub const NEXT: &str = "كمّل";
pub const FINISH: &str = "خلّصت";
pub const SKIP: &str = "عدّي الخطوة دي";

pub const THINK: &str = "فكّر الأول";
pub const THINK_HINT: &str = "قول الإجابة في دماغك، وبعدين اكشف.";
pub const REVEAL: &str = "اكشف";
pub const SHOW_ALL: &str = "اكشف الكل";
pub const MNEMONIC: &str = "احفظها كده";
pub const ZOOM_IN: &str = "كبّر الرسمة";
pub const ZOOM_OUT: &str = "صغّر الرسمة";
pub const ZOOM_FIT: &str = "على قد الشاشة";
pub const ZOOM_CLOSE: &str = "اقفل الصورة";
pub const ZOOM_HINT: &str = "قرّب بصباعين أو بالزراير، ودوس مرتين علشان تكبّر أو ترجع.";
pub const TIP: &str = "نصيحة";
pub const EXAM: &str = "الممتحن بيحب دي";
pub const TRAP: &str = "خلّي بالك";
pub const ANALOGY: &str = "تخيّل كده";
pub const JOKE: &str = "تحشيشة";
pub const UPDATE: &str = "تحديث من الجايدلاين";
pub const BOOK_WINS: &str = "في الامتحان اكتب إجابة الكتاب.";

pub const CHECK: &str = "سؤال سريع";
pub const ORDER: &str = "رتّب";
pub const UNDO: &str = "تراجع";
pub const REMOVE: &str = "شيل";
pub const MARK_MINE_RIGHT: &str = "✓ اختيارك";
pub const MARK_MINE_WRONG: &str = "✗ اختيارك";
pub const MARK_RIGHT: &str = "✓ الصح";
pub const KEY: &str = "الإجابة الصح";
pub const RETRY: &str = "حاول تاني";
pub const MATCH: &str = "وصّل";
pub const MATCH_HINT: &str = "اختار من الشمال، وبعدين اللي يناسبه من اليمين.";
pub const SORT: &str = "صنّف";
pub const SORT_HINT: &str = "اسحب كل كارت لخانته، أو دوس عليه وبعدين على الخانة.";
pub const SORT_PICKED: &str = "تمام، دلوقتي دوس على الخانة اللي تناسبه.";
pub const SORT_DROP: &str = "حطّه هنا";
pub const SORT_EMPTY: &str = "لسه فاضية";
pub const CASE: &str = "حالة صغيرة";
pub const CASE_NEXT: &str = "اللي بعده";
pub const CASE_SHOW: &str = "اكشف";
pub const TABLE_HINT: &str = "فيه خانات مستخبية: فكّر فيها ودوس علشان تتأكد.";
pub const CHECK_IT: &str = "صحّح";
pub const RIGHT: &str = "صح كده! 👏";
pub const NOT_QUITE: &str = "قريب، بس مش دي.";

pub const TRY: &str = "جرّب بنفسك";
pub const TRY_OF: &str = "كارت";
pub const TRY_DONE: &str = "تمام كده ✓";

pub const DONE_TITLE: &str = "خلّصت الدرس";
pub const FIRST_TRY: &str = "صح من أول مرة";
pub const RECAP: &str = "الخلاصة";
pub const PRACTICE: &str = "راجع كروت الدرس";
pub const NEXT_LESSON: &str = "الدرس الجاي";
pub const BACK_TO_LIST: &str = "رجوع للدروس";

pub fn placed_of(k: usize, n: usize) -> String {
    format!("اخترت {k} من {n}")
}

pub fn slot(n: usize) -> String {
    format!("الخطوة رقم {n}")
}

pub fn should_be(n: usize) -> String {
    format!("مكانها رقم {n}")
}

pub fn sort_left(left: usize, n: usize) -> String {
    format!("فاضل {left} من {n}")
}

pub fn in_place(right: usize, n: usize) -> String {
    format!("{right} من {n} في مكانهم.")
}

pub fn misses(n: usize) -> String {
    if n == 1 {
        "وصّلتهم كلهم، بغلطة واحدة في السكة.".into()
    } else {
        match super::count::count(n, &super::count::MISTAKE) {
            m if n == 2 => format!("وصّلتهم كلهم، ب{m} في السكة."),
            m => format!("وصّلتهم كلهم، بـ {m} في السكة."),
        }
    }
}

pub fn score_line(right: u32, asked: u32) -> &'static str {
    if right == asked {
        "ولا غلطة. الدرس ده بقى بتاعك."
    } else if right * 2 >= asked {
        "كويس جدًا. اللي غلطت فيه هيرجعلك في المراجعة."
    } else {
        "عادي جدًا من أول مرة، ده درس جديد. الكروت هتثبّته."
    }
}

// The 2026-10-29 redesign of the list.
pub const HEAD: &str = "دروس قصيرة، وفهم أعمق";
pub const SEARCH: &str = "محاضرة، موضوع، أو كلمة…";
pub const SEARCH_LABEL: &str = "دوّر في الدروس";
pub const CLEAR_SEARCH: &str = "امسح البحث";
pub fn no_match(q: &str) -> String {
    format!("مفيش درس فيه «{q}». جرّب كلمة تانية أو اسم المحاضرة بالإنجليزي.")
}
/// `قلب › المحاضرة 3`: the course, then the lecture's number in it.
pub fn lecture_n(course: &str, n: &str) -> String {
    format!("{course} › المحاضرة {n}")
}
pub fn lessons_n(n: usize) -> String {
    match n {
        1 => "درس واحد".to_string(),
        2 => "درسين".to_string(),
        3..=10 => format!("{n} دروس"),
        _ => format!("{n} درس"),
    }
}
/// "2 من 5 خلّصتهم".
pub fn done_of(done: usize, total: usize) -> String {
    format!("{done} من {total} خلّصتهم")
}
pub const LECTURE_PAGE: &str = "صفحة المحاضرة";
pub const OTHER_LECTURES: &str = "محاضرات تانية";
pub const COURSE_NO_LESSONS: &str =
    "دروس المادة دي لسه بتتكتب. محاضراتها تحت، وكل واحدة بتفتح صفحتها بالكروت وصفحات الكتاب.";
pub const SEARCH_ALL_COURSES: &str = "دوّر في كل المواد";
pub const SHOW_LECTURE: &str = "اعرض دروس المحاضرة دي";
pub const STATE_CURRENT: &str = "كمّل";
pub const STATE_NEXT: &str = "الجاي";
pub const STATE_NEW: &str = "جديد";
pub const STATE_DONE: &str = "خلّصته";
/// "خطوة 3 من 9".
pub fn step_of(at: usize, n: usize) -> String {
    format!("خطوة {at} من {n}")
}
pub const NOW_KICKER: &str = "الدرس الحالي";
pub const NEXT_KICKER: &str = "الدرس الجاي";
/// The Lessons list's card back to the lesson left part-way.
pub const LEFT_OFF_KICKER: &str = "كمّل من حيث وقفت";
/// After a lecture's lesson counts, when one of them is part-way.
pub const LESSON_STARTED: &str = "فيه درس بدأته";
/// Under the picked lecture's Continue button: "هتكمّل من خطوة 4 من 9".
pub fn resume_at(at: usize, n: usize) -> String {
    format!("هتكمّل من خطوة {at} من {n}")
}

// Reminders under a step: a تحشيشة this topic already has (docs/CONTENT.md).
pub const REMIND: &str = "افتكر التحشيشة";
pub const REMIND_LIST: &str = "افتكرها";
pub const REMIND_FROM: &str = "من درس";
pub const REMIND_FROM_HINT: &str = "افتح الدرس اللي التحشيشة دي اتشرحت فيه";
