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
