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
pub const CONTENT_FETCHING: &str = "بيجيب الدروس والأسئلة الجديدة…";

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
