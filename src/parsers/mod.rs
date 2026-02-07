mod cargo;
mod cargo_lock;
#[cfg(test)]
mod cargo_lock_test;
#[cfg(test)]
mod cargo_test;
mod composer;
#[cfg(test)]
mod composer_golden_test;
#[cfg(test)]
mod composer_test;
mod dart;
#[cfg(test)]
mod dart_golden_test;
#[cfg(test)]
mod dart_test;
mod go;
#[cfg(test)]
mod go_golden_test;
#[cfg(test)]
mod go_test;
mod maven;
#[cfg(test)]
mod maven_test;
pub mod metadata;
mod npm;
#[cfg(test)]
mod npm_golden_test;
mod npm_lock;
#[cfg(test)]
mod npm_lock_test;
#[cfg(test)]
mod npm_test;
mod npm_workspace;
#[cfg(test)]
mod npm_workspace_test;
mod nuget;
#[cfg(test)]
mod nuget_golden_test;
#[cfg(test)]
mod nuget_test;
mod pep508;
mod pipfile_lock;
#[cfg(test)]
mod pipfile_lock_test;
mod pnpm_lock;
#[cfg(test)]
mod pnpm_lock_test;
mod poetry_lock;
#[cfg(test)]
mod poetry_lock_test;
mod python;
#[cfg(test)]
mod python_test;
mod requirements_txt;
#[cfg(test)]
mod requirements_txt_test;
mod ruby;
#[cfg(test)]
mod ruby_golden_test;
#[cfg(test)]
mod ruby_test;
pub mod utils;
mod yarn_lock;
#[cfg(test)]
mod yarn_lock_test;

use std::path::Path;

use crate::models::PackageData;

/// Package parser trait for extracting metadata from package manifest files.
///
/// Each parser implementation handles a specific package manager/ecosystem
/// (npm, Maven, Python, Cargo, etc.) and extracts standardized metadata into
/// `PackageData` structures compatible with ScanCode Toolkit JSON output format.
///
/// # Implementation Guide
///
/// Implementors must provide:
/// - `PACKAGE_TYPE`: Package URL (purl) type identifier (e.g., "npm", "pypi", "maven")
/// - `is_match()`: Returns true if the given file path matches this parser's expected format
/// - `extract_package_data()`: Parses the file and returns extracted metadata
///
/// # Error Handling
///
/// Parsers should handle errors gracefully by returning default/empty `PackageData`
/// and logging warnings rather than panicking. This allows the scan to continue
/// processing other files even when individual files fail to parse.
///
/// # Example
///
/// ```ignore
/// use scancode_rust::parsers::PackageParser;
/// use scancode_rust::models::PackageData;
/// use std::path::Path;
///
/// pub struct MyParser;
///
/// impl PackageParser for MyParser {
///     const PACKAGE_TYPE: &'static str = "my-package-type";
///
///     fn is_match(path: &Path) -> bool {
///         path.file_name()
///             .is_some_and(|name| name == "package-manifest.json")
///     }
///
///     fn extract_package_data(path: &Path) -> PackageData {
///         // Parse file and return metadata
///         // On error, log warning and return default
///         PackageData::default()
///     }
/// }
/// ```
pub trait PackageParser {
    /// Package URL type identifier for this parser (e.g., "npm", "pypi", "maven").
    const PACKAGE_TYPE: &'static str;

    /// Extracts package metadata from the given file path.
    ///
    /// Returns a `PackageData` structure containing all extracted metadata including
    /// name, version, dependencies, licenses, etc. On parse errors, returns a default
    /// `PackageData` with minimal or no fields populated.
    fn extract_package_data(path: &Path) -> PackageData;

    /// Checks if the given file path matches this parser's expected format.
    ///
    /// Returns true if the file should be handled by this parser based on filename,
    /// extension, or path patterns. Used by the scanner to route files to appropriate parsers.
    fn is_match(path: &Path) -> bool;
}

pub use self::cargo::CargoParser;
#[cfg_attr(not(test), allow(unused_imports))]
pub use self::cargo_lock::CargoLockParser;
pub use self::composer::{ComposerJsonParser, ComposerLockParser};
pub use self::dart::{PubspecLockParser, PubspecYamlParser};
pub use self::go::{GoModParser, GoSumParser, GodepsParser};
pub use self::maven::MavenParser;
pub use self::npm::NpmParser;
pub use self::npm_lock::NpmLockParser;
pub use self::npm_workspace::NpmWorkspaceParser;
pub use self::nuget::{NupkgParser, NuspecParser, PackagesConfigParser, PackagesLockParser};
pub use self::pipfile_lock::PipfileLockParser;
pub use self::pnpm_lock::PnpmLockParser;
pub use self::poetry_lock::PoetryLockParser;
pub use self::python::PythonParser;
pub use self::requirements_txt::RequirementsTxtParser;
pub use self::ruby::{GemArchiveParser, GemfileLockParser, GemfileParser, GemspecParser};
pub use self::yarn_lock::YarnLockParser;

macro_rules! define_parsers {
    ($($parser:ty),* $(,)?) => {
        pub fn try_parse_file(path: &Path) -> Option<Vec<PackageData>> {
            $(
                if <$parser>::is_match(path) {
                    return Some(vec![<$parser>::extract_package_data(path)]);
                }
            )*
            None
        }
    };
}

define_parsers! {
    NpmWorkspaceParser,
    NpmParser,
    NpmLockParser,
    YarnLockParser,
    PnpmLockParser,
    PoetryLockParser,
    PipfileLockParser,
    RequirementsTxtParser,
    ComposerJsonParser,
    ComposerLockParser,
    CargoParser,
    PubspecYamlParser,
    PubspecLockParser,
    PythonParser,
    MavenParser,
    GemfileParser,
    GemfileLockParser,
    GemspecParser,
    GemArchiveParser,
    GoModParser,
    GoSumParser,
    GodepsParser,
    PackagesConfigParser,
    NuspecParser,
    PackagesLockParser,
    NupkgParser,
}
