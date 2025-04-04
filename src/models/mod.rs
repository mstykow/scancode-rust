mod file_info;
mod output;

pub use file_info::{FileInfo, FileInfoBuilder, FileType, LicenseDetection, Match};
pub use output::{ExtraData, Header, Output, SCANCODE_OUTPUT_FORMAT_VERSION, SystemEnvironment};
