//! The questions he adds from a file (ownq/): the card in Settings › بياناتك,
//! what a file's rows get wrong, and the bank's name for each category.

use steps_core::ownq::Field;

use super::count::{QUESTION, count};
use crate::ownq::report::Group;

pub const TITLE: &str = "ضيف أسئلة من ملف";
pub const INTRO: &str = "ارفع ملف Excel (أو JSON أو YAML)، لوحده أو في zip مع صوره. كل تصنيف بيبقى بنك لوحده، والأسئلة بتتزامن على كل أجهزتك.";
pub const UPDATE_HINT: &str =
    "عشان تحدّث: زوّد أو عدّل في نفس الملف وارفعه تاني. السؤال بنفس الرقم والتصنيف بيتعدّل مكانه.";
pub const PICK: &str = "اختار الملف";
pub const PICKING: &str = "اختار الملف…";
pub const PICK_FAILED: &str = "ما عرفتش أفتح اختيار الملفات";
pub const COPY_FAILED: &str = "ما عرفتش أفتح الملف ده.";
pub const TEMPLATE: &str = "نزّل التمبلت (Excel)";
pub const TEMPLATE_SAVED: &str = "التمبلت اتحفظ في:";
pub const TEMPLATE_FAILED: &str = "ما عرفتش أحفظ التمبلت:";
pub const READING: &str = "بقرا الملف…";
pub const FILE: &str = "الملف:";
pub const NONE_GOOD: &str = "مفيش ولا سؤال سليم في الملف ده. صلّح اللي تحت وارفعه تاني.";
pub const NOTHING_NEW: &str = "كل أسئلة الملف ده موجودة هنا زي ما هي.";
pub const SKIPPED: &str =
    "الصفوف اللي فيها غلطة مش هتتضاف، والباقي يتضاف عادي. صلّحها وارفع الملف تاني وقت ما تحب.";
pub const APPLY: &str = "ضيفهم";
pub const CANCEL: &str = "إلغاء";
pub const BACK: &str = "رجوع";
pub const AGAIN: &str = "ضيف ملف تاني";
pub const FAILED: &str = "ما عرفتش أحفظ الأسئلة:";
pub const YOURS: &str = "أسئلتك";
pub const DELETE: &str = "امسح";
pub const DELETE_SURE: &str = "أمسح كل أسئلة التصنيف ده من كل أجهزتك؟ إجاباتك عليها هتفضل في سجلّك.";
pub const DELETE_YES: &str = "أيوه امسحهم";
pub const MORE_PROBLEMS: &str = "وغلطات تانية بنفس الشكل.";
pub const WAITING_FIGURE: &str = "الصورة لسه بتوصل من جهازك التاني.";
pub const EXPLANATION: &str = "الشرح";

/// «هيتضاف 12 سؤال ويتعدّل 3 أسئلة في «أسئلة Pathoma».»
pub fn will(new: usize, changed: usize, sets: &[String]) -> String {
    let mut parts = Vec::new();
    if new > 0 {
        parts.push(format!("هيتضاف {}", count(new, &QUESTION)));
    }
    if changed > 0 {
        let verb = if parts.is_empty() { "هيتعدّل" } else { "يتعدّل" };
        parts.push(format!("{verb} {}", count(changed, &QUESTION)));
    }
    format!("{} في {}.", parts.join(" و"), quoted(sets))
}

fn quoted(sets: &[String]) -> String {
    sets.iter().map(|s| format!("«{s}»")).collect::<Vec<_>>().join(" و")
}

/// «اتضاف 12 سؤال واتعدّل 3 أسئلة في «أسئلة Pathoma».»
pub fn done(new: usize, changed: usize, sets: &[String]) -> String {
    let mut parts = Vec::new();
    if new > 0 {
        parts.push(format!("اتضاف {}", count(new, &QUESTION)));
    }
    if changed > 0 {
        parts.push(format!("اتعدّل {}", count(changed, &QUESTION)));
    }
    format!("{} في {}.", parts.join(" و"), quoted(sets))
}

/// «افتح أسئلة MH في البنك».
pub fn open_set(set: &str) -> String {
    format!("افتح {set} في البنك")
}

// The report of a file (ownq/report.rs), like Anki's import log.

pub const FIGURE: &str = "صورة";
pub const VIDEO: &str = "فيديو";
pub const TAP_TO_OPEN: &str = "دوس على أي سؤال تفتحه في البنك.";
pub const MOVED: &str = "اتنقل لمحاضرة تانية، فإجاباتك عليه هتبدأ من الأول.";

/// A report tile: what a file does (or did) to its questions.
pub fn group(g: Group, done: bool) -> &'static str {
    match (g, done) {
        (Group::New, true) => "اتضاف",
        (Group::New, false) => "هيتضاف",
        (Group::Changed, true) => "اتعدّل",
        (Group::Changed, false) => "هيتعدّل",
        (Group::Same, _) => "زي ما هو",
        (Group::Skipped, true) => "اتساب",
        (Group::Skipped, false) => "هيتساب",
    }
}

/// A report list's heading; its count follows.
pub fn group_title(g: Group, done: bool) -> &'static str {
    match (g, done) {
        (Group::New, true) => "الأسئلة اللي اتضافت",
        (Group::New, false) => "الأسئلة اللي هتتضاف",
        (Group::Changed, true) => "الأسئلة اللي اتعدّلت",
        (Group::Changed, false) => "الأسئلة اللي هتتعدّل",
        (Group::Same, true) => "موجودة زي ما هي، ما اتغيّرتش",
        (Group::Same, false) => "موجودة زي ما هي، مش هتتغيّر",
        (Group::Skipped, true) => "صفوف فيها غلطة، ما اتضافتش",
        (Group::Skipped, false) => "صفوف فيها غلطة، مش هتتضاف",
    }
}

/// «ص 118»: a First Aid page.
pub fn page(n: u32) -> String {
    format!("ص {n}")
}

/// «الإجابة C».
pub fn answer(letter: char) -> String {
    format!("الإجابة {letter}")
}

/// «و23 سؤال كمان.»: questions a report lists no more of.
pub fn more(n: usize) -> String {
    format!("و{} كمان.", count(n, &QUESTION))
}

/// A part of a question, as «اتغيّر: …» names it.
pub fn field(f: Field) -> &'static str {
    match f {
        Field::Lecture => "المحاضرة",
        Field::Topic => "الموضوع",
        Field::Page => "الصفحة",
        Field::Stem => "نص السؤال",
        Field::Options => "الاختيارات",
        Field::Answer => "الإجابة",
        Field::Why => "شرح الاختيارات",
        Field::Explanation => "الشرح",
        Field::Objective => "الهدف التعليمي",
        Field::Image => "الصورة",
        Field::Video => "الفيديو",
        Field::Tags => "الـ tags",
    }
}

/// «اتغيّر: الإجابة بقت C (كانت B)، الشرح».
pub fn changed(fields: &[Field], was: Option<char>, now: char) -> String {
    let parts: Vec<String> = fields
        .iter()
        .map(|f| match (f, was) {
            (Field::Answer, Some(w)) if w != now => format!("الإجابة بقت {now} (كانت {w})"),
            _ => field(*f).to_string(),
        })
        .collect();
    format!("اتغيّر: {}", parts.join("، "))
}

/// «اتمسح 12 سؤال.»
pub fn deleted(n: usize) -> String {
    format!("اتمسح {}.", count(n, &QUESTION))
}

/// «أسئلة Pathoma»: a category as a set of the bank.
pub fn set(category: &str) -> String {
    format!("أسئلة {category}")
}

/// «بنك أسئلة Pathoma».
pub fn bank_title(category: &str) -> String {
    format!("بنك أسئلة {category}")
}

/// Quick search: what a hit on one of his questions is.
pub const KIND: &str = "سؤال من أسئلتك";

/// «الصف 5» in a sheet, «السؤال 5» in a JSON or YAML list.
pub fn at(line: usize, sheet: bool) -> String {
    match (line, sheet) {
        (0, _) => "الملف".to_string(),
        (n, true) => format!("الصف {n}"),
        (n, false) => format!("السؤال {n}"),
    }
}

/// What a row gets wrong, in a few words; `field` names stay as the
/// sheet's headings.
pub fn problem(p: &steps_core::ownq::Problem) -> String {
    use steps_core::ownq::Problem as P;
    match p {
        P::NoRows => "مفيش أسئلة في الملف.".into(),
        P::UnknownColumn(c) => format!("عمود مش معروف «{c}»، هيتساب."),
        P::Missing(f) => format!("ناقص «{f}»."),
        P::BadId(id) => format!("الرقم «{id}» فيه حروف مش مسموحة: حروف إنجليزي وأرقام و - و _ بس."),
        P::Twice(id) => format!("الرقم «{id}» متكرر في نفس التصنيف (الرقم بين القوسين أول مرة)."),
        P::TooLong(f, max) => format!("«{f}» أطول من {max} حرف."),
        P::FewOptions => "محتاج اختيارين على الأقل، A وB.".into(),
        P::Gap(c) => format!("الاختيار {c} فاضي وبعده اختيارات: الاختيارات تبقى ورا بعض من غير فراغ."),
        P::BadAnswer(a) => format!("الإجابة «{a}» مش حرف من حروف الاختيارات."),
        P::WhyCount { options, why } => {
            format!("فيه شرح لـ {why} من {options} اختيارات: اكتب why لكل اختيار أو سيبهم كلهم فاضيين.")
        }
        P::NoPlace => "ناقص page أو lecture: صفحة First Aid أو رقم المحاضرة.".into(),
        P::PageNotInBook(n) => format!("صفحة {n} مش في محاضرة مكتوبة في Steps."),
        P::BadPage(t) => format!("الصفحة «{t}» مش رقم."),
        P::UnknownLecture { asked, course } => match (course.first(), course.last()) {
            (Some(first), Some(last)) if first != last => format!(
                "المحاضرة «{asked}» مش موجودة: المادة دي فيها من {first} لـ {last}. أو اكتب صفحة First Aid في page بدلها."
            ),
            (Some(only), _) => {
                format!(
                    "المحاضرة «{asked}» مش موجودة: المادة دي فيها {only} بس. أو اكتب صفحة First Aid في page بدلها."
                )
            }
            _ => format!(
                "المحاضرة «{asked}» مش موجودة: اكتبها زي cvs.L04، أو اكتب صفحة First Aid في page بدلها."
            ),
        },
        P::UnknownTopic(t) => {
            format!("الموضوع «{t}» مش في المحاضرة دي: اكتبه زي ما هو مكتوب في Steps، أو سيبه فاضي.")
        }
        P::BadVideo(v) => format!("«{v}» مش لينك فيديو يوتيوب."),
        P::NoImage(n) => format!("الصورة «{n}» مش في الـ zip."),
        P::TooManyTags => "أكتر من 10 tags.".into(),
    }
}

/// What is wrong with the file itself.
pub fn file_problem(p: &crate::ownq::file::FileProblem) -> String {
    use crate::ownq::file::FileProblem as F;
    match p {
        F::Kind(n) => format!("«{n}» مش zip ولا Excel ولا JSON ولا YAML."),
        F::NoQuestions => "الـ zip مفيهوش ملف Excel ولا JSON ولا YAML.".into(),
        F::Unreadable(e) => format!("ما عرفتش أقرا الملف: {e}"),
        F::ImageTooBig(n) => format!("الصورة «{n}» أكبر من 1 ميجا: صغّرها."),
        F::NotAnImage(n) => format!("«{n}» مش صورة PNG ولا JPG ولا WebP ولا GIF."),
    }
}
