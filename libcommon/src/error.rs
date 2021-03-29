pub fn flatten_errors<T>(mut result: anyhow::Result<T>, chained_error: &anyhow::Error) -> anyhow::Result<T> {
    for error in chained_error.chain() {
        result = anyhow::Context::context(result, anyhow::anyhow!("{}", error));
    }
    result
}

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
