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
