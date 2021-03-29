#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid event kind")]
    BadEventKind,
}
