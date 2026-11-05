// The self-assessment tab on the progress screen (stats/scores.rs).
pub const TAB: &str = "اختباراتي";
pub const VERDICT_TITLE: &str = "جاهزيتك";
pub const V_NONE: &str = "لسه ما سجّلتش أي فورم NBME. أول فورم هو نقطة بداية تعرف منها فين، مش حكم عليك.";
pub const V_ONE: &str = "فورم واحد بداية كويسة. محتاج فورم كمان عشان نشوف الاتجاه رايح فين.";
pub const V_READY: &str = "آخر فورمين 68% أو أكتر والاتجاه ثابت أو طالع: شكلك جاهز. كمّل على نفس الإيقاع.";
pub const V_CLOSE: &str = "قريب من الخط. كمّل مذاكرة وجرّب فورم كمان قبل ما تقرّر.";
pub const V_NOT_YET: &str = "لسه تحت المنطقة دي. لو الميعاد قرّب، فكّر تأجّله شوية: ده تعديل في الخطة، مش فشل.";
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
