mod count;
mod process;

use crate::models::FileInfo;

pub struct ProcessResult {
    pub files: Vec<FileInfo>,
    pub excluded_count: usize,
}

pub use self::count::count;
pub use self::process::process;
