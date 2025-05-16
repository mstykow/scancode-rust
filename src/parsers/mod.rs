mod npm;
mod cargo;
mod python;
#[cfg(test)]
mod cargo_test;
#[cfg(test)]
mod python_test;
#[cfg(test)]
mod npm_test;

use std::path::Path;

use crate::models::PackageData;

pub trait PackageParser {
    const PACKAGE_TYPE: &'static str;

    fn extract_package_data(path: &Path) -> PackageData;
    fn is_match(path: &Path) -> bool;
}

pub use self::npm::NpmParser;
pub use self::cargo::CargoParser;
pub use self::python::PythonParser;
