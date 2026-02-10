mod about;
#[cfg(test)]
mod about_test;
mod alpine;
mod alpine_golden_test;
mod autotools;
#[cfg(test)]
mod autotools_test;
mod bazel;
#[cfg(test)]
mod bazel_test;
mod bower;
#[cfg(test)]
mod bower_test;
mod buck;
#[cfg(test)]
mod buck_test;
mod cargo;
mod cargo_lock;
#[cfg(test)]
mod cargo_lock_test;
#[cfg(test)]
mod cargo_test;
mod chef;
#[cfg(test)]
mod chef_test;
#[cfg(test)]
mod cocoapods_golden_test;
mod composer;
#[cfg(test)]
mod composer_golden_test;
#[cfg(test)]
mod composer_test;
mod conan;
mod conan_data;
#[cfg(test)]
mod conan_data_test;
#[cfg(test)]
mod conan_test;
mod conda;
mod conda_meta_json;
#[cfg(test)]
mod conda_meta_json_test;
#[cfg(test)]
mod conda_test;
mod cpan;
mod cpan_dist_ini;
#[cfg(test)]
mod cpan_dist_ini_test;
mod cpan_makefile_pl;
#[cfg(test)]
mod cpan_makefile_pl_test;
#[cfg(test)]
mod cpan_test;
mod cran;
#[cfg(test)]
mod cran_golden_test;
#[cfg(test)]
mod cran_test;
mod dart;
#[cfg(test)]
mod dart_golden_test;
#[cfg(test)]
mod dart_test;
mod debian;
mod debian_golden_test;
#[cfg(test)]
mod debian_test;
mod freebsd;
#[cfg(test)]
mod freebsd_test;
mod go;
#[cfg(test)]
mod go_golden_test;
#[cfg(test)]
mod go_test;
mod gradle;
#[cfg(test)]
mod gradle_golden_test;
mod gradle_lock;
#[cfg(test)]
mod gradle_lock_test;
mod haxe;
#[cfg(test)]
mod haxe_golden_test;
#[cfg(test)]
mod haxe_test;
mod maven;
#[cfg(test)]
mod maven_test;
pub mod metadata;
mod microsoft_update_manifest;
#[cfg(test)]
mod microsoft_update_manifest_test;
mod misc;
#[cfg(test)]
mod misc_test;
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
mod opam;
#[cfg(test)]
mod opam_golden_test;
mod os_release;
#[cfg(test)]
mod os_release_test;
#[cfg(test)]
mod osgi_test;
mod pep508;
mod pip_inspect_deplock;
#[cfg(test)]
mod pip_inspect_deplock_test;
mod pipfile_lock;
#[cfg(test)]
mod pipfile_lock_test;
mod pnpm_lock;
#[cfg(test)]
mod pnpm_lock_test;
mod podfile;
mod podfile_lock;
#[cfg(test)]
mod podfile_lock_test;
mod podspec;
mod podspec_json;
#[cfg(test)]
mod podspec_json_test;
mod poetry_lock;
#[cfg(test)]
mod poetry_lock_test;
mod python;
#[cfg(test)]
mod python_test;
mod readme;
#[cfg(test)]
mod readme_test;
mod requirements_txt;
#[cfg(test)]
mod requirements_txt_test;
pub(crate) mod rfc822;
mod rpm_db;
mod rpm_golden_test;
mod rpm_license_files;
#[cfg(test)]
mod rpm_license_files_test;
mod rpm_mariner_manifest;
#[cfg(test)]
mod rpm_mariner_manifest_test;
mod rpm_parser;
mod rpm_specfile;
#[cfg(test)]
mod rpm_specfile_test;
mod ruby;
#[cfg(test)]
mod ruby_golden_test;
#[cfg(test)]
mod ruby_test;
#[cfg(test)]
mod swift_golden_test;
mod swift_manifest_json;
#[cfg(test)]
mod swift_manifest_json_test;
mod swift_resolved;
#[cfg(test)]
mod swift_resolved_test;
mod swift_show_dependencies;
#[cfg(test)]
mod swift_show_dependencies_test;
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
/// - `extract_packages()`: Parses the file and returns all extracted package metadata
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
///     fn extract_packages(path: &Path) -> Vec<PackageData> {
///         // Parse file and return metadata
///         // On error, log warning and return default
///         vec![PackageData::default()]
///     }
/// }
/// ```
pub trait PackageParser {
    /// Package URL type identifier for this parser (e.g., "npm", "pypi", "maven").
    const PACKAGE_TYPE: &'static str;

    /// Extracts all packages from the given file path.
    ///
    /// Returns a vector of `PackageData` structures containing all extracted metadata
    /// including name, version, dependencies, licenses, etc. Most parsers return a
    /// single-element vector, but some (e.g., Bazel BUILD, Buck BUCK, Debian control)
    /// can contain multiple packages in a single file.
    ///
    /// On parse errors, returns a vector with a default `PackageData` with minimal or
    /// no fields populated.
    fn extract_packages(path: &Path) -> Vec<PackageData>;

    /// Checks if the given file path matches this parser's expected format.
    ///
    /// Returns true if the file should be handled by this parser based on filename,
    /// extension, or path patterns. Used by the scanner to route files to appropriate parsers.
    fn is_match(path: &Path) -> bool;

    /// Returns the first package from [`extract_packages()`](Self::extract_packages),
    /// or a default [`PackageData`] if the file contains no packages.
    fn extract_first_package(path: &Path) -> PackageData {
        Self::extract_packages(path)
            .into_iter()
            .next()
            .unwrap_or_default()
    }
}

pub use self::about::AboutFileParser;
pub use self::alpine::{AlpineApkParser, AlpineInstalledParser};
pub use self::autotools::AutotoolsConfigureParser;
pub use self::bazel::BazelBuildParser;
pub use self::bower::BowerJsonParser;
pub use self::buck::{BuckBuildParser, BuckMetadataBzlParser};
pub use self::cargo::CargoParser;
#[cfg_attr(not(test), allow(unused_imports))]
pub use self::cargo_lock::CargoLockParser;
pub use self::chef::{ChefMetadataJsonParser, ChefMetadataRbParser};
pub use self::composer::{ComposerJsonParser, ComposerLockParser};
pub use self::conan::{ConanFilePyParser, ConanLockParser, ConanfileTxtParser};
pub use self::conan_data::ConanDataParser;
pub use self::conda::{CondaEnvironmentYmlParser, CondaMetaYamlParser};
pub use self::conda_meta_json::CondaMetaJsonParser;
pub use self::cpan::{CpanManifestParser, CpanMetaJsonParser, CpanMetaYmlParser};
pub use self::cpan_dist_ini::CpanDistIniParser;
pub use self::cpan_makefile_pl::CpanMakefilePlParser;
pub use self::cran::CranParser;
pub use self::dart::{PubspecLockParser, PubspecYamlParser};
pub use self::debian::{
    DebianControlParser, DebianCopyrightParser, DebianDebParser, DebianDebianTarParser,
    DebianDistrolessInstalledParser, DebianDscParser, DebianInstalledListParser,
    DebianInstalledMd5sumsParser, DebianInstalledParser, DebianMd5sumInPackageParser,
    DebianOrigTarParser,
};
pub use self::freebsd::FreebsdCompactManifestParser;
pub use self::go::{GoModParser, GoSumParser, GodepsParser};
pub use self::gradle::GradleParser;
pub use self::gradle_lock::GradleLockfileParser;
pub use self::haxe::HaxeParser;
pub use self::maven::MavenParser;
pub use self::microsoft_update_manifest::MicrosoftUpdateManifestParser;
pub use self::misc::{
    AndroidLibraryRecognizer, AppleDmgRecognizer, Axis2MarRecognizer, Axis2ModuleXmlRecognizer,
    CabArchiveRecognizer, ChromeCrxRecognizer, IosIpaRecognizer, IsoImageRecognizer,
    IvyXmlRecognizer, JBossSarRecognizer, JBossServiceXmlRecognizer, JavaEarAppXmlRecognizer,
    JavaEarRecognizer, JavaJarRecognizer, JavaWarRecognizer, JavaWarWebXmlRecognizer,
    MeteorPackageRecognizer, MozillaXpiRecognizer, SharArchiveRecognizer,
};
pub use self::npm::NpmParser;
pub use self::npm_lock::NpmLockParser;
pub use self::npm_workspace::NpmWorkspaceParser;
pub use self::nuget::{NupkgParser, NuspecParser, PackagesConfigParser, PackagesLockParser};
pub use self::opam::OpamParser;
pub use self::os_release::OsReleaseParser;
pub use self::pip_inspect_deplock::PipInspectDeplockParser;
pub use self::pipfile_lock::PipfileLockParser;
pub use self::pnpm_lock::PnpmLockParser;
pub use self::podfile::PodfileParser;
pub use self::podfile_lock::PodfileLockParser;
pub use self::podspec::PodspecParser;
pub use self::podspec_json::PodspecJsonParser;
pub use self::poetry_lock::PoetryLockParser;
pub use self::python::PythonParser;
pub use self::readme::ReadmeParser;
pub use self::requirements_txt::RequirementsTxtParser;
pub use self::rpm_db::{RpmBdbDatabaseParser, RpmNdbDatabaseParser, RpmSqliteDatabaseParser};
pub use self::rpm_license_files::RpmLicenseFilesParser;
pub use self::rpm_mariner_manifest::RpmMarinerManifestParser;
pub use self::rpm_parser::RpmParser;
pub use self::rpm_specfile::RpmSpecfileParser;
pub use self::ruby::{
    GemArchiveParser, GemMetadataExtractedParser, GemfileLockParser, GemfileParser, GemspecParser,
};
pub use self::swift_manifest_json::SwiftManifestJsonParser;
pub use self::swift_resolved::SwiftPackageResolvedParser;
pub use self::swift_show_dependencies::SwiftShowDependenciesParser;
pub use self::yarn_lock::YarnLockParser;

macro_rules! define_parsers {
    ($($parser:ty),* $(,)?) => {
        pub fn try_parse_file(path: &Path) -> Option<Vec<PackageData>> {
            $(
                if <$parser>::is_match(path) {
                    return Some(<$parser>::extract_packages(path));
                }
            )*
            None
        }

        #[allow(dead_code)] // Used by bin/generate_test_expected.rs, not library code
        pub fn parse_by_type_name(type_name: &str, path: &Path) -> Option<PackageData> {
            match type_name {
                $(
                    stringify!($parser) => Some(<$parser>::extract_first_package(path)),
                )*
                _ => None
            }
        }

        #[allow(dead_code)] // Used by bin/generate_test_expected.rs and tests/scanner_integration.rs
        pub fn list_parser_types() -> Vec<&'static str> {
            vec![
                $(
                    stringify!($parser),
                )*
            ]
        }
    };
}

define_parsers! {
    AboutFileParser,
    AlpineApkParser,
    AlpineInstalledParser,
    AutotoolsConfigureParser,
    BazelBuildParser,
    BowerJsonParser,
    BuckBuildParser,
    BuckMetadataBzlParser,
    CargoLockParser,
    CargoParser,
    ChefMetadataJsonParser,
    ChefMetadataRbParser,
    ComposerJsonParser,
    ComposerLockParser,
    ConanDataParser,
    ConanFilePyParser,
    ConanfileTxtParser,
    ConanLockParser,
    CondaEnvironmentYmlParser,
    CondaMetaJsonParser,
    CondaMetaYamlParser,
    CpanDistIniParser,
    CpanMakefilePlParser,
    CpanManifestParser,
    CpanMetaJsonParser,
    CpanMetaYmlParser,
    CranParser,
    DebianControlParser,
    DebianCopyrightParser,
    DebianDebianTarParser,
    DebianDebParser,
    DebianDistrolessInstalledParser,
    DebianDscParser,
    DebianInstalledListParser,
    DebianInstalledMd5sumsParser,
    DebianInstalledParser,
    DebianMd5sumInPackageParser,
    DebianOrigTarParser,
    FreebsdCompactManifestParser,
    GemArchiveParser,
    GemfileLockParser,
    GemfileParser,
    GemMetadataExtractedParser,
    GemspecParser,
    GodepsParser,
    GoModParser,
    GoSumParser,
    GradleLockfileParser,
    GradleParser,
    HaxeParser,
    MavenParser,
    MicrosoftUpdateManifestParser,
    NpmLockParser,
    NpmParser,
    NpmWorkspaceParser,
    NupkgParser,
    NuspecParser,
    OpamParser,
    OsReleaseParser,
    PackagesConfigParser,
    PackagesLockParser,
    PipInspectDeplockParser,
    PipfileLockParser,
    PnpmLockParser,
    PodfileLockParser,
    PodfileParser,
    PodspecJsonParser,
    PodspecParser,
    PoetryLockParser,
    PubspecLockParser,
    PubspecYamlParser,
    PythonParser,
    ReadmeParser,
    RequirementsTxtParser,
    RpmBdbDatabaseParser,
    RpmLicenseFilesParser,
    RpmMarinerManifestParser,
    RpmNdbDatabaseParser,
    RpmParser,
    RpmSpecfileParser,
    RpmSqliteDatabaseParser,
    SwiftManifestJsonParser,
    SwiftPackageResolvedParser,
    SwiftShowDependenciesParser,
    YarnLockParser,
    // File type recognizers (misc) - MUST come last to avoid shadowing real parsers
    JavaJarRecognizer,
    IvyXmlRecognizer,
    JavaWarRecognizer,
    JavaWarWebXmlRecognizer,
    JavaEarRecognizer,
    JavaEarAppXmlRecognizer,
    Axis2ModuleXmlRecognizer,
    Axis2MarRecognizer,
    JBossSarRecognizer,
    JBossServiceXmlRecognizer,
    MeteorPackageRecognizer,
    AndroidLibraryRecognizer,
    MozillaXpiRecognizer,
    ChromeCrxRecognizer,
    IosIpaRecognizer,
    CabArchiveRecognizer,
    SharArchiveRecognizer,
    AppleDmgRecognizer,
    IsoImageRecognizer,
}
