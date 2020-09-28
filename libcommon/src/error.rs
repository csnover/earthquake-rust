pub trait ReasonsExt {
    fn reasons(&self) -> String;
}

impl ReasonsExt for anyhow::Error {
    fn reasons(&self) -> String {
        let mut reasons = String::new();
        for reason in self.chain().skip(1) {
            if !reasons.is_empty() {
                reasons += "\n";
            }
            reasons += &format!("• {}", reason);
        }
        reasons
    }
}
