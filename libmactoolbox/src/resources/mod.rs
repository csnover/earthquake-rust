pub mod kinds;
mod application_vise;
mod error;
mod file;
mod id;
mod manager;
#[macro_use]
mod source;

pub use error::Error;
pub use file::{File, RefNum};
pub use id::{OsType, OsTypeReadExt, ResNum, ResourceId};
pub use manager::Manager;
pub use source::{Source, TypedResource};
use application_vise::ApplicationVise;

pub type Result<T> = core::result::Result<T, Error>;
