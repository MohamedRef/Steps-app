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
