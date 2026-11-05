pub const TITLE: &str = "Steps";
pub const SECTION: &str = "التذكير";
pub const SECTION_SUB: &str = "تذكير هادي بمراجعة اليوم";
/// Android's notification channel, as Settings › Notifications lists it.
pub const CHANNEL: &str = "تذكير المذاكرة";
pub const CHANNEL_ABOUT: &str = "تذكير هادي بمراجعة اليوم";
pub const PHONE_ONLY: &str = "التذكير بيرن من تطبيق Steps المتثبّت على الموبايل أو الماك، مش من نسخة التطوير.";
pub const MAC_HINT: &str =
    "على الماك الإشعار بيظهر حتى لو Steps مقفولة. لو ما ظهرش، اسمح لـ Steps من إعدادات النظام › الإشعارات.";
pub const DAILY: &str = "تذكير المذاكرة اليومي";
pub const ON: &str = "شغّال";
pub const OFF: &str = "مقفول";
pub const TIMES: &str = "المواعيد";
pub const TIME: &str = "معاد التذكير";
pub const ADD: &str = "زوّد معاد";
pub const REMOVE: &str = "شيل";
pub const HERE: &str = "يرن على الجهاز ده";
pub const BLOCKED: &str = "الإشعارات مقفولة لـ Steps، فالتذكير مش هيظهر.";
pub const ALLOW: &str = "اسمح بالإشعارات";
pub const OPEN_NOTIFICATIONS: &str = "افتح إعدادات الإشعارات";
pub const BATTERY: &str = "لو التذكير ما رنّش، خلّي البطارية لـ Steps «من غير قيود» عشان الموبايل ما ينيّمهاش.";
pub const OPEN_APP: &str = "افتح إعدادات التطبيق";

/// The reminder line, with the reviews due that day when known.
pub fn text(due: Option<u32>) -> String {
    match due {
        Some(0) | None => "وقت جلسة قصيرة. كارت واحد أحسن من مفيش.".into(),
        Some(1) => "عندك مراجعة واحدة النهارده. دقيقة وتخلص.".into(),
        Some(n) => {
            format!("عندك {} النهارده. يلا نبدأ بجلسة قصيرة.", super::count::count(n, &super::count::REVIEW))
        }
    }
}
