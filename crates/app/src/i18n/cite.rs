/// "ص 179".
pub fn page(page: u32) -> String {
    format!("ص {page}")
}
/// "سلايد 25": a lecture's slide (courses taught from slides).
pub fn slide(n: u32) -> String {
    format!("سلايد {n}")
}
