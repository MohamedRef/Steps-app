pub const BACK: &str = "رجوع";
pub const KICKER: &str = "المحاضرة";
pub const PAGES: &str = "صفحات";
pub const SLIDES: &str = "سلايدز";
pub const PROGRESS: &str = "اللي بدأته من المحاضرة";
pub const OF: &str = "من";
pub const STARTED: &str = "بدأتهم";
pub const DUE: &str = "مستحق";
pub const STUDY: &str = "ذاكر المحاضرة دي";
pub const REVIEW_DUE: &str = "راجع المستحق";
pub const WRITING: &str = "المحاضرة دي لسه بتتكتب. أول ما تجهز هتلاقي أسئلتها هنا.";
pub const LESSONS: &str = "الدروس";
pub const PRACTICE: &str = "اتمرّن";
pub const MCQS: &str = "سؤال اختيارات في البنك";
pub const CASES: &str = "حالة";
pub const TOPICS: &str = "المواضيع";
pub const CHECKED: &str = "صحة المحتوى: راجعت";
/// "قريت 6 من 21 صفحة" (A2).
pub fn read_count(read: u32, all: u32) -> String {
    format!("قريت {read} من {all} صفحة")
}
pub const READ_NONE: &str = "لسه ما علّمتش صفحات مقروءة في المحاضرة دي.";
pub const READ_ALL: &str = "قريت المحاضرة كلها";
pub const UNREAD_ALL: &str = "شيل علامة القراءة";
pub const READ_UPTO: &str = "قريت لحد صفحة";
pub const READ_UPTO_GO: &str = "علّم";
pub const READ_GROUP: &str = "الصفحات اللي قريتها";
