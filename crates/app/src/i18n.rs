//! UI text, in Egyptian Arabic. One module per feature: a feature chunk adds
//! its own `pub mod <feature>` block and touches nothing else here.
//! Medical terms stay in English (wrap them in `bdi` in the markup).

pub mod app {
    pub const NAME: &str = "Steps";
    pub const TAGLINE: &str = "مذاكرة USMLE Step 1، جلسة قصيرة كل مرة";
}

/// Counted nouns in Egyptian Arabic, so no screen writes "5 ساعة" or
/// "1 أيام": `count(1, &HOUR)` is "ساعة", 2 is "ساعتين", 3–10 take the
/// plural ("5 ساعات") and 11 and up the singular ("11 ساعة"). Digits are
/// always 0–9.
pub mod count {
    /// The forms of one noun: alone for 1, the dual for 2, the plural for
    /// 3–10, and the singular that follows 0 and 11 up.
    pub struct Noun {
        pub one: &'static str,
        pub two: &'static str,
        pub few: &'static str,
        pub many: &'static str,
    }

    pub const SECOND: Noun =
        Noun { one: "ثانية", two: "ثانيتين", few: "ثواني", many: "ثانية" };
    pub const MINUTE: Noun =
        Noun { one: "دقيقة", two: "دقيقتين", few: "دقايق", many: "دقيقة" };
    pub const HOUR: Noun = Noun { one: "ساعة", two: "ساعتين", few: "ساعات", many: "ساعة" };
    pub const DAY: Noun = Noun { one: "يوم", two: "يومين", few: "أيام", many: "يوم" };
    pub const MONTH: Noun = Noun { one: "شهر", two: "شهرين", few: "شهور", many: "شهر" };
    pub const WEEK: Noun =
        Noun { one: "أسبوع", two: "أسبوعين", few: "أسابيع", many: "أسبوع" };
    pub const QUESTION: Noun =
        Noun { one: "سؤال واحد", two: "سؤالين", few: "أسئلة", many: "سؤال" };
    pub const LECTURE: Noun = Noun {
        one: "محاضرة واحدة", two: "محاضرتين", few: "محاضرات", many: "محاضرة"
    };
    pub const TOPIC: Noun =
        Noun { one: "موضوع واحد", two: "موضوعين", few: "مواضيع", many: "موضوع" };
    /// Questions and cards together.
    pub const ITEM: Noun = Noun {
        one: "سؤال واحد", two: "سؤالين", few: "أسئلة وكروت", many: "سؤال وكارت"
    };
    pub const NOTE: Noun = Noun {
        one: "ملاحظة واحدة", two: "ملاحظتين", few: "ملاحظات", many: "ملاحظة"
    };
    pub const REVIEW: Noun = Noun {
        one: "مراجعة واحدة", two: "مراجعتين", few: "مراجعات", many: "مراجعة"
    };
    pub const NEW_THING: Noun =
        Noun {
            one: "حاجة جديدة", two: "حاجتين جداد", few: "حاجات جديدة", many: "حاجة جديدة"
        };
    pub const TIME: Noun = Noun { one: "مرة", two: "مرتين", few: "مرات", many: "مرة" };
    pub const CASE: Noun =
        Noun { one: "حالة واحدة", two: "حالتين", few: "حالات", many: "حالة" };

    pub const FIELD: Noun =
        Noun { one: "خانة واحدة", two: "خانتين", few: "خانات", many: "خانة" };
    pub const LESSON: Noun =
        Noun { one: "درس واحد", two: "درسين", few: "دروس", many: "درس" };
    pub const COURSE: Noun =
        Noun { one: "مادة واحدة", two: "مادتين", few: "مواد", many: "مادة" };
    pub const CARD: Noun =
        Noun { one: "كارت واحد", two: "كارتين", few: "كروت", many: "كارت" };
    pub const STEP: Noun =
        Noun { one: "خطوة واحدة", two: "خطوتين", few: "خطوات", many: "خطوة" };
    pub const MISTAKE: Noun =
        Noun { one: "غلطة واحدة", two: "غلطتين", few: "غلطات", many: "غلطة" };
    pub const ANSWER: Noun =
        Noun { one: "إجابة واحدة", two: "إجابتين", few: "إجابات", many: "إجابة" };
    pub const LINE: Noun =
        Noun { one: "سطر واحد", two: "سطرين", few: "سطور", many: "سطر" };
    pub const THING: Noun =
        Noun { one: "حاجة واحدة", two: "حاجتين", few: "حاجات", many: "حاجة" };
    pub const CHANGE: Noun =
        Noun { one: "تغيير واحد", two: "تغييرين", few: "تغييرات", many: "تغيير" };
    pub const ROW: Noun = Noun { one: "صف واحد", two: "صفين", few: "صفوف", many: "صف" };
    pub const POINT: Noun =
        Noun { one: "نقطة واحدة", two: "نقطتين", few: "نقط", many: "نقطة" };
    pub const STUDY_DAY: Noun =
        Noun {
            one: "يوم مذاكرة واحد", two: "يومين مذاكرة", few: "أيام مذاكرة", many: "يوم مذاكرة"
        };
    pub const PAGE: Noun =
        Noun { one: "صفحة واحدة", two: "صفحتين", few: "صفحات", many: "صفحة" };

    /// Any integer type; a negative number counts as 0.
    pub fn count<N: TryInto<u64>>(n: N, noun: &Noun) -> String {
        let n: u64 = n.try_into().unwrap_or(0);
        let rest = n % 100;
        match n {
            1 => noun.one.to_string(),
            2 => noun.two.to_string(),
            _ if (3..=10).contains(&rest) => format!("{n} {}", noun.few),
            _ => format!("{n} {}", noun.many),
        }
    }

    /// The bare noun for a number shown on its own (a big digit with a
    /// label beside it): the plural for 2–10, the singular otherwise.
    pub fn word<N: TryInto<u64>>(n: N, noun: &Noun) -> &'static str {
        let n: u64 = n.try_into().unwrap_or(0);
        if (2..=10).contains(&(n % 100)) && n != 0 { noun.few } else { noun.many }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn bare_words() {
            assert_eq!(word(7, &MINUTE), "دقايق");
            assert_eq!(word(45, &MINUTE), "دقيقة");
            assert_eq!(word(1, &MINUTE), "دقيقة");
            assert_eq!(word(0, &MINUTE), "دقيقة");
        }

        /// Every noun keeps the digit-free forms for 1 and 2 and the digit
        /// for the rest, so no line reads "7 ساعة" or "2 أيام".
        #[test]
        fn every_noun_agrees() {
            for noun in [
                &SECOND, &MINUTE, &HOUR, &DAY, &MONTH, &QUESTION, &LECTURE, &TOPIC, &ITEM, &REVIEW,
                &NEW_THING, &TIME, &CASE, &FIELD, &LESSON, &COURSE, &CARD, &STEP, &MISTAKE, &ANSWER, &LINE,
                &THING, &CHANGE, &ROW, &POINT, &STUDY_DAY, &PAGE,
            ] {
                assert_eq!(count(1, noun), noun.one);
                assert_eq!(count(2, noun), noun.two);
                assert_eq!(count(7, noun), format!("7 {}", noun.few));
                assert_eq!(count(11, noun), format!("11 {}", noun.many));
                assert_eq!(count(0, noun), format!("0 {}", noun.many));
            }
        }

        #[test]
        fn egyptian_number_agreement() {
            assert_eq!(count(1, &HOUR), "ساعة");
            assert_eq!(count(2, &HOUR), "ساعتين");
            assert_eq!(count(5, &HOUR), "5 ساعات");
            assert_eq!(count(10, &DAY), "10 أيام");
            assert_eq!(count(11, &HOUR), "11 ساعة");
            assert_eq!(count(0, &QUESTION), "0 سؤال");
            assert_eq!(count(103, &DAY), "103 أيام");
            assert_eq!(count(120, &DAY), "120 يوم");
            assert_eq!(count(1, &QUESTION), "سؤال واحد");
        }
    }
}

/// How every screen cites the book and names a lecture.
pub mod cite {
    /// "ص 179".
    pub fn page(page: u32) -> String {
        format!("ص {page}")
    }
    /// "سلايد 25": a lecture's slide (courses taught from slides).
    pub fn slide(n: u32) -> String {
        format!("سلايد {n}")
    }
}

/// The web app's boot screen (`platform/web.rs`).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod web {
    pub const NO_CONTENT: &str = "Steps مقدرتش تجيب المحتوى. اتأكد إن النت شغال وافتحها تاني.";
}

/// A book page's sheet (pagesheet/mod.rs).
pub mod sheet {
    pub const RESIZE: &str = "اسحب لفوق أو لتحت عشان تكبّرها أو تصغّرها";
    pub const CLOSE: &str = "اقفل";
}

/// A lesson step's abbreviations chip and sheet (abbrsheet/mod.rs).
pub mod abbr {
    pub const CHIP: &str = "اختصارات";
    pub const CHIP_HINT: &str = "اعرف معنى الاختصارات اللي في الخطوة دي";
    pub const TITLE: &str = "الاختصارات في الخطوة دي";
    pub const UNVERIFIED: &str = "المعاني دي لسه ما اتراجعتش.";
    /// The abbreviations dictionary (abbrdict/mod.rs).
    pub const DICT_TITLE: &str = "قاموس الاختصارات";
    pub const DICT_OPEN: &str = "القاموس كله";
    pub const DICT_COURSES: &str = "المادة";
    pub const DICT_SEARCH: &str = "دوّر على اختصار أو معناه";
    pub const DICT_NO_MATCH: &str = "مفيش اختصار بالكلام ده في المادة دي.";
    pub const DICT_EMPTY: &str = "لسه مفيش دروس فيها اختصارات.";
    pub const ABBR: super::count::Noun =
        super::count::Noun {
            one: "اختصار واحد", two: "اختصارين", few: "اختصارات", many: "اختصار"
        };

    pub fn count(n: usize) -> String {
        super::count::count(n, &ABBR)
    }

    /// "في درسين": the lessons that use a term.
    pub fn count_lessons(n: usize) -> String {
        format!("موجود في {}:", super::count::count(n, &super::count::LESSON))
    }
}

pub mod book {
    pub const TITLE: &str = "صفحة الكتاب";
    pub const NO_LECTURE: &str = "الصفحة دي برّه المحاضرات اللي في Steps.";
    pub const QUOTE_HELD: &str = "كلام الكتاب هيظهر هنا بعد ما تجاوب، عشان مايكشفش الإجابة.";
    /// Above the تحشيشة shown under a quote.
    pub const HOOK: &str = "تحشيشة";
    pub const MADE: &str = "اللي في Steps من الصفحة دي:";
    pub const NOTHING_YET: &str = "لسه مفيش كروت أو أسئلة من الصفحة دي.";
    pub const STUDY: &str = "ذاكر الصفحة دي";
    pub const OPEN_LECTURE: &str = "افتح المحاضرة";
    pub const CHIP_HINT: &str = "افتح صفحة الكتاب";
    pub const SLIDE_TITLE: &str = "سلايد";
    pub const SLIDE_HINT: &str = "افتح سلايد المحاضرة";
    /// The page-read switch (A2): new cards from a page start once it's read.
    pub const MARK_READ: &str = "قريت الصفحة دي";
    pub const IS_READ: &str = "قريتها ✓";
    pub const READ_HINT: &str = "الكروت الجديدة بتبدأ من الصفحات اللي قريتها. دوس تاني لو عايز تشيل العلامة.";
}

/// Weak-fact marks (weak/mod.rs): neutral words, no red counts.
pub mod weak {
    use super::count::{CARD, QUESTION, count};

    pub const MARK: &str = "راجعها تاني";
    pub const WHY: &str = "فيه حاجة من الصفحة دي غلطت فيها آخر مرة. العلامة بتمشي أول ما تجاوبها صح.";
    pub const SETTING_TITLE: &str = "علامات المراجعة";
    pub const SETTING_LINE: &str = "علّم في الدروس وصفحات الكتاب الحاجات اللي غلطت فيها آخر مرة";
    /// "ذاكر الـ 3 كروت دول"
    pub fn study(n: usize) -> String {
        format!("ذاكر {} دلوقتي", count(n, &CARD))
    }
    /// "وفيه سؤال واحد من الصفحة دي محتاج تبص عليه تاني."
    pub fn questions(n: usize) -> String {
        format!("وفيه {} من الصفحة دي محتاج تبص عليه تاني.", count(n, &QUESTION))
    }
}

/// B&B videos and UWorld QIDs per lecture (qbank/mod.rs).
pub mod qbank {
    use super::count::{QUESTION, count};

    pub const TITLE: &str = "فيديوهات B&B وأسئلة UWorld";
    pub const HOW: &str =
        "انسخ الأرقام والزقها في UWorld › Create Test › Custom Test. علّم على الفيديو لما تخلصه.";
    pub const DONE: &str = "خلصته";
    pub const COPIED: &str = "اتنسخ ✓";
    pub const NONE: &str = "أسئلته تحت فيديو تاني";
    pub const FIND: &str = "دوّر برقم سؤال UWorld";
    pub const PAGE_TITLE: &str = "رقم سؤال UWorld";
    pub const PLACEHOLDER: &str = "مثلاً 1707";
    pub const FOUND: &str = "السؤال ده من";
    pub const FROM_VIDEO: &str = "فيديو B&B:";
    pub const OPEN_LECTURE: &str = "افتح المحاضرة";
    pub const RETIRED: &str = "UWorld شالت السؤال ده.";
    pub const NOT_FOUND: &str = "الرقم ده مش في القايمة. مش كل أسئلة UWorld فيها.";
    pub const SOURCE: &str =
        "الأرقام من ملف عام بيربط فيديوهات B&B بأسئلة UWorld (حوالي 86% من البنك)، وبتتحدث كل أسبوع.";
    /// "12 فيديو · 85 سؤال"
    pub fn summary(videos: usize, qids: usize) -> String {
        format!("{videos} فيديو · {}", count(qids, &QUESTION))
    }
    pub fn copy(n: usize) -> String {
        format!("انسخ {n}")
    }
    pub fn copy_left(n: usize) -> String {
        format!("انسخ كل اللي فاضل ({})", count(n, &QUESTION))
    }
    pub fn page_sub(n: usize) -> String {
        format!("اكتب رقم سؤال غلطت فيه، وأقولك هو من أنهي محاضرة. فيه {n} رقم.")
    }
}

pub mod nav {
    pub const LABEL: &str = "التنقل";
    pub const TODAY: &str = "النهارده";
    pub const LEARN: &str = "الدروس";
    pub const STUDY: &str = "المراجعة";
    pub const BANK: &str = "الأسئلة";
    pub const CASES: &str = "حالات";
    pub const ORAL: &str = "شفوي";
    pub const STATS: &str = "تقدّمي";
    pub const SETTINGS: &str = "الإعدادات";
    pub const MORE: &str = "المزيد";
    pub const MORE_PRACTICE: &str = "درّب طريقة تفكيرك";
    pub const MORE_TOOLS: &str = "مراجعك وأدواتك";
}

pub mod today {
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
}

pub mod study_settings {
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
}

pub mod study {
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
    pub const NOTHING_NOW: &str =
        "مفيش حاجة عليها الدور في اللي اخترته دلوقتي. جرّب محاضرة تانية أو ارجع بعدين.";
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
}

pub mod anki {
    pub const TITLE: &str = "ذاكر في Anki";
    pub const BUTTON: &str = "صدّر لـ Anki";
    pub const WORKING: &str = "بجهّز الملف…";
    pub const DONE: &str = "الملف جاهز";
    pub const SAVED_IN: &str = "اتحفظ في";
    pub const FAILED: &str = "معرفتش أكمّل التصدير:";
    /// "12 محاضرة · 340 سؤال وكارت".
    pub fn contents(lectures: usize, items: usize) -> String {
        use super::count::{ITEM, LECTURE, count};
        format!("{} · {}", count(lectures, &LECTURE), count(items, &ITEM))
    }
    /// "الملف جاهز (340 كارت)."
    pub fn done(notes: usize) -> String {
        format!("{DONE} ({}).", super::count::count(notes, &super::count::CARD))
    }
}

/// The two optional taps around an answer (`crate::metacog`): how sure he
/// was before it came out, and why he missed it.
pub mod metacog {
    pub const CONFIDENCE_LABEL: &str = "متأكد قد إيه؟";
    pub const OPTIONAL: &str = "اختياري";
    pub const SURE: &str = "متأكد";
    pub const UNSURE: &str = "مش متأكد";
    pub const GUESS: &str = "تخمين";

    pub const REASON_LABEL: &str = "غلطت ليه؟";
    /// The study player: a reason chip rates the item "تاني" too.
    pub const REASON_LABEL_AGAIN: &str = "نسيتها؟ اختار السبب وهتتسجل «تاني»";
    pub const KNOWLEDGE: &str = "مكنتش عارف المعلومة";
    pub const MISREAD: &str = "قريت السؤال غلط";
    pub const SECOND_GUESSED: &str = "غيّرت إجابتي الصح";
    pub const REASONING: &str = "خطوة في التفكير";
    pub const TIMING: &str = "استعجلت / الوقت";

    /// On a mistake he was sure of.
    pub const WAS_SURE: &str = "كنت متأكد";
    pub const WAS_SURE_HINT: &str = "غلطة كنت متأكد منها: دي أهم حاجة تتراجع، فبتيجي الأول.";
    pub const ALL_REASONS: &str = "الكل";
    pub const BY_REASON: &str = "حسب السبب";
    /// The mistakes view: misses he was sure of, back for one more try.
    pub const RETEST_TITLE: &str = "جرّب تاني اللي كنت متأكد منه";
    pub const RETEST_BODY: &str = "غلطات كنت متأكد منها من يومين لأسبوع. الغلطة دي بتتصلح بسرعة بس بتتنسي لو ما اتراجعتش، فمحاولة واحدة دلوقتي بتثبّتها.";
    pub const RETEST_START: &str = "جرّب الكروت";
    pub const RETEST_MCQS: &str = "جرّب الأسئلة";
}

pub mod bank {
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
}

pub mod stats {
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
}

pub mod scores {
    // The self-assessment tab on the progress screen (stats/scores.rs).
    pub const TAB: &str = "اختباراتي";
    pub const VERDICT_TITLE: &str = "جاهزيتك";
    pub const V_NONE: &str = "لسه ما سجّلتش أي فورم NBME. أول فورم هو نقطة بداية تعرف منها فين، مش حكم عليك.";
    pub const V_ONE: &str = "فورم واحد بداية كويسة. محتاج فورم كمان عشان نشوف الاتجاه رايح فين.";
    pub const V_READY: &str = "آخر فورمين 68% أو أكتر والاتجاه ثابت أو طالع: شكلك جاهز. كمّل على نفس الإيقاع.";
    pub const V_CLOSE: &str = "قريب من الخط. كمّل مذاكرة وجرّب فورم كمان قبل ما تقرّر.";
    pub const V_NOT_YET: &str =
        "لسه تحت المنطقة دي. لو الميعاد قرّب، فكّر تأجّله شوية: ده تعديل في الخطة، مش فشل.";
    pub const LAST_NBME: &str = "آخر NBME:";
    pub const TREND_UP: &str = "طالع";
    pub const TREND_DOWN: &str = "نازل";
    pub const TREND_FLAT: &str = "ثابت تقريباً";
    pub const POINTS: &str = "نقط";
    pub const THEN: &str = "ثم";

    pub const CHART_TITLE: &str = "درجاتك مع الوقت";
    pub const CHART_LABEL: &str = "نسب اختباراتك مع الوقت، الأحدث على الشمال";
    pub const KEY_BAND: &str = "على الحافة 62–68%";
    pub const KEY_READY: &str = "خط 68%";
    pub const KEY_PRACTICE: &str = "تمرين (UWSA وغيره)";

    pub const LIST_TITLE: &str = "اختباراتك";
    pub const ADD: &str = "سجّل اختبار";
    pub const EMPTY: &str = "مفيش اختبارات متسجلة لسه. بعد ما تخلّص فورم سجّل نسبته هنا.";
    pub const EDIT: &str = "عدّل";
    pub const DELETE: &str = "امسح";
    pub const CONFIRM_DELETE: &str = "امسحه فعلاً";
    pub const CANCEL: &str = "إلغاء";
    pub const SAVE: &str = "احفظ";
    pub const FAILED: &str = "ما اتحفظش:";
    pub const PRACTICE_TAG: &str = "تمرين";
    pub const PROBABILITY_SHORT: &str = "احتمال النجاح في التقرير";

    pub const FORM_NEW: &str = "اختبار جديد";
    pub const FORM_EDIT: &str = "تعديل الاختبار";
    pub const KIND: &str = "نوع الاختبار";
    pub const KIND_OTHER: &str = "غيره";
    pub const FORM: &str = "رقم الفورم أو اسمه";
    pub const FORM_HINT: &str = "30";
    pub const DATE: &str = "عملته يوم";
    pub const PERCENT: &str = "النسبة %";
    pub const PROBABILITY: &str = "احتمال النجاح في التقرير % (لو موجود)";
    pub const PROBABILITY_HINT: &str = "اكتبه زي ما هو في تقرير NBME، أو سيبه فاضي.";
    pub const NOTE: &str = "ملاحظة";

    pub const BAD_PERCENT: &str = "اكتب رقم من 0 لـ 100.";
    pub const BAD_DATE: &str = "اختار تاريخ.";
    pub const FORM_TOO_LONG: &str = "اسم الفورم طويل، اختصره.";
    pub const NOTE_TOO_LONG: &str = "الملاحظة طويلة، اختصرها.";
    pub const IN_THE_FUTURE: &str = "التاريخ ده لسه ما جاش.";

    pub const RULE_NOTE: &str = "القاعدة: آخر فورمين NBME عند 68% أو أكتر واتجاه ثابت أو طالع. ده رقم بيستخدمه المدرّسين والطلبة، مش حد رسمي، ومفيش هنا احتمال نجاح متخمّن: الأرقام كلها اللي إنت كتبتها.";
    pub const PRACTICE_NOTE: &str =
        "UWSA وباقي البنوك تمرين: نسبتها مش زي نسبة NBME، فمش بتدخل في الحكم ومتقارنهاش بيها.";
    pub const PROBABILITY_NOTE: &str =
        "احتمال نجاح 90% في التقرير معناه إن 1 من 10 بنفس الدرجة ما نجحش، فاقراه كمدى مش كوعد.";
    pub const FREE120_NOTE: &str = "الـ Free 120 الجديد أحسن وقت ليه قبل الامتحان بـ 3 لـ 7 أيام.";
}

pub mod plan {
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
    pub const DEDICATED_FROM: &str =
        "الفترة المركّزة (بنك أسئلة ومواضيع ضعيفة وتقييم كل أسبوع، من غير جديد) تبدأ";

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
    pub const TIGHT_HIGH_YIELD: &str =
        "حتى بأيام أكتر وحد مراجعات أعلى مش هتلحق كلها، فركّز على high-yield الأول.";
    pub const TIGHT_DEDICATED: &str =
        "إنت في الفترة المركّزة، فالوقت لبنك الأسئلة والمواضيع الضعيفة. في الجديد، ركّز على high-yield بس.";
    /// The calm offer to move the exam day, never to cut dedicated short.
    pub fn later_exam(date: &str) -> String {
        format!("أو أجّل الامتحان لـ {date} وتفضل الفترة المركّزة كاملة. ده تخطيط، مش فشل.")
    }
    pub const MOVE_EXAM: &str = "انقل الامتحان للميعاد ده";
    pub const PICK_DAY_FAR: &str = "اختار يوم في خلال 3 سنين من النهارده.";
    pub const DEDICATED: &str = "إنت في الفترة المركّزة: بنك أسئلة ومواضيع ضعيفة وتقييم كل أسبوع. مفيش جديد.";
    pub const EXAM_DONE: &str = "الامتحان عدّى. ربنا يوفقك! لو عندك امتحان تاني، حط ميعاده.";

    pub const PROGRESS: &str = "وصلت لفين";
    pub const FIRST_PASS: &str = "اللفّة الأولى";
    pub const SECOND_PASS: &str = "اللفّة التانية";
    pub const WEEK: &str = "الأيام الجاية";
    pub const LOAD_TITLE: &str = "الحِمل في الأسبوعين الجايين";
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
    pub const DAY_LOADS: &str = "حِمل كل يوم";
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
}

pub mod table {
    pub const HINT_TAP: &str = "دوس على كل ؟ تشوف الإجابة، وبعدين علّم: عرفتها ✓ ولا لأ ✗.";
    pub const HINT_TYPE: &str = "اكتب كل خانة فاضية، وبعدين دوس صحّح.";
    pub const REVEAL_CELL: &str = "اكشف الخانة";
    pub const TYPE_CELL: &str = "اكتب الخانة";
    pub const REVEAL_ALL: &str = "اكشف الكل";
    pub const CHECK_TYPED: &str = "صحّح";
    pub const TYPING_ON: &str = "عايز أكتب";
    pub const TYPING_OFF: &str = "من غير كتابة";
    pub const KNEW_IT: &str = "عرفتها";
    pub const MISSED_IT: &str = "ماعرفتهاش";
    pub const MARK_RIGHT: &str = "اعتبرها صح";
    pub const MARK_WRONG: &str = "اعتبرها غلط";

    pub fn mark_each(left: usize) -> String {
        if left == 1 {
            "فاضل خانة واحدة: عرفتها ولا لأ؟".to_string()
        } else {
            format!("فاضل {}: علّم على كل واحدة عرفتها ولا لأ.", super::count::count(left, &super::count::FIELD))
        }
    }

    pub fn recalled(right: usize, total: usize) -> String {
        if right == total {
            format!("كل الخانات صح ({total} من {total}). حلو أوي!")
        } else {
            format!("افتكرت {right} من {}.", super::count::count(total, &super::count::FIELD))
        }
    }
}

pub mod order {
    pub const HINT: &str =
        "دوس على الخطوات بالترتيب. تقدر ترجّع خطوة بالدوس عليها، أو تحركها بالأسهم أو بالسحب.";
    pub const PLACED: &str = "ترتيبك";
    pub const YOUR_ORDER: &str = "ترتيبك";
    pub const POOL: &str = "الخطوات";
    pub const NEXT_SLOT: &str = "الخطوة الجاية هنا";
    pub const CORRECT_ORDER: &str = "الترتيب في الكتاب";
    pub const CHECK: &str = "صحّح";
    pub const CLEAR: &str = "ابدأ من الأول";
    pub const SEND_BACK: &str = "رجّعها للخطوات";
    pub const MOVE_UP: &str = "حركها لفوق";
    pub const MOVE_DOWN: &str = "حركها لتحت";
    pub const IN_PLACE: &str = "في مكانها";
    pub const ALL_IN_PLACE: &str = "كل الخطوات في مكانها. ترتيب ممتاز!";

    pub fn in_place(right: usize, total: usize) -> String {
        format!("{right} من {total} في مكانها. الخطوات المعلّمة بالأحمر محتاجة نظرة تانية.")
    }

    pub fn should_be(position: usize) -> String {
        format!("مكانها {position}")
    }

    /// When one early slip pushes many steps off their numbers.
    pub fn moves_to_fix(moves: usize) -> String {
        if moves == 1 {
            "الترتيب قريب: نقل خطوة واحدة بس بيظبطه.".to_string()
        } else {
            format!("الترتيب قريب: نقل {} بس بيظبطه.", super::count::count(moves, &super::count::STEP))
        }
    }
}

pub mod oral {
    pub const MISSED_HINT: &str = "راجع السؤال ده في المراجعة";
    pub const TITLE: &str = "امتحان شفوي";
    pub const LECTURE: &str = "المحاضرة";
    pub const ALL_LECTURES: &str = "كل المحاضرات الجاهزة";
    pub const SIZE: &str = "عدد الأسئلة";
    pub const THINK: &str = "وقت التفكير";
    pub const VOICE: &str = "اقرا الأسئلة بصوت عالي";
    pub const RATE: &str = "سرعة الصوت";
    pub const RATES: [&str; 3] = ["أبطأ", "عادي", "أسرع"];
    pub const AUTO: &str = "كمّل لوحدك بعد الإجابة";
    pub const AUTO_HINT: &str =
        "لو ما ضغطتش على تقييم في 8 ثواني، يعدّي للسؤال اللي بعده من غير ما يسجّل حاجة.";
    pub const AVAILABLE: &str = "ينفع يتسألوا شفوي:";
    pub const START: &str = "ابدأ";
    pub const EMPTY: &str = "لسه مفيش محاضرات جاهزة فيها أسئلة شفوي. أول ما محاضرة تخلص هتلاقيها هنا.";
    pub const NO_VOICE: &str = "مفيش صوت إنجليزي شغّال على الجهاز ده، فالأسئلة هتظهر مكتوبة بس. على أندرويد: الإعدادات ← تحويل النص إلى كلام ← نزّل صوت English.";

    pub const HONEST: &str = "مفيش تسجيل لصوتك ولا تصحيح آلي: انت اللي بتقارن إجابتك بنقاط الكتاب وتقيّم نفسك.";
    pub const PAUSED: &str = "المؤقت واقف";
    pub const PAUSE_TIMER: &str = "وقّف المؤقت";
    pub const RESUME_TIMER: &str = "كمّل المؤقت";
    pub const RESTART_TIMER: &str = "عيد من الأول";
    pub const NOTES: &str = "اكتب نقاط إجابتك (اختياري)";
    pub const NOTES_HINT: &str = "ابدأ بالمراحل الرئيسية…";
    pub const YOUR_NOTES: &str = "نقاطك";
    pub const TICK_HINT: &str = "علّم على النقط اللي قلتها.";
    pub const SAID: &str = "قلت";
    pub const ENOUGH: &str = "كفاية للنجاح";
    pub const LISTEN: &str = "اسمع السؤال";
    pub const THINKING: &str = "فكّر وجاوب بصوتك";
    pub const WHAT_ELSE: &str = "وإيه كمان؟";
    pub const ANSWER: &str = "الإجابة";
    pub const FOLLOW_UP: &str = "سؤال الممتحن بعد الكارت";
    pub const AGAIN_BADGE: &str = "تاني مرة";
    pub const PASS: &str = "كفاية تذكر";
    pub const OF: &str = "من";
    pub const LEFT: &str = "فاضل";
    pub const SHOW: &str = "اعرض الإجابة";
    pub const REPEAT: &str = "عيد السؤال";
    pub const AGAIN: &str = "تاني";
    pub const AGAIN_HINT: &str = "هيرجعلك بعد 3 أسئلة";
    pub const GOOD: &str = "تمام";
    pub const GOOD_HINT: &str = "جاوبت صح";
    pub const END: &str = "إنهاء";
    /// The Android notification channel for oral mode in the background.
    pub const CHANNEL: &str = "المذاكرة الشفوي";
    pub const CHANNEL_HINT: &str = "التحكم في الشفوي وهو شغّال في الخلفية";

    pub const DONE: &str = "خلصت الجلسة";
    pub const DONE_GOOD: &str = "تمام";
    pub const DONE_AGAIN: &str = "تاني";
    pub const DONE_HEARD: &str = "سمعتها بس";
    pub const MISSED: &str = "الأسئلة اللي محتاجة مراجعة";
    pub const NONE_MISSED: &str = "ولا سؤال وقع منك. شغل حلو!";
    pub const ANOTHER: &str = "جلسة كمان";
    pub const BACK: &str = "رجوع للإعدادات";
}

pub mod case {
    pub const SAVE_FAILED: &str = "ما عرفتش أحفظ الإجابة. إجابتك لسه موجودة؛ جرّب تاني قبل ما تقفل.";
    pub const EMPTY: &str = "لسه مفيش حالات. هتظهر هنا أول ما محاضراتها تتكتب.";
    pub const STEP: &str = "خطوة";
    /// "5 خطوات", "خطوتين".
    pub fn steps(n: usize) -> String {
        super::count::count(n, &super::count::STEP)
    }
    pub const NEW_DATA: &str = "بيانات جديدة";
    pub const STEPS_LABEL: &str = "خطوات الحالة";
    pub const NOW: &str = "دلوقتي";
    pub const KNOWN: &str = "المعطيات لحد دلوقتي";
    pub const CHECK: &str = "صحّح";
    pub const PICK_FIRST: &str = "اختار قرارك الأول، وبعدين صحّحه.";
    pub const RECAP: &str = "مراجعة تفكيرك";
    pub const YOURS: &str = "إجابتك:";
    pub const MARK_RIGHT: &str = "الصح";
    pub const MARK_YOURS: &str = "اختيارك";
    pub const RIGHT: &str = "صح، برافو.";
    pub const WRONG: &str = "مش دي. الإجابة الصح متعلّمة.";
    pub const CORRECT_WAS: &str = "الصح:";
    pub const SHOW_ANSWER: &str = "ورّيني الإجابة";
    pub const GOT_IT: &str = "جاوبتها صح";
    pub const MISSED: &str = "فاتتني";
    pub const NEXT: &str = "الخطوة اللي بعدها";
    pub const FINISH: &str = "خلّص الحالة";
    pub const SCORE: &str = "خطوات صح";
    pub const ALL_RIGHT: &str = "كل الخطوات صح. الحالة دي هترجعلك بعد مدة أطول.";
    pub const SOME_RIGHT: &str = "حلو. بص على الخطوات اللي فاتتك، والحالة هترجعلك قريب في المراجعة.";
    pub const FEW_RIGHT: &str = "ولا يهمك، الحالات دي بتتعلم بالتكرار. هترجعلك بكرة.";
    pub const AGAIN: &str = "عيدها";
    pub const BACK: &str = "رجوع للقائمة";
    pub const UNVERIFIED: &str = "لسه ما اتراجعش";
    pub const UPDATE: &str = "تحديث عن الكتاب";
    pub const DUE: &str = "مستحقة مراجعة";
}

pub mod desk {
    pub const TABLET: &str = "التابلت والقلم";
    pub const TABLET_SUB: &str = "القايمة الجانبية ولمحة القلم";
    pub const MAC: &str = "الماك والكيبورد";
    pub const MAC_SUB: &str = "القايمة الجانبية ولمحة الماوس والاختصارات";
    pub const NAV_STYLE: &str = "القايمة الجانبية في الشاشة العريضة";
    pub const NAV_LABELS: &str = "أيقونات وأسامي";
    pub const NAV_SLIM: &str = "أيقونات بس";
    // The pane labels are read by desk::Panes / ListDetail, which feature
    // screens opt into one by one.
    #[allow(dead_code)]
    pub const SIDE_WAITING: &str = "الشرح وصفحة الكتاب هيظهروا هنا أول ما تجاوب.";
    #[allow(dead_code)]
    pub const SIDE_LABEL: &str = "الشرح";
    #[allow(dead_code)]
    pub const LIST_LABEL: &str = "القايمة";
}

pub mod hover {
    pub const TITLE: &str = "اللمحة";
    pub const PEN: &str = "لمحة بقلم S Pen";
    pub const PEN_NOTE: &str = "قرّب القلم من اختصار أو اسم دوا من غير ما تلمس الشاشة، هيظهرلك معناه.";
    pub const MOUSE: &str = "لمحة بالماوس";
    pub const MOUSE_NOTE: &str =
        "سيب الماوس ثانية على الكلمة، أو على رقم صفحة الكتاب، أو على اختيار بعد ما تجاوب.";
    pub const UNVERIFIED: &str = "لسه ما اتراجعش";
    pub const TERMS: &str = "مصطلح في القاموس";
}

pub mod gopage {
    pub const TITLE: &str = "روح لصفحة في First Aid";
    pub const TOOL: &str = "روح لصفحة";
    pub const HINT: &str = "اكتب رقم الصفحة المطبوع في الكتاب (أو مدى زي 300–305).";
    pub const PLACEHOLDER: &str = "مثلاً 300";
    pub const GO: &str = "افتح";
    pub const INVALID: &str = "اكتب رقم صفحة بس.";
    pub fn outside(page: u32) -> String {
        format!("صفحة {page} مش جوه أي محاضرة في Steps (المحاضرات من Section II و III).")
    }
    pub fn nearest(page: u32) -> String {
        format!("افتح أقرب صفحة: {page}")
    }
}

pub mod catchup {
    use super::count::{REVIEW, count};

    pub const OFFER_TITLE: &str = "نلحق المراجعات على كام يوم؟";
    pub const RUNNING_TITLE: &str = "خطة اللحاق شغالة";
    pub const DAYS_LABEL: &str = "نوزّعها على";
    pub const PAUSE_NEW: &str = "وقّف الجديد لحد ما نلحق";
    pub const START: &str = "يلا نبدأ";
    pub const STOP: &str = "وقّف الخطة";
    pub const TODAY_DONE: &str = "خلصت نصيب النهارده. اللي فاضل مستنيك بكرة.";
    pub const NEW_PAUSED: &str = "الجديد واقف لحد ما الخطة تخلص.";
    /// "اتجمّع 120 مراجعة، حوالي 45 دقيقة. عادي، بيحصل."
    pub fn offer_line(due: usize, minutes: u32) -> String {
        format!(
            "اتجمّع عندك {}، حوالي {minutes} دقيقة. عادي، بيحصل، ونقسمهم على كام يوم.",
            count(due, &REVIEW)
        )
    }
    pub fn days(n: u32) -> String {
        format!("{n} أيام")
    }
    pub fn per_day(minutes: u32) -> String {
        format!("يعني حوالي {minutes} دقيقة مراجعة في اليوم، الأقدم الأول.")
    }
    pub fn running_line(day: i64, days: u32) -> String {
        format!("اليوم {day} من {days}.")
    }
    pub fn today_left(left: usize, minutes: u32) -> String {
        format!("فاضل النهارده {}، حوالي {minutes} دقيقة.", count(left, &REVIEW))
    }
}

pub mod keys {
    pub const TITLE: &str = "اختصارات الكيبورد";
    pub const OPEN: &str = "اعرض الاختصارات";
    pub const CLOSE: &str = "اقفل";
    pub const NOTE: &str = "بتشتغل على كيبورد التابلت والماك. على الماك استخدم ⌘ بدل Ctrl.";
    pub const SCREENS: &str = "تروح لشاشة";
    pub const SETTINGS: &str = "الإعدادات";
    pub const REVEAL: &str = "تكشف الإجابة";
    pub const RATE: &str = "تقيّم نفسك (من نسيت لسهل)";
    pub const OPTIONS: &str = "تختار إجابة في سؤال الاختيارات";
    pub const NEXT: &str = "اللي بعده";
    pub const MOVE: &str = "السؤال اللي بعده أو اللي قبله في الامتحان";
    pub const FLAG: &str = "تعلّم السؤال عشان ترجعله";
    pub const ESC: &str = "تقفل أي حاجة مفتوحة";
    pub const HELP: &str = "القايمة دي";
    pub const PAGE: &str = "تروح لصفحة في First Aid";
}

pub mod verify {
    pub const GROUP: &str = "مراجعة صحة المحتوى";
    pub const QUESTION: &str = "المحتوى ده مظبوط؟";
    pub const RIGHT: &str = "✓ صح";
    pub const EDIT: &str = "✎ عدّل";
    pub const WRONG: &str = "✗ غلط";
    pub const UNDO: &str = "رجوع";
    pub const CANCEL: &str = "إلغاء";
    pub const SAVE: &str = "احفظ تعديلي";
    pub const SAVED_APPROVED: &str = "✓ اتسجّل إنه صح، ومش هيسألك عليه تاني.";
    pub const SAVED_EDITED: &str = "✎ اتسجّل تعديلك، وهتذاكره بكلامك من هنا ورايح.";
    pub const SAVED_WRONG: &str = "✗ اتشال من المراجعة لحد ما يتصلّح.";
    pub const NOT_SAVED: &str = "ما اتحفظش، واختيارك لسه زي ما هو. اضغط «حاول تاني».";
    pub const WRONG_NOTE: &str = "إيه الغلط؟ (اختياري)";
    pub const WRONG_PLACEHOLDER: &str = "مثلاً: الكتاب صفحة 14 بيقول 7–9 نقط";
    pub const WRONG_CONFIRM: &str = "شيله من المراجعة";
    pub const EDIT_NOTE: &str = "ملاحظة للي هيصلّح المحتوى (اختياري)";
    pub const NOTHING_CHANGED: &str = "ما غيّرتش حاجة لسه. عدّل حقل أو اكتب ملاحظة.";
    pub const F_FRONT: &str = "الوش";
    pub const F_BACK: &str = "الضهر";
    pub const F_TEXT: &str = "النص بالفراغات";
    pub const F_EXTRA: &str = "معلومة زيادة";
    pub const F_STEM: &str = "السؤال";
    pub const F_OPTIONS: &str = "الاختيارات، كل واحد في سطر";
    pub const F_ANSWER: &str = "الإجابة الصح";
    pub const F_WHY: &str = "الشرح، سطر لكل اختيار بنفس الترتيب";
    pub const F_PROMPT: &str = "السؤال";
    pub const F_POINTS: &str = "النقط، كل نقطة في سطر";
    pub const F_STEPS: &str = "الخطوات بالترتيب، كل خطوة في سطر";
    pub const F_TITLE: &str = "العنوان";
    pub const F_QUESTION: &str = "المطلوب";
    pub const F_EXPECT: &str = "النتيجة";
    pub const F_INTERPRET: &str = "التفسير";
    pub const EXPORT_TITLE: &str = "صدّر مراجعاتك للمحتوى";
    pub const EXPORT_BUTTON: &str = "صدّر الملف";
    pub const EXPORT_SAVED: &str = "اتحفظ في:";
    pub const EXPORT_SHOW: &str = "اعرض النص (تقدر تنسخه)";
    pub const EXPORT_FAILED: &str = "ما عرفتش أحفظ الملف:";
}

pub mod own {
    pub const TITLE: &str = "كروتك";
    pub const NEW: &str = "＋ كارت جديد";
    pub fn cards(n: usize) -> String {
        super::count::count(n, &super::count::CARD)
    }
    pub const NONE_YET: &str = "لسه مفيش كروت.";
    pub const EDIT: &str = "عدّل";
    pub const DELETE: &str = "امسح";
    pub const DELETE_SURE: &str = "أمسحه من كل أجهزتك؟";
    pub const DELETE_YES: &str = "أيوه امسحه";
    pub const STUDY_THEM: &str = "ذاكر كروتك بس";
    pub const FORM_NEW: &str = "كارت جديد";
    pub const FORM_EDIT: &str = "عدّل الكارت";
    pub const FORM_SHARED: &str = "كارت من نص شيّرته";
    pub const SHARED_HINT: &str = "النص اتحط في الضهر. اكتب السؤال في الوش.";
    pub const SHARED_TRIMMED: &str = "النص كان أطول من 3000 حرف، فآخره اتشال. لو محتاجه، اعمل منه كارت تاني.";
    pub const SHARE_WAITING: &str = "في نص تاني شيّرته مستني، هيفتح أول ما تخلّص الكارت ده.";
    pub const FRONT: &str = "الوش (السؤال)";
    pub const BACK: &str = "الضهر (الإجابة)";
    pub const LECTURE: &str = "المحاضرة";
    pub const NO_LECTURE: &str = "من غير محاضرة";
    pub const PAGE: &str = "صفحة الكتاب";
    pub const SOURCE: &str = "منين";
    pub const SOURCE_HINT: &str = "مثلاً: Pathoma أو UWorld أو اسم الموقع";
    pub const SAVE: &str = "احفظ";
    pub const CANCEL: &str = "إلغاء";
    pub const FAILED: &str = "ما عرفتش أحفظ الكارت:";
    pub const DELETE_FAILED: &str = "ما عرفتش أمسح الكارت، ولسه موجود:";
    pub const E_FRONT: &str = "اكتب السؤال في الوش.";
    pub const E_BACK: &str = "اكتب الإجابة في الضهر.";
    pub const E_FRONT_LONG: &str = "الوش طويل، خليه سؤال قصير (600 حرف بالكتير).";
    pub const E_BACK_LONG: &str = "الضهر طويل، قسّمه على كذا كارت (3000 حرف بالكتير).";
    pub const E_SOURCE_LONG: &str = "«منين» طويلة، خليها اسم قصير (200 حرف بالكتير).";
    pub const FROM: &str = "من";
    pub const FRONT_HINT: &str = "سؤال واحد يختبر فكرة واحدة…";
    pub const BACK_HINT: &str = "الإجابة الأساسية، وبعدها شرح قصير لو محتاج…";
    pub const PREVIEW: &str = "عاين الكارت";
}

pub mod data {
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
}

pub mod cloud {
    pub const SECTION: &str = "المزامنة";
    pub const SECTION_SUB: &str = "حسابك وأجهزتك";
    pub const TITLE: &str = "المزامنة بين أجهزتك";
    pub const NOT_CONFIGURED: &str = "المزامنة لسه ما اتفعّلتش في النسخة دي. مذاكرتك محفوظة على الجهاز، وتقدر تنقلها بنسخة احتياطية من «بياناتك».";
    pub const SIGN_IN: &str = "سجّل دخول بجوجل";
    pub const SIGNING_IN: &str = "كمّل تسجيل الدخول في المتصفح، وبعدين ارجع هنا.";
    /// The browser page after Google sends him back (HTML).
    pub const BROWSER_DONE: &str = "<h1>✓</h1><p>تمام، ارجع لـ Steps.</p>";
    pub const BROWSER_CANCELLED: &str = "<p>ما اتسجلتش. ارجع لـ Steps وجرّب تاني.</p>";
    /// The browser page's button back to the app (Android).
    pub const BROWSER_BACK: &str = "ارجع لـ Steps";
    /// Google said yes; Steps is finishing with the server.
    pub const FINISHING: &str = "بيكمّل تسجيل الدخول…";
    /// "من" in "من 3 دقايق".
    pub const AGO: &str = "من";
    pub const SIGNED_IN_AS: &str = "داخل بـ";
    pub const SIGN_OUT: &str = "اخرج من الجهاز ده";
    pub const CONNECTING: &str = "بيتصل";
    pub const CATCHING_UP: &str = "بيجيب اللي فاتك";
    pub const LIVE: &str = "متزامن";
    pub const OFFLINE: &str = "مش متصل، هيحاول تاني لوحده";
    pub const STOPPED: &str = "واقف";
    pub const PENDING: &str = "تغيير مستني يترفع";
    /// "3 تغييرات مستنية تترفع".
    pub fn pending(n: u64) -> String {
        match n {
            1 => "تغيير واحد مستني يترفع".to_string(),
            2 => "تغييرين مستنيين يترفعوا".to_string(),
            _ if (3..=10).contains(&(n % 100)) => format!("{n} تغييرات مستنية تترفع"),
            _ => format!("{n} {PENDING}"),
        }
    }
    pub const LAST: &str = "آخر مزامنة";
    pub const JUST_NOW: &str = "دلوقتي";
    pub const DEVICES: &str = "أجهزتك";
    pub const REFRESH: &str = "حدّث";
    pub const THIS_DEVICE: &str = "الجهاز ده";
    pub const DEVICE_OUT: &str = "خرج";
    pub const NEVER_SYNCED: &str = "لسه ما اتزامنش";
    pub const SIGN_OUT_DEVICE: &str = "خرّجه";
    pub const LOADING: &str = "بيحمّل…";

    pub const P_NOT_CONFIGURED: &str = "تسجيل الدخول لسه ما اتظبطش في النسخة دي.";
    pub const P_NOT_ALLOWED: &str = "الحساب ده مش مسموح له. ادخل بالجيميل اللي متسجل في السيرفر.";
    pub const P_CANCELLED: &str = "تسجيل الدخول اتلغى.";
    pub const P_FAILED: &str = "تسجيل الدخول ما نجحش. جرّب تاني.";
    pub const P_REMOTE: &str = "الجهاز ده اتخرّج من جهاز تاني. سجّل دخول تاني وقت ما تحب.";
    pub const P_NETWORK: &str = "مش قادر يوصل للسيرفر دلوقتي. هيحاول تاني لوحده، ومذاكرتك محفوظة على الجهاز.";
    pub const P_SIGN_IN_WAITING: &str = "جوجل وافقت، بس النت مش واصل لـ Steps دلوقتي. هيكمّل الدخول لوحده أول ما النت يرجع، أو دوس «جرّب دلوقتي».";
    pub const TRY_NOW: &str = "جرّب دلوقتي";
    pub const P_SERVER: &str = "السيرفر فيه مشكلة دلوقتي. هيحاول تاني لوحده.";
    pub const P_OUTDATED: &str = "السيرفر أحدث من النسخة دي. حدّث Steps.";
    pub const P_CLOCK: &str =
        "ساعة الجهاز سابقة الوقت الحقيقي، فالتغييرات الجديدة مستنية. ظبط الساعة على التلقائي وهتتبعت لوحدها.";

    // Username + password (docs/CLOUD.md §4.1).
    pub const PW_TITLE: &str = "الدخول باسم مستخدم وباسورد";
    pub const PW_INTRO: &str = "على جهاز مش بتاعك ومن غير جيميلك: ادخل باسم المستخدم والباسورد اللي ظبطتهم من جهازك، وهتلاقي مذاكرتك كلها وآخر حتة وقفت عندها.";
    pub const USERNAME: &str = "اسم المستخدم";
    pub const PASSWORD: &str = "الباسورد";
    pub const NOT_MINE: &str = "الجهاز ده مش بتاعي";
    pub const NOT_MINE_HINT: &str = "هتخرج لوحدك أول ما تقفل التاب أو التطبيق.";
    pub const PW_SIGN_IN: &str = "ادخل";
    pub const PW_CHECKING: &str = "بيدخل…";
    pub const PW_YOURS: &str = "بتدخل من أي جهاز بـ";
    pub const PW_HOW: &str = "على أي جهاز: افتح Steps، ومن «المزامنة» اكتب اسم المستخدم والباسورد.";
    pub const PW_NONE: &str =
        "اختار اسم مستخدم وباسورد عشان تدخل من أي جهاز من غير جيميل. حسابك هو هو، فكل حاجة بتتزامن.";
    pub const USERNAME_HINT: &str = "من ٣ لـ ٣٢: حروف إنجليزي صغيرة وأرقام و . _ -";
    pub const NEW_PASSWORD: &str = "الباسورد الجديد";
    pub const PASSWORD_HINT: &str = "١٠ حروف على الأقل. جملة قصيرة تفتكرها أحسن من كلمة صعبة.";
    pub const PASSWORD_AGAIN: &str = "اكتبه تاني";
    pub const PW_SAVE: &str = "احفظ";
    pub const PW_SAVING: &str = "بيحفظ…";
    pub const PW_SAVED: &str = "اتحفظ. تقدر تدخل بيه من أي جهاز دلوقتي.";
    pub const PW_CHANGE: &str = "غيّر";
    pub const PW_REMOVE: &str = "اقفل الدخول بالباسورد";
    pub const CANCEL: &str = "إلغاء";
    pub const TEMPORARY: &str =
        "دخول مؤقت: هتخرج لوحدك لما تقفل التاب أو التطبيق. لو خلصت قبلها، دوس «اخرج من الجهاز ده».";
    pub const P_WRONG_PASSWORD: &str = "اسم المستخدم أو الباسورد مش مظبوط. جرّب تاني.";
    pub const P_LOCKED: &str = "محاولات غلط كتير. استنى شوية وجرّب تاني.";
    /// "استنى 5 دقايق": the lock's wait, with the Egyptian number forms.
    pub fn locked(minutes: u64) -> String {
        use crate::i18n::count::{MINUTE, count};
        format!("محاولات غلط كتير، فالدخول بالاسم ده واقف شوية. جرّب تاني بعد {}.", count(minutes, &MINUTE))
    }
    pub const P_TAKEN: &str = "اسم المستخدم ده محجوز. اختار اسم تاني.";
    pub const P_BAD_USERNAME: &str = "اسم المستخدم من ٣ لـ ٣٢: حروف إنجليزي وأرقام و . _ - بس.";
    pub const P_SHORT_PASSWORD: &str = "الباسورد لازم يبقى ١٠ حروف على الأقل.";
    pub const P_MISMATCH: &str = "الباسوردين مش زي بعض.";
}

pub mod settings {
    pub const TITLE: &str = "الإعدادات";
    pub const CLOSE: &str = "اقفل";
    pub const BACK: &str = "رجوع";
    pub const SECTIONS: &str = "أقسام الإعدادات";
    pub const SEARCH: &str = "دوّر في الإعدادات";
    pub const NO_RESULTS: &str = "مفيش قسم مناسب. جرّب كلمة تانية زي الخط أو التذكيرات.";
    pub const CLEAR_SEARCH: &str = "امسح البحث";
    pub const GROUP_STUDY: &str = "المذاكرة والتذكير";
    pub const GROUP_DEVICE: &str = "الشكل والجهاز";
    pub const GROUP_DATA: &str = "الحساب والبيانات";
    pub const THIS_DEVICE: &str = "للجهاز ده بس";

    pub const LOOK: &str = "الشكل";
    pub const LOOK_SUB: &str = "الألوان، نمط الواجهة وحجمها";
    pub const DATA: &str = "بياناتك";
    pub const DATA_SUB: &str = "فين بتتحفظ، والتصدير";
    pub const ABOUT: &str = "عن Steps";
    pub const ABOUT_SUB: &str = "النسخة والمحتوى";

    pub const THEME: &str = "الألوان";
    pub const THEME_SYSTEM: &str = "زي الجهاز";
    pub const THEME_SYSTEM_SUB: &str = "فاتح بالنهار، وغامق لما الجهاز يبقى غامق";
    pub const THEME_PAPER: &str = "ورق";
    pub const THEME_PAPER_SUB: &str = "فاتح وهادي";
    pub const THEME_NIGHT: &str = "ليل";
    pub const THEME_NIGHT_SUB: &str = "غامق";
    pub const THEME_SYSTEM_BLACK: &str = "زي الجهاز، وبالليل أسود";
    pub const THEME_SYSTEM_BLACK_SUB: &str = "فاتح بالنهار، وأسود خالص لما الجهاز يبقى غامق";
    pub const THEME_BLACK: &str = "أسود AMOLED";
    pub const THEME_BLACK_SUB: &str = "أسود خالص، فبكسلات شاشة الأموليد بتطفي: أريح للعين بالليل وبتوفّر بطارية";
    pub const THEME_SHIFT: &str = "نبطشية";
    pub const THEME_SHIFT_SUB: &str = "غامق ودافي من غير ضوء أزرق، للمذاكرة متأخر قبل النوم";

    pub const APPEARANCE: &str = "نمط الواجهة";
    pub const APPEARANCE_CALM: &str = "هادئ";
    pub const APPEARANCE_CALM_SUB: &str = "أسطح سادة وهادية، من غير لمعة";
    pub const APPEARANCE_GLOSSY: &str = "Glossy · Material You";
    pub const APPEARANCE_GLOSSY_SUB: &str =
        "أسطح ملونة فيها لمعة خفيفة وأزرار مدوّرة. ألوانها من ألوان Steps، مش من خلفية الموبايل";
    pub const APPEARANCE_LIQUID_GLASS: &str = "Liquid Glass · iOS";
    pub const APPEARANCE_LIQUID_GLASS_SUB: &str =
        "خلفية وكروت زجاجية ناعمة وشريط تنقّل مطفي، مع الحفاظ على وضوح أسئلة المذاكرة";

    pub const TEXT_SIZE: &str = "حجم الواجهة";
    pub const TEXT_SMALLER: &str = "أصغر";
    pub const TEXT_SMALL: &str = "صغير";
    pub const TEXT_NORMAL: &str = "عادي";
    pub const TEXT_LARGE: &str = "كبير";
    pub const TEXT_LARGER: &str = "أكبر";
    pub const TEXT_SAMPLE_AR: &str = "إيه أول علاج لـ";
    pub const TEXT_SAMPLE_TERM: &str = "Stanford type A dissection";

    pub const DATA_FOLDER: &str = "فولدر البيانات";
    pub const DATA_NOWHERE: &str = "مش معروف على الجهاز ده";
    pub const DATA_IN_BROWSER: &str =
        "في المتصفح ده (IndexedDB). مسح بيانات الموقع بيمسحها، فخد نسخة احتياطية من فوق من وقت للتاني.";
    pub const DATA_GROUP_BACKUP: &str = "احفظ ورجّع";
    pub const DATA_GROUP_EXPORT: &str = "صدّر لبرّه";
    pub const DATA_GROUP_DEVICE: &str = "على الجهاز ده";

    pub const VERSION: &str = "النسخة";
    pub const CONTENT: &str = "المحتوى";
    pub const LECTURES_WRITTEN: &str = "المحاضرات المكتوبة:";
    pub const SOURCE: &str = "المصدر";
    pub const BOOK: &str = "First Aid for the USMLE Step 1 2026";

    pub const DIAG: &str = "اللي حصل من مشاكل";
    pub const DIAG_EMPTY: &str = "مفيش مشاكل لحد دلوقتي.";
    pub fn diag_lines(n: usize) -> String {
        super::count::count(n, &super::count::LINE)
    }
    pub const DIAG_REFRESH: &str = "حدّث";
    pub const DIAG_COPY: &str = "انسخ";
    pub const DIAG_COPIED: &str = "اتنسخ";
}

pub mod update {
    // Settings › Updates (crates/app/src/update/view.rs).
    pub const SECTION: &str = "التحديثات";
    pub const SECTION_SUB: &str = "نسخة جديدة ومحتوى جديد";
    pub const BUILD: &str = "بناء";
    /// "بناء 40 · محتوى 45" in the "what's new" sheet.
    pub const CONTENT_SHORT: &str = "محتوى";

    // Why updates are off here.
    pub const OFF_DEV: &str =
        "دي نسخة تطوير، فمش بتدوّر على تحديثات. النسخ اللي على الموبايل والتابلت والماك هي اللي بتتحدّث.";
    pub const OFF_KEY: &str = "التحديثات لسه مش شغالة: مفتاح التوقيع لسه ما اتعملش. أول نسخة ألفا هتيجي بيه.";
    pub const OFF_PLATFORM: &str = "مفيش تحديثات منشورة للجهاز ده.";
    pub const OFF_BUNDLE: &str = "Steps شغال من فولدر البناء، مش من Steps.app، فمش هيتحدّث لوحده.";

    // Where things stand.
    pub const CHECKING: &str = "بيدوّر على تحديث…";
    pub const CHECK_NOW: &str = "دوّر على تحديث";
    pub const UP_TO_DATE: &str = "معاك آخر نسخة";
    pub const NOT_FOR_DEVICE: &str = "فيه نسخة أحدث، بس لسه مش منشورة للجهاز ده.";
    pub const AVAILABLE: &str = "فيه نسخة جديدة";
    pub const DOWNLOAD: &str = "نزّلها";
    pub const DOWNLOADING: &str = "بينزّل";
    pub const READY: &str = "النسخة الجديدة جاهزة";
    pub const INSTALL_NOW: &str = "ثبّتها";
    pub const INSTALL_RESTART: &str = "ثبّتها وافتح تاني";
    pub const INSTALLING: &str = "بيثبّت…";
    pub const INSTALLING_ANDROID: &str = "أندرويد ممكن يسألك توافق. مذاكرتك محفوظة ومش هتضيع.";
    pub const NEEDS_PERMISSION: &str = "أندرويد محتاج إذن \"تثبيت تطبيقات غير معروفة\" لـ Steps";
    pub const OPEN_PERMISSION: &str = "افتح الإذن";
    pub const TRY_AGAIN: &str = "جرّب تاني";
    pub const INSTALLED: &str = "اتثبّتت";
    pub const RESTART_NOW: &str = "افتح Steps تاني";
    pub const NEXT_LAUNCH: &str = "النسخة الجديدة هتشتغل أول ما تفتح Steps تاني.";
    pub const FAILED_NETWORK: &str = "مقدرش يوصل للتحديثات. هيجرّب تاني بعدين.";
    pub const FAILED_VERIFY: &str = "التحديث ما عداش من الفحص، فما اتثبّتش. مفيش حاجة اتغيّرت.";
    pub const FAILED_INSTALL: &str = "التثبيت ما كملش. النسخة اللي معاك شغالة زي ما هي.";
    pub const TOO_NEW: &str = "التحديث ده محتاج نسخة أحدث من Steps. ثبّتها بإيدك مرة واحدة.";
    pub const KEY_CHANGED: &str = "مفتاح نسخ التجربة اتغيّر، فالنسخة الجديدة مش هتتركّب فوق دي.";
    /// What to do about it, once, without losing anything.
    pub const KEY_CHANGED_STEPS: [&str; 4] = [
        "احفظ نسخة من الإعدادات › بياناتك، واختار «احفظها في Downloads أو Drive».",
        "امسح Steps من الموبايل.",
        "نزّل آخر نسخة تجربة من الزرار ده وركّبها.",
        "افتحها واسترجع النسخة من الإعدادات › بياناتك.",
    ];
    pub const GET_PREVIEW: &str = "نزّل آخر نسخة تجربة";
    pub const LATER: &str = "بعدين";
    pub const WHATS_NEW: &str = "الجديد";

    // Settings › Updates: what is on this device, and one check for both.
    pub const GROUP_HAVE: &str = "اللي على جهازك";
    pub const ROW_APP: &str = "التطبيق";
    pub const ROW_CONTENT: &str = "الدروس والأسئلة والشكل";
    /// Before "من يومين": when the content in use was published.
    pub const CONTENT_PUBLISHED: &str = "اتنشر";
    pub const GROUP_CHECK: &str = "البحث عن تحديث";
    pub const LAST_CHECK: &str = "آخر مرة دوّرنا";
    /// The Settings list's Updates line.
    pub const SUMMARY_CONTENT_WAITING: &str = "دروس جديدة نزلت وهتفتح لوحدها";
    pub const SUMMARY_FAILED: &str = "آخر بحث ما وصلش، هيجرّب تاني";
    pub const NEVER_CHECKED: &str = "لسه ما دوّرناش على الجهاز ده";
    /// The two halves of a check's result.
    pub const RESULT_APP: &str = "التطبيق";
    pub const RESULT_CONTENT: &str = "الدروس والأسئلة";
    pub const CONTENT_CURRENT: &str = "معاك آخر محتوى";
    pub const CONTENT_NEW: &str = "فيه محتوى جديد نزل، وهيفتح لوحده أول ما تخلّص اللي بتذاكره";
    pub const CONTENT_FAILED: &str = "مقدرش يجيب المحتوى الجديد دلوقتي، هيجرّب تاني بعدين";

    // Mode and channel.
    pub const MODE: &str = "إزاي يتحدّث";
    pub const MODE_AUTO: &str = "لوحده";
    pub const MODE_AUTO_SUB: &str = "ينزّل على الواي فاي ويثبّت في وقت هادي، عمره ما يقطع مذاكرة.";
    pub const MODE_NOTIFY: &str = "قولّي الأول";
    pub const MODE_NOTIFY_SUB: &str = "يقولك إن فيه نسخة جديدة، وإنت تختار امتى.";
    pub const MODE_OFF: &str = "لأ";
    pub const MODE_OFF_SUB: &str = "بس لما تدوس \"دوّر على تحديث\".";
    pub const CHANNEL: &str = "القناة";
    pub const ALPHA: &str = "ألفا";
    pub const BETA: &str = "بيتا";
    pub const STABLE: &str = "ثابتة";
    pub const PREVIEW: &str = "تجربة";
    pub const PREVIEW_CHANNEL_NOTE: &str = "نسخة التجربة بتنزل مع كل تعديل، فممكن تلاقي فيها حاجة لسه مش مظبوطة. على الماك هيوصلك منها المحتوى بس. ترجع للألفا وقت ما تحب.";
    pub const MB: &str = "ميجا";

    // The notifications Android shows around an install.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub const DONE_TITLE: &str = "Steps اتحدّث";
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub const DONE_BODY: &str = "افتحه وكمّل من مكانك.";
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub const READY_TITLE: &str = "تحديث Steps جاهز: دوس عشان تثبّته";
    /// The Android channel those two notifications go to.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub const INSTALL_CHANNEL: &str = "تحديثات Steps";
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub const INSTALL_CHANNEL_SUB: &str = "تحديث جاهز يتثبّت، و Steps اتحدّث";
    // The "new version" notification, and its switch.
    pub const NOTICE_AVAILABLE: &str = "نزلت نسخة جديدة من Steps";
    pub const NOTICE_READY: &str = "نسخة Steps الجديدة جاهزة تتثبّت";
    pub const NOTICE_INSTALLED: &str = "نسخة Steps الجديدة اتحطّت";
    pub const NOTICE_TAP_AVAILABLE: &str = "دوس تشوف الجديد وتنزّلها.";
    pub const NOTICE_TAP_READY: &str = "اتنزّلت واتفحصت. دوس عشان تثبّتها.";
    pub const NOTICE_CHANNEL: &str = "نسخ Steps الجديدة";
    pub const NOTICE_CHANNEL_SUB: &str = "نسخة جديدة نزلت، وإيه الجديد فيها";
    pub const NOTIFY: &str = "ابعتلي إشعار أول ما تنزل نسخة جديدة";
    /// "وحاجتين كمان", "و5 حاجات كمان".
    pub fn notice_more(n: usize) -> String {
        format!("و{} كمان", super::count::count(n, &super::count::THING))
    }

    // The "what's new" sheet (crates/app/src/update/sheet.rs).
    pub const WHATS_NEW_ASK: &str = "إيه الجديد؟";
    pub const HISTORY: &str = "سجل التغييرات";
    pub const HISTORY_EMPTY: &str = "لسه مفيش تغييرات متسجّلة. هتظهر هنا مع أول تحديث.";
    pub const UPDATED_TITLE: &str = "Steps اتحدّثت";
    /// A content-only update: the app itself didn't change (#336).
    pub const CONTENT_UPDATED_TITLE: &str = "تحديث محتوى";
    /// The quiet card on Today after a content-only update.
    pub const CONTENT_NOTE: &str = "الدروس اتحدّثت";
    pub const KIND_NEW: &str = "جديد";
    pub const KIND_IMPROVED: &str = "اتحسّن";
    pub const KIND_FIXED: &str = "اتصلّح";
    pub const BADGE_LATEST: &str = "الأحدث";
    pub const BADGE_INSTALLED: &str = "اللي معاك";
    /// The content-only update in use, next to the app build he runs (#349).
    pub const BADGE_CONTENT_IN_USE: &str = "المحتوى اللي معاك";
    pub const BADGE_NEW: &str = "جديد عليك";
    /// A content-only update: lectures, lessons or the look, no install.
    pub const BADGE_CONTENT: &str = "تحديث محتوى";
    pub const TODAY: &str = "النهارده";
    pub const YESTERDAY: &str = "امبارح";
    pub const TWO_DAYS_AGO: &str = "من يومين";
    pub const DAYS_AGO: &str = "من {n} أيام";
    pub const SHOW_ALL: &str = "اعرض الكل ({n})";
    pub const SHOW_LESS: &str = "اعرض أقل";
    pub const EARLIER: &str = "نسخ قبل كده";
    pub const SHOW_OLDER: &str = "اعرض الأقدم";
    pub const CLOSE: &str = "اقفل";
    pub const GOT_IT: &str = "تمام";
    pub const NO_DETAILS: &str = "النسخة دي مفيهاش تفاصيل مكتوبة.";
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

    /// "3 new builds", with Arabic's dual and plural.
    pub fn builds_count(n: usize) -> String {
        match n {
            1 => "نسخة جديدة".to_owned(),
            2 => "نسختين جداد".to_owned(),
            3..=10 => format!("{n} نسخ جديدة"),
            _ => format!("{n} نسخة جديدة"),
        }
    }

    /// The same count for content-only updates: no new app came (#349).
    pub fn content_updates_count(n: usize) -> String {
        match n {
            1 => "تحديث محتوى".to_owned(),
            2 => "تحديثين محتوى".to_owned(),
            3..=10 => format!("{n} تحديثات محتوى"),
            _ => format!("{n} تحديث محتوى"),
        }
    }
}

pub mod sound {
    pub const TITLE: &str = "الأصوات والاهتزاز";
    pub const SOUNDS: &str = "الأصوات";
    pub const ON: &str = "شغّالة";
    pub const OFF: &str = "مقفولة";
    pub const RIGHT: &str = "صوت الإجابة الصح";
    pub const END: &str = "نغمة آخر الجلسة";
    pub const VARIATION: &str = "صوت";
    pub const VIBRATE: &str = "الاهتزاز";
}

pub mod remind {
    pub const TITLE: &str = "Steps";
    pub const SECTION: &str = "التذكير";
    pub const SECTION_SUB: &str = "تذكير هادي بمراجعة اليوم";
    /// Android's notification channel, as Settings › Notifications lists it.
    pub const CHANNEL: &str = "تذكير المذاكرة";
    pub const CHANNEL_ABOUT: &str = "تذكير هادي بمراجعة اليوم";
    pub const PHONE_ONLY: &str =
        "التذكير بيرن من تطبيق Steps المتثبّت على الموبايل أو الماك، مش من نسخة التطوير.";
    pub const MAC_HINT: &str =
        "على الماك الإشعار بيظهر حتى لو Steps مقفولة. لو ما ظهرش، اسمح لـ Steps من إعدادات النظام › الإشعارات.";
    pub const DAILY: &str = "تذكير المذاكرة اليومي";
    pub const ON: &str = "شغّال";
    pub const OFF: &str = "مقفول";
    pub const TIMES: &str = "المواعيد";
    pub const TIME: &str = "معاد التذكير";
    pub const ADD: &str = "زوّد معاد";
    pub const REMOVE: &str = "شيل";
    pub const HERE: &str = "يرن على الجهاز ده";
    pub const BLOCKED: &str = "الإشعارات مقفولة لـ Steps، فالتذكير مش هيظهر.";
    pub const ALLOW: &str = "اسمح بالإشعارات";
    pub const OPEN_NOTIFICATIONS: &str = "افتح إعدادات الإشعارات";
    pub const BATTERY: &str =
        "لو التذكير ما رنّش، خلّي البطارية لـ Steps «من غير قيود» عشان الموبايل ما ينيّمهاش.";
    pub const OPEN_APP: &str = "افتح إعدادات التطبيق";

    /// The reminder line, with the reviews due that day when known.
    pub fn text(due: Option<u32>) -> String {
        match due {
            Some(0) | None => "وقت جلسة قصيرة. كارت واحد أحسن من مفيش.".into(),
            Some(1) => "عندك مراجعة واحدة النهارده. دقيقة وتخلص.".into(),
            Some(n) => format!(
                "عندك {} النهارده. يلا نبدأ بجلسة قصيرة.",
                super::count::count(n, &super::count::REVIEW)
            ),
        }
    }
}

pub mod onboard {
    pub const TITLE: &str = "أهلاً في Steps";
    pub const BACK: &str = "رجوع";
    pub const PICK_FUTURE: &str = "اختار يوم بعد النهارده.";
    pub const PICK_FAR: &str = "اختار يوم في خلال 3 سنين من النهارده.";
    pub const DAYS_TITLE: &str = "بتذاكر أنهي أيام في الأسبوع؟";
    pub const DAYS_BODY: &str = "الخطة والتذكير هيمشوا على الأيام دي بس. أيام الراحة راحة.";
    pub const PICK_A_DAY: &str = "اختار يوم واحد على الأقل.";
    pub const MINUTES_TITLE: &str = "قد إيه في اليوم؟";
    pub const STUDIED_TITLE: &str = "ذاكرت أنهي محاضرات قبل كده؟";
    pub const STUDIED_BODY: &str = "علّم اللي قريته قبل كده. لو مفيش، كمّل عادي.";
    pub const ALL: &str = "كلهم";
    pub const NONE: &str = "ولا واحدة";
    pub const UP_TO_HERE: &str = "لحد هنا";
    pub const REMIND_TITLE: &str = "أفكّرك إمتى؟";
    pub const REMIND_BODY: &str = "إشعار هادي فيه عدد المراجعات المستنياك. بيرن على الموبايل بس.";
    pub const ADD_TIME: &str = "زوّد معاد";
    pub const REMOVE: &str = "شيل";
    pub const NO_REMINDERS: &str = "من غير تذكير. تقدر تشغّله بعدين من الإعدادات.";
    pub const THEME_TITLE: &str = "تحب الألوان إزاي؟";
    pub const THEME_BODY: &str = "«نبطشية» غامق ودافي، للمذاكرة بالليل.";
    pub const REOPEN_TITLE: &str = "أسئلة البداية";
    pub const REOPEN: &str = "افتحها تاني";

    // The 2026-10-29 redesign: three quick screens, details optional.
    pub fn progress(n: usize, of: usize) -> String {
        format!("البداية · {n} من {of}")
    }
    pub const LATER: &str = "بعد كده";
    pub const CONTINUE: &str = "كمّل";
    pub const HAVE_ACCOUNT: &str = "بتذاكر على Steps قبل كده؟ ادخل على حسابك";
    pub const WELCOME_TITLE: &str = "مذاكرة تناسب يومك.";
    pub const WELCOME_BODY: &str = "درس قصير، محاولة تفتكر، ومراجعة تثبّت الفهم.";
    pub const WELCOME_NOTE: &str =
        "الفهم والتذكّر وتدريب الأسئلة، كل واحد له مكانه. والكتاب هو المرجع في الامتحان.";
    pub const TIME_TITLE: &str = "خلّي البداية على قد وقتك.";
    pub const TIME_BODY: &str = "كل جلسة صغيرة وتخلص. تقدر تغيّر ده بعدين.";
    pub const EXAM_OPTIONAL: &str = "يوم الامتحان (لو تعرفه)";
    pub const EXAM_HINT: &str = "بيوزّع المحاضرات والمراجعة على الأيام اللي فاضلة.";
    pub const READY_TITLE: &str = "كده جاهز تبدأ.";
    pub const READY_BODY: &str = "اختياراتك تتعدّل في أي وقت. ودي حاجات اختيارية لو حابب تظبطها دلوقتي:";
    pub const START_LESSON: &str = "ابدأ أول درس";
    pub const START_TODAY: &str = "افتح النهارده";
    pub const DETAIL_DONE: &str = "تمام";
    pub const DAYS_ROW: &str = "أيام المذاكرة";
    pub fn days_sub(n: usize) -> String {
        match n {
            7 => "كل أيام الأسبوع".to_string(),
            1 => "يوم واحد في الأسبوع".to_string(),
            2 => "يومين في الأسبوع".to_string(),
            n => format!("{n} أيام في الأسبوع"),
        }
    }
    pub const STUDIED_ROW: &str = "اللي ذاكرته قبل كده";
    pub fn studied_sub(n: usize) -> String {
        match n {
            0 => "ولا محاضرة لسه".to_string(),
            n => super::count::count(n, &super::count::LECTURE),
        }
    }
    pub const REMIND_ROW: &str = "التذكير";
    pub const REMIND_OFF: &str = "من غير تذكير";
    pub const THEME_ROW: &str = "الألوان";
}

pub mod recap {
    pub const OPEN_PROGRESS: &str = "افتح تقدّمي";
    /// Between list items and clauses.
    pub const COMMA: &str = "، ";
    use super::count::{DAY, NEW_THING, QUESTION, REVIEW, count};

    pub const DONE_REST: &str = "مراجعة النهارده خلصت، استريح.";
    pub const WEEK_TITLE: &str = "آخر 7 أيام";
    pub const WEEK_RETENTION: &str = "بتفتكر صح:";
    pub const WEEK_WEAK: &str = "يستاهلوا رجوع:";

    /// "النهارده جاوبت 12 سؤال".
    pub fn answered(n: usize) -> String {
        format!("النهارده جاوبت {}", count(n, &QUESTION))
    }

    /// "واتعلمت حاجتين جداد".
    pub fn learned(n: u32) -> String {
        format!("واتعلمت {}", count(n, &NEW_THING))
    }

    pub fn done_rest(tomorrow: u32) -> String {
        match tomorrow {
            0 => DONE_REST.to_string(),
            n => format!("{DONE_REST} بكرة عندك {}.", count(n, &REVIEW)),
        }
    }

    pub fn left(n: usize) -> String {
        format!("فاضل {} لو عندك وقت، ولو لأ بكرة موجودة.", count(n, &REVIEW))
    }

    /// "اتعلمت 10 حاجات جديدة في 3 أيام."
    pub fn week_learned(learned: u32, days: usize) -> String {
        format!("اتعلمت {} في {}.", count(learned, &NEW_THING), count(days, &DAY))
    }

    pub fn exam_in(days: i32) -> String {
        match days {
            ..=0 => "الامتحان النهارده. بالتوفيق.".to_string(),
            n => format!("فاضل على الامتحان {}.", count(n, &DAY)),
        }
    }
}

pub mod learn {
    pub const NONE_YET: &str = "الدروس لسه بتتكتب. أول ما درس يجهز هتلاقيه هنا.";
    pub const OF: &str = "من";
    pub const START_FIRST: &str = "ابدأ أول درس";
    pub const CONTINUE: &str = "كمّل";
    pub const ALL_LESSONS_DONE: &str = "خلّصت كل الدروس المكتوبة. عاش!";
    pub const UNVERIFIED_NOTE: &str =
        "الدروس لسه ما اتراجعتش: الكتاب هو المرجع، ولو لقيت حاجة مش مظبوطة قولّي.";
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
}

/// The view switcher on list screens (parts::ViewBar).
pub mod views {
    pub const LAYOUT_TITLE: &str = "طريقة العرض";
    pub const CARDS: &str = "كروت";
    pub const LIST: &str = "قايمة";
    pub const COMPACT: &str = "قايمة مضغوطة";
    pub const SORT_TITLE: &str = "رتّب حسب";
    pub const SORT_BOOK: &str = "ترتيب الكتاب";
    pub const SORT_LEFT: &str = "اللي فاضل الأول";
    pub const SORT_NAME: &str = "بالاسم";
    pub const SORT_DUE: &str = "المستحق الأول";
    pub const GROUPED: &str = "قسّمها حسب المحاضرة";
    pub const GROUPED_SHORT: &str = "مقسّمة";
    pub const CLOSE_MENU: &str = "اقفل القايمة";
}

pub mod labs {
    pub const TITLE: &str = "القيم المعملية";
    pub const OPEN: &str = "القيم المعملية";
    pub const OPEN_HINT: &str = "افتح القيم الطبيعية زي ما الامتحان بيعرضها جنب كل سؤال";
    pub const SEARCH: &str = "دوّر على تحليل…";
    pub const UNITS: &str = "الوحدات";
    pub const CONVENTIONAL: &str = "وحدات الكتاب";
    pub const NO_MATCH: &str = "مفيش تحليل بالاسم ده. جرّب كلمة إنجليزي زي sodium.";
    pub const SMA12: &str = "من الـ SMA-12";
    pub const SMA12_NOTE: &str = "* جزء من الـ Biochemical Profile (SMA-12)";
}

pub mod courses {
    pub const TITLE: &str = "المواد";
    pub const ACTIVE: &str = "بذاكرها";
    pub const MAINTENANCE: &str = "مراجعة خفيفة";
    pub const ARCHIVED: &str = "مستخبية";
    pub const LATER: &str = "بعدين";
    pub const ACTIVE_HINT: &str = "بتظهر في اليوم والمراجعة والخطة.";
    pub const MAINTENANCE_HINT: &str = "خلصت قرايتها. مراجعات خفيفة بس عشان تفضل حاضرة ليوم الامتحان.";
    pub const ARCHIVED_HINT: &str = "مستخبية من اليوم، بس موجودة في المراجع وفي ملف Anki.";
    pub const LATER_HINT: &str = "لسه ما بدأتهاش. محاضراتها موجودة أول ما تحب تبدأ.";
    pub const G_GENERAL: &str = "General Principles";
    pub const G_SYSTEMS: &str = "Organ Systems";
    pub const G_GENERAL_HINT: &str = "Section II في الكتاب: الأساسيات اللي بتتسأل جوه كل جهاز.";
    pub const G_SYSTEMS_HINT: &str = "Section III: كل جهاز لوحده، من الـ anatomy للـ pharmacology.";
    pub const STATUS: &str = "حالة المادة";
    pub const OPEN_LECTURES: &str = "افتح المحاضرات";
    pub const NO_CONTENT: &str = "لسه مفيش محاضرات للمادة دي.";
    pub const ALL: &str = "كل المواد";
    pub const PICK_TITLE: &str = "تذاكر إيه دلوقتي؟";
    pub const MAP_LINK: &str = "خريطة المواد وتقدّمك";

    pub fn chip_label(name: &str) -> String {
        format!("المادة المعروضة: {name}. دوس عشان تبدّل")
    }
    use super::count::{CARD, COURSE, LECTURE, LESSON, PAGE, count};

    pub fn all_sub(courses: usize) -> String {
        format!("{} مع بعض", count(courses, &COURSE))
    }
    /// "3 من 12 درس", "1 من درسين".
    pub fn lessons_done(done: usize, all: usize) -> String {
        format!("{done} من {}", count(all, &LESSON))
    }
    pub fn lectures_only(n: usize) -> String {
        format!("{}، الدروس جاية", count(n, &LECTURE))
    }

    /// "614 صفحة من First Aid في 16 مادة".
    pub fn pages_line(pages: u32, courses: usize) -> String {
        format!("{} من First Aid في {}", count(pages, &PAGE), count(courses, &COURSE))
    }
    /// "بدأت 12 من 40 كارت".
    pub fn started(seen: usize, total: usize) -> String {
        format!("بدأت {seen} من {}", count(total, &CARD))
    }
    /// The tile's foot: "قلب · ص 284–328".
    pub fn tile_line(short: &str, from: u32, to: u32) -> String {
        format!("{short} · ص {from}–{to}")
    }
    /// "First Aid ص 284–328 (45 صفحة)".
    pub fn book_pages(from: u32, to: u32, pages: u32) -> String {
        format!("First Aid ص {from}–{to} ({})", count(pages, &PAGE))
    }
    pub fn content_line(written: usize, lectures: usize, items: usize, lessons: usize) -> String {
        format!(
            "{} · {} · {}",
            ready_lectures(written, lectures),
            count(items, &CARD),
            count(lessons, &LESSON)
        )
    }
    /// "مكتوب 3 من 12 محاضرة".
    pub fn ready_lectures(written: usize, total: usize) -> String {
        format!("مكتوب {written} من {}", count(total, &LECTURE))
    }
}

#[cfg(test)]
mod grammar_tests {
    use super::*;

    #[test]
    fn course_map_lines_agree() {
        assert_eq!(courses::pages_line(614, 16), "614 صفحة من First Aid في 16 مادة");
        assert_eq!(courses::pages_line(2, 2), "صفحتين من First Aid في مادتين");
        assert_eq!(courses::started(12, 40), "بدأت 12 من 40 كارت");
        assert_eq!(courses::started(0, 5), "بدأت 0 من 5 كروت");
        assert_eq!(courses::tile_line("قلب", 284, 328), "قلب · ص 284–328");
        assert_eq!(courses::book_pages(284, 328, 45), "First Aid ص 284–328 (45 صفحة)");
        assert_eq!(courses::all_sub(9), "9 مواد مع بعض");
        assert_eq!(courses::all_sub(12), "12 مادة مع بعض");
        assert_eq!(courses::lessons_done(3, 7), "3 من 7 دروس");
        assert_eq!(courses::lessons_done(3, 30), "3 من 30 درس");
        assert_eq!(courses::lectures_only(5), "5 محاضرات، الدروس جاية");
        assert_eq!(courses::ready_lectures(1, 2), "مكتوب 1 من محاضرتين");
        assert_eq!(courses::content_line(3, 4, 120, 6), "مكتوب 3 من 4 محاضرات · 120 كارت · 6 دروس");
    }

    #[test]
    fn other_counted_lines_agree() {
        assert_eq!(today::exam_in(5), "فاضل 5 أيام على الامتحان");
        assert_eq!(today::exam_in(40), "فاضل 40 يوم على الامتحان");
        assert_eq!(today::plan_line(3, 4), "3 جديد + 4 مراجعات");
        assert_eq!(stats::day_bar(5, 2), "5 دقايق، إجابتين");
        assert_eq!(table::mark_each(2), "فاضل خانتين: علّم على كل واحدة عرفتها ولا لأ.");
        assert_eq!(table::recalled(1, 12), "افتكرت 1 من 12 خانة.");
        assert_eq!(order::moves_to_fix(2), "الترتيب قريب: نقل خطوتين بس بيظبطه.");
        assert_eq!(learn::misses(2), "وصّلتهم كلهم، بغلطتين في السكة.");
        assert_eq!(learn::misses(4), "وصّلتهم كلهم، بـ 4 غلطات في السكة.");
        assert_eq!(update::notice_more(1), "وحاجة واحدة كمان");
        assert_eq!(update::notice_more(4), "و4 حاجات كمان");
        assert_eq!(cloud::pending(2), "تغييرين مستنيين يترفعوا");
        assert_eq!(cloud::pending(14), "14 تغيير مستني يترفع");
        assert_eq!(data::import_partial(3).split(" (").next().unwrap(), "بس فيه 3 صفوف ما اتدمجوش");
        assert_eq!(study::enum_goal(8, 5), "فيه 8 نقط، ولو افتكرت 5 منهم تعدّي.");
        assert_eq!(plan::days_left(40), "فاضل 40 يوم مذاكرة");
        assert_eq!(anki::contents(12, 2), "12 محاضرة · سؤالين");
    }

    #[test]
    fn each_count_takes_the_right_form() {
        assert_eq!(today::due_cases(2), "حالتين مستنية مراجعة في شاشة الحالات");
        assert_eq!(today::exam_in(1), "الامتحان بكرة. إنت جاهز أكتر مما فاكر.");
        assert_eq!(today::exam_in(2), "فاضل يومين على الامتحان");
        assert!(today::tight_days(1).starts_with("الخطة محتاجة يوم مذاكرة زيادة في الأسبوع"));
        assert!(today::tight_days(3).starts_with("الخطة محتاجة 3 أيام مذاكرة زيادة في الأسبوع"));
        assert_eq!(today::headline(0, 5), "5 حاجات جديدة، على قد وقتك.");
        assert_eq!(today::headline(1, 0), "مراجعة واحدة، وبس.");
        assert_eq!(today::headline(2, 12), "مراجعتين، و12 حاجة جديدة.");
        assert_ne!(today::hero_body(0, 3), today::hero_body(3, 0));
        assert_ne!(today::hero_body(3, 0), today::hero_body(3, 3));
        assert_eq!(today::plan_reviews_only(7), "7 مراجعات بس");
        assert_eq!(
            study_settings::personal(40),
            format!("{} 40 مراجعة. بيتحدّث كل أسبوع.", study_settings::PERSONAL_1)
        );
        assert_eq!(study::due_elsewhere(1), "فيه كارت واحد مستحق في مادة تانية.");
        assert_eq!(study::due_elsewhere(2), "فيه كارتين مستحقين في مواد تانية.");
        assert_eq!(study::due_elsewhere(4), "فيه 4 كروت مستحقة في مواد تانية.");
        assert_eq!(study::due_elsewhere(11), "فيه 11 كارت مستحق في مواد تانية.");
        assert_eq!(table::mark_each(1), "فاضل خانة واحدة: عرفتها ولا لأ؟");
        assert_eq!(table::recalled(3, 3), "كل الخانات صح (3 من 3). حلو أوي!");
        assert_eq!(order::moves_to_fix(1), "الترتيب قريب: نقل خطوة واحدة بس بيظبطه.");
        assert_eq!(case::steps(5), "5 خطوات");
        assert_eq!(own::cards(1), "كارت واحد");
        assert_eq!(cloud::pending(1), "تغيير واحد مستني يترفع");
        assert_eq!(cloud::pending(5), "5 تغييرات مستنية تترفع");
        assert_eq!(cloud::pending(103), "103 تغييرات مستنية تترفع");
        assert_eq!(update::builds_count(1), "نسخة جديدة");
        assert_eq!(update::builds_count(2), "نسختين جداد");
        assert_eq!(update::builds_count(4), "4 نسخ جديدة");
        assert_eq!(update::builds_count(12), "12 نسخة جديدة");
        assert_eq!(update::content_updates_count(1), "تحديث محتوى");
        assert_eq!(update::content_updates_count(2), "تحديثين محتوى");
        assert_eq!(update::content_updates_count(3), "3 تحديثات محتوى");
        assert_eq!(update::content_updates_count(20), "20 تحديث محتوى");
        assert_eq!(remind::text(None), remind::text(Some(0)));
        assert_eq!(remind::text(Some(1)), "عندك مراجعة واحدة النهارده. دقيقة وتخلص.");
        assert_eq!(remind::text(Some(14)), "عندك 14 مراجعة النهارده. يلا نبدأ بجلسة قصيرة.");
        assert_eq!(onboard::days_sub(7), "كل أيام الأسبوع");
        assert_eq!(onboard::days_sub(1), "يوم واحد في الأسبوع");
        assert_eq!(onboard::days_sub(2), "يومين في الأسبوع");
        assert_eq!(onboard::days_sub(5), "5 أيام في الأسبوع");
        assert_eq!(onboard::studied_sub(0), "ولا محاضرة لسه");
        assert_eq!(onboard::studied_sub(1), "محاضرة واحدة");
        assert_eq!(onboard::studied_sub(2), "محاضرتين");
        assert_eq!(onboard::studied_sub(9), "9 محاضرات");
        assert_eq!(onboard::studied_sub(11), "11 محاضرة");
        assert_eq!(onboard::studied_sub(103), "103 محاضرات");
        assert_eq!(recap::exam_in(0), "الامتحان النهارده. بالتوفيق.");
        assert_eq!(recap::exam_in(30), "فاضل على الامتحان 30 يوم.");
        assert_eq!(learn::misses(1), "وصّلتهم كلهم، بغلطة واحدة في السكة.");
        assert_eq!(learn::score_line(5, 5), "ولا غلطة. الدرس ده بقى بتاعك.");
        assert_eq!(learn::score_line(3, 5), "كويس جدًا. اللي غلطت فيه هيرجعلك في المراجعة.");
        assert_eq!(learn::score_line(1, 5), "عادي جدًا من أول مرة، ده درس جديد. الكروت هتثبّته.");
    }

    #[test]
    fn numbers_and_names_land_where_the_sentence_wants_them() {
        assert_eq!(cite::slide(4), "سلايد 4");
        assert_eq!(bank::all_of("قلب"), "كل محاضرات قلب");
        assert_eq!(bank::checked(true, 'C'), "إجابتك صح.");
        assert_eq!(bank::checked(false, 'C'), "إجابتك غلط. الإجابة الصح C.");
        assert_eq!(stats::better(true, 12), "أحسن من الأسبوع اللي فات بـ 12%");
        assert_eq!(stats::better(false, 3), "أحسن من الشهر اللي فات بـ 3%");
        assert!(today::tight_cap(150).starts_with("لو علّيت حد المراجعات اليومي لـ 150، الخطة تلحق كلها."));
        assert_eq!(plan::tight_cap(150), "لو علّيت حد المراجعات اليومي لـ 150 هتلحق كلها.");
        assert!(order::in_place(3, 5).starts_with("3 من 5 في مكانها."));
        assert_eq!(order::should_be(2), "مكانها 2");
        assert_eq!(learn::placed_of(2, 6), "اخترت 2 من 6");
        assert_eq!(learn::slot(3), "الخطوة رقم 3");
        assert_eq!(learn::should_be(1), "مكانها رقم 1");
        assert_eq!(learn::sort_left(4, 9), "فاضل 4 من 9");
        assert_eq!(learn::in_place(5, 7), "5 من 7 في مكانهم.");
        assert!(learn::no_match("VSD").starts_with("مفيش درس فيه «VSD»."));
        assert_eq!(learn::resume_at(3, 8), "هتكمّل من خطوة 3 من 8");
    }
}

pub mod refs {
    pub const TITLE: &str = "مراجعك";
    pub const ADD_CARD: &str = "كارت جديد";
    pub const SEARCH: &str = "دوّر في المحاضرات";
    pub const SEARCH_HINT: &str = "دوّر على محاضرة أو موضوع…";
    pub const LECTURES: &str = "المحاضرات";
    pub const NO_MATCH: &str = "مفيش محاضرة أو موضوع بالاسم ده. جرّب كلمة إنجليزي من العنوان.";
    pub const CHECKED: &str = "راجعت صحته";
    pub const CHECKED_HINT: &str = "اللي علّمت عليه ✓ صح وإنت بتذاكر، من كل اللي في المحاضرة.";
    pub const WRITING: &str = "لسه بتتكتب";
    pub const STATUS: &str = "حالة المادة";
    pub const STATUS_BODY: &str = "إنك تخلّص درس أو تجاوب صح مش معناه إن المحتوى اتراجع. صحة المحتوى بتتحسب بس من ✓ اللي إنت بتديها للكارت وإنت بتذاكر، والباقي فاضل unverified لحد ما تراجعه.";
    pub const CHECKED_ALL: &str = "راجعت صحته في كل المحاضرات:";
}

pub mod lecture {
    pub const BACK: &str = "رجوع";
    pub const KICKER: &str = "المحاضرة";
    pub const PAGES: &str = "صفحات";
    pub const SLIDES: &str = "سلايدز";
    pub const PROGRESS: &str = "اللي بدأته من المحاضرة";
    pub const OF: &str = "من";
    pub const STARTED: &str = "بدأتهم";
    pub const DUE: &str = "مستحق";
    pub const STUDY: &str = "ذاكر المحاضرة دي";
    pub const REVIEW_DUE: &str = "راجع المستحق";
    pub const WRITING: &str = "المحاضرة دي لسه بتتكتب. أول ما تجهز هتلاقي أسئلتها هنا.";
    pub const LESSONS: &str = "الدروس";
    pub const PRACTICE: &str = "اتمرّن";
    pub const MCQS: &str = "سؤال اختيارات في البنك";
    pub const CASES: &str = "حالة";
    pub const TOPICS: &str = "المواضيع";
    pub const CHECKED: &str = "صحة المحتوى: راجعت";
    /// "قريت 6 من 21 صفحة" (A2).
    pub fn read_count(read: u32, all: u32) -> String {
        format!("قريت {read} من {all} صفحة")
    }
    pub const READ_NONE: &str = "لسه ما علّمتش صفحات مقروءة في المحاضرة دي.";
    pub const READ_ALL: &str = "قريت المحاضرة كلها";
    pub const UNREAD_ALL: &str = "شيل علامة القراءة";
    pub const READ_UPTO: &str = "قريت لحد صفحة";
    pub const READ_UPTO_GO: &str = "علّم";
    pub const READ_GROUP: &str = "الصفحات اللي قريتها";
}

/// Shared preference write recovery.
pub mod saving {
    pub const FAILED: &str = "فيه إعدادات ما اتحفظتش. اختياراتك شغالة دلوقتي، بس ممكن تضيع لما تقفل التطبيق.";
    pub const RETRY: &str = "حاول تحفظ تاني";
    pub const TRY_AGAIN: &str = "حاول تاني";
}

/// His notes on items, lesson steps and pages (notes/mod.rs).
pub mod notes {
    pub const CHIP_ADD: &str = "＋ ملاحظة";
    pub const CHIP_HAS: &str = "ملاحظتك";
    pub const ADD_HINT: &str = "اكتب ملاحظة";
    pub const EDIT_HINT: &str = "افتح ملاحظتك";
    pub const TITLE_NEW: &str = "ملاحظة جديدة";
    pub const TITLE_EDIT: &str = "ملاحظتك";
    pub const FIELD: &str = "الملاحظة";
    pub const HINT: &str = "بالعربي أو بالإنجليزي: اللي عايز تفتكره هنا…";
    pub const PRIVATE: &str = "ملاحظاتك بتتحفظ على أجهزتك بس، وبتروح معاك في النسخة الاحتياطية وفي Anki.";
    pub const SAVE: &str = "احفظ";
    pub const DELETE: &str = "امسحها";
    pub const CANCEL: &str = "إلغاء";
    pub const TOO_LONG: &str = "الملاحظة طويلة، خليها 2000 حرف بالكتير. لو محتاج أكتر، اعمل كارت.";
    pub const FAILED: &str = "ما عرفتش أحفظ الملاحظة، وكلامك لسه هنا:";
    pub const ON_LESSON: &str = "درس";
    pub const PAGE_NONE: &str = "مفيش ملاحظات على الصفحة دي لسه.";
    pub const ADD_PAGE: &str = "اكتب ملاحظة على الصفحة دي";
    pub const LIST_TITLE: &str = "ملاحظاتك";
    pub const LIST_EMPTY: &str =
        "لسه مكتبتش ملاحظات. اضغط «＋ ملاحظة» تحت أي كارت أو سؤال أو خطوة في درس، أو من صفحة الكتاب.";
    pub const COURSES: &str = "المواد";
    pub const ALL: &str = "الكل";
    pub const SEARCH: &str = "دوّر في ملاحظاتك";
    pub const NO_MATCH: &str = "مفيش ملاحظة فيها الكلام ده.";
    /// "ملاحظة على ص 284".
    pub fn on_page(page: u32) -> String {
        format!("ملاحظة على {}", super::cite::page(page))
    }
    /// "ملاحظاتك على ص 284".
    pub fn on_page_title(page: u32) -> String {
        format!("ملاحظاتك على {}", super::cite::page(page))
    }
    pub fn count(n: usize) -> String {
        super::count::count(n, &super::count::NOTE)
    }
}
