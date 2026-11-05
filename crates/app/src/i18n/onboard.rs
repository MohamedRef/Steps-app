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
pub const WELCOME_NOTE: &str = "الفهم والتذكّر وتدريب الأسئلة، كل واحد له مكانه. والكتاب هو المرجع في الامتحان.";
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
