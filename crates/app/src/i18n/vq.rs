//! The VQ question bank (vq/): the key card in Settings › بياناتك, the
//! bank's set «أسئلة MH», quick search, a question's figure and its video.
//! Mohamed calls the bank MH (2026-11-19); its numbers stay `VQ-0001`.

use super::count::{QUESTION, count};

pub const TITLE: &str = "مفتاح أسئلة MH";
pub const INTRO: &str = "أسئلة MH بتاعتك جاية مقفولة جوه المحتوى، وبتتفتح على الجهاز اللي تكتب فيه مفتاحها.";
pub const LOCKED: &str = "مقفول";
pub const OPENED_1: &str = "مفتوح:";
/// «مفتوح: 1314 سؤال».
pub fn opened(n: usize) -> String {
    format!("{OPENED_1} {}", count(n, &QUESTION))
}
pub const NONE_YET: &str = "لسه مفيش أسئلة MH في المحتوى. لو حفظت المفتاح، هتتفتح لوحدها أول ما توصل.";
pub const STORED_OTHER: &str = "المفتاح اللي على الجهاز ده مش بتاع الأسئلة دي. الزق المفتاح بتاعها.";
pub const FIELD: &str = "المفتاح";
pub const PLACEHOLDER: &str = "الزق المفتاح هنا";
pub const FIELD_HINT: &str = "تلاقيه في مدير الباسوردات بتاعك. المسافات والشَرط والحروف الصغيرة مش فارقين.";
pub const OPEN: &str = "افتح";
pub const FORGET: &str = "اقفل على الجهاز ده";
pub const DEVICE_ONLY: &str =
    "المفتاح بيتحفظ على الجهاز ده بس: مش في النسخة الاحتياطية ولا في المزامنة، فكل جهاز بيتفتح لوحده.";
pub const SAID_EMPTY: &str = "الزق المفتاح الأول.";
pub const SAID_NOT_A_KEY: &str = "ده مش شكل مفتاح MH: المفتاح 52 حرف ورقم في مجموعات من 4. انسخه تاني كله.";
pub const SAID_OTHER_KEY: &str =
    "المفتاح ده سليم، بس مش هو اللي اتقفلت بيه الأسئلة دي. اتأكد إنك نسخت مفتاح MH.";
pub const SAID_NOT_KEPT: &str = "ما عرفتش أحفظ المفتاح على الجهاز ده. جرّب تاني.";
pub const SAID_OPENED: &str = "اتفتحت. أسئلة MH بقت في بنك الأسئلة، في بندها لوحدها.";
pub const SAID_KEPT: &str = "اتحفظ المفتاح. أسئلة MH هتتفتح لوحدها أول ما توصل.";
pub const SAID_LOCKED: &str = "اتقفلت على الجهاز ده. المفتاح لسه معاك في مدير الباسوردات.";

/// The bank's set of VQ questions, beside Steps' own, and the side menu's
/// way to it.
pub const SET: &str = "أسئلة MH";
/// The bank's heading with that set in view.
pub const BANK_TITLE: &str = "بنك أسئلة MH";
/// Quick search: what a VQ hit is.
pub const KIND: &str = "سؤال MH";
/// Quick search: the video of the question asked for by its number.
pub const KIND_VIDEO: &str = "فيديو الشرح";

/// The video that explains the question, with the explanation.
pub const VIDEO: &str = "فيديو الشرح";
pub const VIDEO_OPEN: &str = "افتح الفيديو في المتصفح";
pub const VIDEO_COPY: &str = "انسخ اللينك";
pub const VIDEO_COPIED: &str = "اتنسخ ✓";

/// The heading of a question's educational objective: English, like the
/// medical text it heads (Mohamed, 2026-11-18).
pub const OBJECTIVE: &str = "Educational objective";

pub const FIGURE_ZOOM: &str = "كبّر الصورة";
pub const FIGURE_CLOSE: &str = "اقفل الصورة";
/// «صورة السؤال VQ-0001».
pub fn figure_alt(name: &str) -> String {
    format!("صورة السؤال {name}")
}
