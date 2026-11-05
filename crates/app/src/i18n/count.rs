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
pub const LESSON: Noun = Noun { one: "درس واحد", two: "درسين", few: "دروس", many: "درس" };
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
pub const LINE: Noun = Noun { one: "سطر واحد", two: "سطرين", few: "سطور", many: "سطر" };
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
            &SECOND, &MINUTE, &HOUR, &DAY, &MONTH, &QUESTION, &LECTURE, &TOPIC, &ITEM, &REVIEW, &NEW_THING,
            &TIME, &CASE, &FIELD, &LESSON, &COURSE, &CARD, &STEP, &MISTAKE, &ANSWER, &LINE, &THING, &CHANGE,
            &ROW, &POINT, &STUDY_DAY, &PAGE,
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
