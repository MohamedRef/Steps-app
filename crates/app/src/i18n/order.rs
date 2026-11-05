pub const HINT: &str = "دوس على الخطوات بالترتيب. تقدر ترجّع خطوة بالدوس عليها، أو تحركها بالأسهم أو بالسحب.";
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
