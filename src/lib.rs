pub mod cli;
pub mod models;
pub mod scanner;
pub mod utils;

pub use models::{ExtraData, FileInfo, FileType, Header, Output, SystemEnvironment};
pub use scanner::{ProcessResult, count, process};
