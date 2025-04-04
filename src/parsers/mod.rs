mod npm;

use std::path::Path;

use crate::models::PackageData;

pub trait PackageParser {
    const PACKAGE_TYPE: &'static str;

    fn extract_package_data(path: &Path) -> PackageData;
    fn is_match(path: &Path) -> bool;
}

pub use self::npm::NpmParser;
