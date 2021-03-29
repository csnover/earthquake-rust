use super::ScriptCode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid country code {0}")]
    BadCountryCode(u16),
    #[error("invalid script code")]
    BadScriptCode,
    #[error("can’t find encoder for script code {0}")]
    NoEncoder(ScriptCode),
    #[error("invalid input for script code {0}")]
    InvalidInput(ScriptCode),
}
