pub mod kinds;
mod application_vise;
mod error;
mod file;
mod id;
mod manager;
mod source;

pub use error::Error;
pub use file::{File, RefNum};
pub use id::{OsType, OsTypeReadExt, ResourceId};
pub use manager::Manager;
pub use source::Source;
use application_vise::ApplicationVise;

pub type Result<T> = core::result::Result<T, Error>;
