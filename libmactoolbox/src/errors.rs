use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("invalid country code {0}")]
    BadCountryCode(u16),
}
