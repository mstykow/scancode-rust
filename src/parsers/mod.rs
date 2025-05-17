mod cargo;
#[cfg(test)]
mod cargo_test;
mod npm;
#[cfg(test)]
mod npm_test;
mod python;
#[cfg(test)]
mod python_test;

use std::path::Path;

use crate::models::PackageData;

pub trait PackageParser {
    const PACKAGE_TYPE: &'static str;

    fn extract_package_data(path: &Path) -> PackageData;
    fn is_match(path: &Path) -> bool;
}

pub use self::cargo::CargoParser;
pub use self::npm::NpmParser;
pub use self::python::PythonParser;
