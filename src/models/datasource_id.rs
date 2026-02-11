//! Datasource identifiers for package parsers.
//!
//! Each variant uniquely identifies the type of package data source (file format)
//! that was parsed. These IDs enable the assembly system to intelligently merge
//! related package files.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for the type of package data source (file format).
///
/// Datasource IDs distinguish between different file types within the same ecosystem
/// (e.g., `NpmPackageJson` vs `NpmPackageLockJson`). The assembly system uses these
/// IDs to match packages from related files for merging into a single logical package.
///
/// # Serialization
///
/// Variants serialize to snake_case strings matching the Python reference values.
/// The JSON output is identical to the Python ScanCode Toolkit.
///
/// # Examples
///
/// ```ignore
/// use scancode_rust::models::DatasourceId;
///
/// let id = DatasourceId::NpmPackageJson;
/// assert_eq!(id.as_ref(), "npm_package_json");
/// assert_eq!(id.to_string(), "npm_package_json");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasourceId {
    // ── About/README/OS ──
    AboutFile,
    Readme,
    EtcOsRelease,

    // ── Alpine ──
    AlpineApkArchive,
    AlpineInstalledDb,

    // ── Android ──
    AndroidAarLibrary,
    AndroidApk,

    // ── Apache Axis2 ──
    Axis2Mar,
    Axis2ModuleXml,

    // ── Autotools ──
    AutotoolsConfigure,

    // ── Bazel ──
    BazelBuild,

    // ── Bower ──
    BowerJson,

    // ── Buck ──
    /// Matches Python reference value. More consistent name would be `buck_file`.
    #[serde(rename = "buck_file")]
    BuckFile,
    /// Matches Python reference value. More consistent name would be `buck_metadata`.
    #[serde(rename = "buck_metadata")]
    BuckMetadata,

    // ── Cargo/Rust ──
    CargoLock,
    CargoToml,

    // ── Chef ──
    /// Matches Python reference value.
    #[serde(rename = "chef_cookbook_metadata_json")]
    ChefCookbookMetadataJson,
    /// Matches Python reference value.
    #[serde(rename = "chef_cookbook_metadata_rb")]
    ChefCookbookMetadataRb,

    // ── CocoaPods ──
    CocoapodsPodfile,
    CocoapodsPodfileLock,
    CocoapodsPodspec,
    CocoapodsPodspecJson,

    // ── Conan ──
    #[serde(rename = "conan_conandata_yml")]
    ConanConanDataYml,
    #[serde(rename = "conan_conanfile_py")]
    ConanConanFilePy,
    #[serde(rename = "conan_conanfile_txt")]
    ConanConanFileTxt,
    ConanLock,

    // ── Conda ──
    /// Matches Python reference value.
    #[serde(rename = "conda_yaml")]
    CondaYaml,
    CondaMetaJson,
    CondaMetaYaml,

    // ── CPAN/Perl ──
    CpanDistIni,
    /// Matches Python reference value.
    #[serde(rename = "cpan_makefile")]
    CpanMakefile,
    CpanManifest,
    CpanMetaJson,
    CpanMetaYml,

    // ── CRAN/R ──
    CranDescription,

    // ── Dart/Flutter ──
    PubspecLock,
    PubspecYaml,

    // ── Debian ──
    DebianControlExtractedDeb,
    DebianControlInSource,
    DebianCopyright,
    DebianDeb,
    /// Matches Python reference value.
    #[serde(rename = "debian_source_metadata_tarball")]
    DebianSourceMetadataTarball,
    DebianDistrolessInstalledDb,
    /// Matches Python reference value.
    #[serde(rename = "debian_installed_files_list")]
    DebianInstalledFilesList,
    #[serde(rename = "debian_installed_md5sums")]
    DebianInstalledMd5Sums,
    DebianInstalledStatusDb,
    #[serde(rename = "debian_md5sums_in_extracted_deb")]
    DebianMd5SumsInExtractedDeb,
    /// Matches Python reference value.
    #[serde(rename = "debian_original_source_tarball")]
    DebianOriginalSourceTarball,
    DebianSourceControlDsc,

    // ── FreeBSD ──
    FreebsdCompactManifest,

    // ── Go ──
    Godeps,
    GoMod,
    GoSum,

    // ── Gradle ──
    BuildGradle,
    GradleLockfile,

    // ── Haxe ──
    HaxelibJson,

    // ── Java ──
    AntIvyXml,
    JavaEarApplicationXml,
    JavaEarArchive,
    JavaJar,
    JavaJarManifest,
    JavaOsgiManifest,
    JavaWarArchive,
    JavaWarWebXml,
    JbossSar,
    JbossServiceXml,

    // ── Maven ──
    MavenPom,

    // ── Microsoft ──
    MicrosoftCabinet,
    MicrosoftUpdateManifestMum,

    // ── Mobile/Browser ──
    AppleDmg,
    ChromeCrx,
    IosIpa,
    MozillaXpi,

    // ── Meteor ──
    MeteorPackage,

    // ── npm ──
    NpmPackageJson,
    NpmPackageLockJson,

    // ── NuGet ──
    NugetNupkg,
    NugetPackagesConfig,
    NugetPackagesLock,
    /// Serializes to `"nuget_nupsec"` to match Python reference value (typo in original).
    #[serde(rename = "nuget_nupsec")]
    NugetNuspec,

    // ── OCaml/opam ──
    OpamFile,

    // ── PHP/Composer ──
    PhpComposerJson,
    PhpComposerLock,

    // ── pnpm ──
    PnpmLockYaml,
    PnpmWorkspaceYaml,

    // ── Python/PyPI ──
    Pipfile,
    PipfileLock,
    PipRequirements,
    PypiEgg,
    PypiInspectDeplock,
    PypiPoetryLock,
    PypiPyprojectToml,
    PypiSdistPkginfo,
    PypiSetupCfg,
    PypiSetupPy,
    PypiWheel,
    PypiWheelMetadata,

    // ── RPM ──
    RpmArchive,
    RpmInstalledDatabaseBdb,
    RpmInstalledDatabaseNdb,
    RpmInstalledDatabaseSqlite,
    RpmMarinerManifest,
    RpmPackageLicenses,
    /// Serializes to `"rpm_spefile"` to match Python reference value (typo in original).
    #[serde(rename = "rpm_spefile")]
    RpmSpecfile,

    // ── Ruby/RubyGems ──
    Gemfile,
    GemfileLock,
    GemArchive,
    /// Matches Python reference value.
    #[serde(rename = "gem_archive_extracted")]
    GemArchiveExtracted,
    Gemspec,

    // ── Disk Images/Installers ──
    InstallshieldInstaller,
    IsoDiskImage,
    NsisInstaller,
    SharShellArchive,
    SquashfsDiskImage,

    // ── Swift ──
    SwiftPackageManifestJson,
    SwiftPackageResolved,
    SwiftPackageShowDependencies,

    // ── Yarn ──
    YarnLock,
}

impl DatasourceId {
    /// Returns the string representation of this datasource ID.
    ///
    /// This matches the serialized form used in JSON output.
    pub fn as_str(&self) -> &'static str {
        match self {
            // About/README/OS
            Self::AboutFile => "about_file",
            Self::Readme => "readme",
            Self::EtcOsRelease => "etc_os_release",

            // Alpine
            Self::AlpineApkArchive => "alpine_apk_archive",
            Self::AlpineInstalledDb => "alpine_installed_db",

            // Android
            Self::AndroidAarLibrary => "android_aar_library",
            Self::AndroidApk => "android_apk",

            // Apache Axis2
            Self::Axis2Mar => "axis2_mar",
            Self::Axis2ModuleXml => "axis2_module_xml",

            // Autotools
            Self::AutotoolsConfigure => "autotools_configure",

            // Bazel
            Self::BazelBuild => "bazel_build",

            // Bower
            Self::BowerJson => "bower_json",

            // Buck
            Self::BuckFile => "buck_file",
            Self::BuckMetadata => "buck_metadata",

            // Cargo/Rust
            Self::CargoLock => "cargo_lock",
            Self::CargoToml => "cargo_toml",

            // Chef
            Self::ChefCookbookMetadataJson => "chef_cookbook_metadata_json",
            Self::ChefCookbookMetadataRb => "chef_cookbook_metadata_rb",

            // CocoaPods
            Self::CocoapodsPodfile => "cocoapods_podfile",
            Self::CocoapodsPodfileLock => "cocoapods_podfile_lock",
            Self::CocoapodsPodspec => "cocoapods_podspec",
            Self::CocoapodsPodspecJson => "cocoapods_podspec_json",

            // Conan
            Self::ConanConanDataYml => "conan_conandata_yml",
            Self::ConanConanFilePy => "conan_conanfile_py",
            Self::ConanConanFileTxt => "conan_conanfile_txt",
            Self::ConanLock => "conan_lock",

            // Conda
            Self::CondaYaml => "conda_yaml",
            Self::CondaMetaJson => "conda_meta_json",
            Self::CondaMetaYaml => "conda_meta_yaml",

            // CPAN/Perl
            Self::CpanDistIni => "cpan_dist_ini",
            Self::CpanMakefile => "cpan_makefile",
            Self::CpanManifest => "cpan_manifest",
            Self::CpanMetaJson => "cpan_meta_json",
            Self::CpanMetaYml => "cpan_meta_yml",

            // CRAN/R
            Self::CranDescription => "cran_description",

            // Dart/Flutter
            Self::PubspecLock => "pubspec_lock",
            Self::PubspecYaml => "pubspec_yaml",

            // Debian
            Self::DebianControlExtractedDeb => "debian_control_extracted_deb",
            Self::DebianControlInSource => "debian_control_in_source",
            Self::DebianCopyright => "debian_copyright",
            Self::DebianDeb => "debian_deb",
            Self::DebianSourceMetadataTarball => "debian_source_metadata_tarball",
            Self::DebianDistrolessInstalledDb => "debian_distroless_installed_db",
            Self::DebianInstalledFilesList => "debian_installed_files_list",
            Self::DebianInstalledMd5Sums => "debian_installed_md5sums",
            Self::DebianInstalledStatusDb => "debian_installed_status_db",
            Self::DebianMd5SumsInExtractedDeb => "debian_md5sums_in_extracted_deb",
            Self::DebianOriginalSourceTarball => "debian_original_source_tarball",
            Self::DebianSourceControlDsc => "debian_source_control_dsc",

            // FreeBSD
            Self::FreebsdCompactManifest => "freebsd_compact_manifest",

            // Go
            Self::Godeps => "godeps",
            Self::GoMod => "go_mod",
            Self::GoSum => "go_sum",

            // Gradle
            Self::BuildGradle => "build_gradle",
            Self::GradleLockfile => "gradle_lockfile",

            // Haxe
            Self::HaxelibJson => "haxelib_json",

            // Java
            Self::AntIvyXml => "ant_ivy_xml",
            Self::JavaEarApplicationXml => "java_ear_application_xml",
            Self::JavaEarArchive => "java_ear_archive",
            Self::JavaJar => "java_jar",
            Self::JavaJarManifest => "java_jar_manifest",
            Self::JavaOsgiManifest => "java_osgi_manifest",
            Self::JavaWarArchive => "java_war_archive",
            Self::JavaWarWebXml => "java_war_web_xml",
            Self::JbossSar => "jboss_sar",
            Self::JbossServiceXml => "jboss_service_xml",

            // Maven
            Self::MavenPom => "maven_pom",

            // Microsoft
            Self::MicrosoftCabinet => "microsoft_cabinet",
            Self::MicrosoftUpdateManifestMum => "microsoft_update_manifest_mum",

            // Mobile/Browser
            Self::AppleDmg => "apple_dmg",
            Self::ChromeCrx => "chrome_crx",
            Self::IosIpa => "ios_ipa",
            Self::MozillaXpi => "mozilla_xpi",

            // Meteor
            Self::MeteorPackage => "meteor_package",

            // npm
            Self::NpmPackageJson => "npm_package_json",
            Self::NpmPackageLockJson => "npm_package_lock_json",

            // NuGet
            Self::NugetNupkg => "nuget_nupkg",
            Self::NugetPackagesConfig => "nuget_packages_config",
            Self::NugetPackagesLock => "nuget_packages_lock",
            Self::NugetNuspec => "nuget_nupsec",

            // OCaml/opam
            Self::OpamFile => "opam_file",

            // PHP/Composer
            Self::PhpComposerJson => "php_composer_json",
            Self::PhpComposerLock => "php_composer_lock",

            // pnpm
            Self::PnpmLockYaml => "pnpm_lock_yaml",
            Self::PnpmWorkspaceYaml => "pnpm_workspace_yaml",

            // Python/PyPI
            Self::Pipfile => "pipfile",
            Self::PipfileLock => "pipfile_lock",
            Self::PipRequirements => "pip_requirements",
            Self::PypiEgg => "pypi_egg",
            Self::PypiInspectDeplock => "pypi_inspect_deplock",
            Self::PypiPoetryLock => "pypi_poetry_lock",
            Self::PypiPyprojectToml => "pypi_pyproject_toml",
            Self::PypiSdistPkginfo => "pypi_sdist_pkginfo",
            Self::PypiSetupCfg => "pypi_setup_cfg",
            Self::PypiSetupPy => "pypi_setup_py",
            Self::PypiWheel => "pypi_wheel",
            Self::PypiWheelMetadata => "pypi_wheel_metadata",

            // RPM
            Self::RpmArchive => "rpm_archive",
            Self::RpmInstalledDatabaseBdb => "rpm_installed_database_bdb",
            Self::RpmInstalledDatabaseNdb => "rpm_installed_database_ndb",
            Self::RpmInstalledDatabaseSqlite => "rpm_installed_database_sqlite",
            Self::RpmMarinerManifest => "rpm_mariner_manifest",
            Self::RpmPackageLicenses => "rpm_package_licenses",
            Self::RpmSpecfile => "rpm_spefile",

            // Ruby/RubyGems
            Self::Gemfile => "gemfile",
            Self::GemfileLock => "gemfile_lock",
            Self::GemArchive => "gem_archive",
            Self::GemArchiveExtracted => "gem_archive_extracted",
            Self::Gemspec => "gemspec",

            // Disk Images/Installers
            Self::InstallshieldInstaller => "installshield_installer",
            Self::IsoDiskImage => "iso_disk_image",
            Self::NsisInstaller => "nsis_installer",
            Self::SharShellArchive => "shar_shell_archive",
            Self::SquashfsDiskImage => "squashfs_disk_image",

            // Swift
            Self::SwiftPackageManifestJson => "swift_package_manifest_json",
            Self::SwiftPackageResolved => "swift_package_resolved",
            Self::SwiftPackageShowDependencies => "swift_package_show_dependencies",

            // Yarn
            Self::YarnLock => "yarn_lock",
        }
    }
}

impl AsRef<str> for DatasourceId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for DatasourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let id = DatasourceId::NpmPackageJson;
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#""npm_package_json""#);
    }

    #[test]
    fn test_deserialization() {
        let json = r#""npm_package_json""#;
        let id: DatasourceId = serde_json::from_str(json).unwrap();
        assert_eq!(id, DatasourceId::NpmPackageJson);
    }

    #[test]
    fn test_as_str() {
        assert_eq!(DatasourceId::NpmPackageJson.as_str(), "npm_package_json");
        assert_eq!(DatasourceId::CargoLock.as_str(), "cargo_lock");
        assert_eq!(
            DatasourceId::PypiPyprojectToml.as_str(),
            "pypi_pyproject_toml"
        );
    }

    #[test]
    fn test_display() {
        assert_eq!(DatasourceId::NpmPackageJson.to_string(), "npm_package_json");
    }

    #[test]
    fn test_as_ref() {
        let id = DatasourceId::NpmPackageJson;
        let s: &str = id.as_ref();
        assert_eq!(s, "npm_package_json");
    }

    #[test]
    fn test_python_rename_mappings() {
        // Test the ~12 IDs that changed from our old values to match Python
        assert_eq!(DatasourceId::BuckFile.as_str(), "buck_file");
        assert_eq!(DatasourceId::BuckMetadata.as_str(), "buck_metadata");
        assert_eq!(
            DatasourceId::ChefCookbookMetadataJson.as_str(),
            "chef_cookbook_metadata_json"
        );
        assert_eq!(
            DatasourceId::ChefCookbookMetadataRb.as_str(),
            "chef_cookbook_metadata_rb"
        );
        assert_eq!(DatasourceId::CondaYaml.as_str(), "conda_yaml");
        assert_eq!(DatasourceId::CpanMakefile.as_str(), "cpan_makefile");
        assert_eq!(
            DatasourceId::DebianInstalledFilesList.as_str(),
            "debian_installed_files_list"
        );
        assert_eq!(
            DatasourceId::DebianOriginalSourceTarball.as_str(),
            "debian_original_source_tarball"
        );
        assert_eq!(
            DatasourceId::DebianSourceMetadataTarball.as_str(),
            "debian_source_metadata_tarball"
        );
        assert_eq!(
            DatasourceId::GemArchiveExtracted.as_str(),
            "gem_archive_extracted"
        );
        assert_eq!(DatasourceId::NugetNuspec.as_str(), "nuget_nupsec");
        assert_eq!(DatasourceId::RpmSpecfile.as_str(), "rpm_spefile");
    }
}
