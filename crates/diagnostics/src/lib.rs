#[derive(Debug)]
pub struct VerifyReport {
    pub missing: Vec<String>,
    pub corrupted: Vec<String>,
    pub ok: usize,
}
