//! The VQ question bank (vq/): the key card in Settings › بياناتك, the
//! bank's filter, quick search and a question's figure.

use super::count::{QUESTION, count};

pub const TITLE: &str = "مفتاح بنك VQ";
pub const INTRO: &str = "أسئلة VQ بتاعتك جاية مقفولة جوه المحتوى، وبتتفتح على الجهاز اللي تكتب فيه مفتاحها.";
pub const LOCKED: &str = "مقفول";
pub const OPENED_1: &str = "مفتوح:";
/// «مفتوح: 1314 سؤال».
pub fn opened(n: usize) -> String {
    format!("{OPENED_1} {}", count(n, &QUESTION))
}
pub const NONE_YET: &str = "لسه مفيش أسئلة VQ في المحتوى. لو حفظت المفتاح، هتتفتح لوحدها أول ما توصل.";
pub const STORED_OTHER: &str = "المفتاح اللي على الجهاز ده مش بتاع الأسئلة دي. الزق المفتاح بتاعها.";
pub const FIELD: &str = "المفتاح";
pub const PLACEHOLDER: &str = "الزق المفتاح هنا";
pub const FIELD_HINT: &str = "تلاقيه في مدير الباسوردات بتاعك. المسافات والشَرط والحروف الصغيرة مش فارقين.";
pub const OPEN: &str = "افتح";
pub const FORGET: &str = "اقفل على الجهاز ده";
pub const DEVICE_ONLY: &str =
    "المفتاح بيتحفظ على الجهاز ده بس: مش في النسخة الاحتياطية ولا في المزامنة، فكل جهاز بيتفتح لوحده.";
pub const SAID_EMPTY: &str = "الزق المفتاح الأول.";
pub const SAID_NOT_A_KEY: &str = "ده مش شكل مفتاح VQ: المفتاح 52 حرف ورقم في مجموعات من 4. انسخه تاني كله.";
pub const SAID_OTHER_KEY: &str =
    "المفتاح ده سليم، بس مش هو اللي اتقفلت بيه الأسئلة دي. اتأكد إنك نسخت مفتاح VQ.";
pub const SAID_NOT_KEPT: &str = "ما عرفتش أحفظ المفتاح على الجهاز ده. جرّب تاني.";
pub const SAID_OPENED: &str = "اتفتحت. أسئلة VQ بقت في البنك جنب أسئلة محاضراتها.";
pub const SAID_KEPT: &str = "اتحفظ المفتاح. أسئلة VQ هتتفتح لوحدها أول ما توصل.";
pub const SAID_LOCKED: &str = "اتقفلت على الجهاز ده. المفتاح لسه معاك في مدير الباسوردات.";

/// The bank's filter chip for the `vq` tag.
pub const TAG: &str = "أسئلة VQ";
/// Quick search: what a VQ hit is.
pub const KIND: &str = "سؤال VQ";

/// The heading of a question's educational objective: English, like the
/// medical text it heads (Mohamed, 2026-11-18).
pub const OBJECTIVE: &str = "Educational objective";

pub const FIGURE_ZOOM: &str = "كبّر الصورة";
pub const FIGURE_CLOSE: &str = "اقفل الصورة";
/// «صورة السؤال VQ-0001».
pub fn figure_alt(name: &str) -> String {
    format!("صورة السؤال {name}")
}
