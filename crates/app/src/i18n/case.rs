pub const TITLE: &str = "حالات";
pub const SUB: &str = "حالة خطوة بخطوة: فكّر زي الامتحان، وصحّح كل قرار قبل اللي بعده.";
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
