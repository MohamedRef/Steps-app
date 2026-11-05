use super::count::{REVIEW, count};

pub const OFFER_TITLE: &str = "نلحق المراجعات على كام يوم؟";
pub const RUNNING_TITLE: &str = "خطة اللحاق شغالة";
pub const DAYS_LABEL: &str = "نوزّعها على";
pub const PAUSE_NEW: &str = "وقّف الجديد لحد ما نلحق";
pub const START: &str = "يلا نبدأ";
pub const STOP: &str = "وقّف الخطة";
pub const TODAY_DONE: &str = "خلصت نصيب النهارده. اللي فاضل مستنيك بكرة.";
pub const NEW_PAUSED: &str = "الجديد واقف لحد ما الخطة تخلص.";
/// "اتجمّع 120 مراجعة، حوالي 45 دقيقة. عادي، بيحصل."
pub fn offer_line(due: usize, minutes: u32) -> String {
    format!("اتجمّع عندك {}، حوالي {minutes} دقيقة. عادي، بيحصل، ونقسمهم على كام يوم.", count(due, &REVIEW))
}
pub fn days(n: u32) -> String {
    format!("{n} أيام")
}
pub fn per_day(minutes: u32) -> String {
    format!("يعني حوالي {minutes} دقيقة مراجعة في اليوم، الأقدم الأول.")
}
pub fn running_line(day: i64, days: u32) -> String {
    format!("اليوم {day} من {days}.")
}
pub fn today_left(left: usize, minutes: u32) -> String {
    format!("فاضل النهارده {}، حوالي {minutes} دقيقة.", count(left, &REVIEW))
}
