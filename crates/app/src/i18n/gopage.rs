pub const TITLE: &str = "روح لصفحة في First Aid";
pub const TOOL: &str = "روح لصفحة";
pub const HINT: &str = "اكتب رقم الصفحة المطبوع في الكتاب (أو مدى زي 300–305).";
pub const PLACEHOLDER: &str = "مثلاً 300";
pub const GO: &str = "افتح";
pub const INVALID: &str = "اكتب رقم صفحة بس.";
pub fn outside(page: u32) -> String {
    format!("صفحة {page} مش جوه أي محاضرة في Steps (المحاضرات من Section II و III).")
}
pub fn nearest(page: u32) -> String {
    format!("افتح أقرب صفحة: {page}")
}
