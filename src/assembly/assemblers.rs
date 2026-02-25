use crate::models::DatasourceId;

use super::{AssemblerConfig, AssemblyMode};

pub static ASSEMBLERS: &[AssemblerConfig] = &[
    // ── Sibling-merge assemblers ──
    //
    // npm ecosystem: package.json + lockfiles in same directory.
    // NOTE: npm-shrinkwrap.json emits "npm_package_lock_json" as its datasource_id,
    // so "npm_shrinkwrap_json" is NOT a real datasource_id.
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::NpmPackageJson,
            DatasourceId::NpmPackageLockJson,
            DatasourceId::YarnLock,
            DatasourceId::PnpmLockYaml,
            DatasourceId::PnpmWorkspaceYaml,
        ],
        sibling_file_patterns: &[
            "package.json",
            "package-lock.json",
            "npm-shrinkwrap.json",
            "yarn.lock",
            "pnpm-lock.yaml",
            "pnpm-workspace.yaml",
        ],
        mode: AssemblyMode::SiblingMerge,
    },
    // Rust/Cargo ecosystem
    AssemblerConfig {
        datasource_ids: &[DatasourceId::CargoToml, DatasourceId::CargoLock],
        sibling_file_patterns: &["Cargo.toml", "Cargo.lock"],
        mode: AssemblyMode::SiblingMerge,
    },
    // CocoaPods ecosystem
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::CocoapodsPodspec,
            DatasourceId::CocoapodsPodspecJson,
            DatasourceId::CocoapodsPodfile,
            DatasourceId::CocoapodsPodfileLock,
        ],
        sibling_file_patterns: &["*.podspec", "*.podspec.json", "Podfile", "Podfile.lock"],
        mode: AssemblyMode::SiblingMerge,
    },
    // PHP Composer ecosystem
    AssemblerConfig {
        datasource_ids: &[DatasourceId::PhpComposerJson, DatasourceId::PhpComposerLock],
        sibling_file_patterns: &["composer.json", "composer.lock"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Go ecosystem (includes legacy Godeps)
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::GoMod,
            DatasourceId::GoSum,
            DatasourceId::Godeps,
        ],
        sibling_file_patterns: &["go.mod", "go.sum", "Godeps.json"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Dart/Flutter ecosystem
    AssemblerConfig {
        datasource_ids: &[DatasourceId::PubspecYaml, DatasourceId::PubspecLock],
        sibling_file_patterns: &["pubspec.yaml", "pubspec.lock"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Chef ecosystem
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::ChefCookbookMetadataJson,
            DatasourceId::ChefCookbookMetadataRb,
        ],
        sibling_file_patterns: &["metadata.json", "metadata.rb"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Conan (C/C++) ecosystem
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::ConanConanFilePy,
            DatasourceId::ConanConanFileTxt,
            DatasourceId::ConanLock,
            DatasourceId::ConanConanDataYml,
        ],
        sibling_file_patterns: &[
            "conanfile.py",
            "conanfile.txt",
            "conan.lock",
            "conandata.yml",
        ],
        mode: AssemblyMode::SiblingMerge,
    },
    // Maven/Java ecosystem (nested merge via META-INF)
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::MavenPom,
            DatasourceId::JavaJarManifest,
            DatasourceId::JavaOsgiManifest,
        ],
        sibling_file_patterns: &["pom.xml", "**/META-INF/MANIFEST.MF"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Python/PyPI ecosystem
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::PypiPyprojectToml,
            DatasourceId::PypiSetupPy,
            DatasourceId::PypiSetupCfg,
            DatasourceId::PypiWheel,
            DatasourceId::PypiWheelMetadata,
            DatasourceId::PypiEgg,
            DatasourceId::PypiSdistPkginfo,
            DatasourceId::PypiInspectDeplock,
            DatasourceId::PipRequirements,
            DatasourceId::PypiPoetryLock,
            DatasourceId::Pipfile,
            DatasourceId::PipfileLock,
        ],
        sibling_file_patterns: &[
            "pyproject.toml",
            "setup.py",
            "setup.cfg",
            "requirements*.txt",
            "Pipfile",
            "Pipfile.lock",
            "poetry.lock",
        ],
        mode: AssemblyMode::SiblingMerge,
    },
    // Ruby/RubyGems ecosystem
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::Gemspec,
            DatasourceId::Gemfile,
            DatasourceId::GemfileLock,
            DatasourceId::GemArchive,
        ],
        sibling_file_patterns: &["*.gemspec", "Gemfile", "Gemfile.lock"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Conda ecosystem
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::CondaMetaYaml,
            DatasourceId::CondaYaml,
            DatasourceId::CondaMetaJson,
        ],
        sibling_file_patterns: &["meta.yaml", "environment.yml"],
        mode: AssemblyMode::SiblingMerge,
    },
    // RPM specfile (source packages)
    AssemblerConfig {
        datasource_ids: &[DatasourceId::RpmSpecfile],
        sibling_file_patterns: &["*.spec"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Debian source packages (nested merge via debian/ directory)
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::DebianControlInSource,
            DatasourceId::DebianCopyright,
        ],
        sibling_file_patterns: &["**/debian/control", "**/debian/copyright"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Gradle/Android ecosystem
    AssemblerConfig {
        datasource_ids: &[DatasourceId::BuildGradle, DatasourceId::GradleLockfile],
        sibling_file_patterns: &["build.gradle", "build.gradle.kts", "gradle.lockfile"],
        mode: AssemblyMode::SiblingMerge,
    },
    // CPAN/Perl ecosystem
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::CpanMetaJson,
            DatasourceId::CpanMetaYml,
            DatasourceId::CpanManifest,
            DatasourceId::CpanDistIni,
            DatasourceId::CpanMakefile,
        ],
        sibling_file_patterns: &[
            "META.json",
            "META.yml",
            "MANIFEST",
            "dist.ini",
            "Makefile.PL",
        ],
        mode: AssemblyMode::SiblingMerge,
    },
    // NuGet/.NET ecosystem
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::NugetNuspec,
            DatasourceId::NugetNupkg,
            DatasourceId::NugetPackagesConfig,
            DatasourceId::NugetPackagesLock,
        ],
        sibling_file_patterns: &[
            "*.nuspec",
            "*.nupkg",
            "packages.config",
            "packages.lock.json",
        ],
        mode: AssemblyMode::SiblingMerge,
    },
    // Swift/SPM ecosystem
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::SwiftPackageManifestJson,
            DatasourceId::SwiftPackageResolved,
            DatasourceId::SwiftPackageShowDependencies,
        ],
        sibling_file_patterns: &["Package.swift", "Package.resolved"],
        mode: AssemblyMode::SiblingMerge,
    },
    // ── Standalone assemblers (single file → single package) ──
    //
    // These ecosystems have only one manifest file type with no sibling merging.
    // They still need configs so their datasource_ids are recognized by the assembler.
    //
    // Bower (JavaScript)
    AssemblerConfig {
        datasource_ids: &[DatasourceId::BowerJson],
        sibling_file_patterns: &["bower.json"],
        mode: AssemblyMode::SiblingMerge,
    },
    // CRAN (R language)
    AssemblerConfig {
        datasource_ids: &[DatasourceId::CranDescription],
        sibling_file_patterns: &["DESCRIPTION"],
        mode: AssemblyMode::SiblingMerge,
    },
    // FreeBSD packages
    AssemblerConfig {
        datasource_ids: &[DatasourceId::FreebsdCompactManifest],
        sibling_file_patterns: &["+COMPACT_MANIFEST"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Haxe ecosystem
    AssemblerConfig {
        datasource_ids: &[DatasourceId::HaxelibJson],
        sibling_file_patterns: &["haxelib.json"],
        mode: AssemblyMode::SiblingMerge,
    },
    // OCaml/opam ecosystem
    AssemblerConfig {
        datasource_ids: &[DatasourceId::OpamFile],
        sibling_file_patterns: &["opam"],
        mode: AssemblyMode::SiblingMerge,
    },
    // RPM Mariner manifest
    AssemblerConfig {
        datasource_ids: &[DatasourceId::RpmMarinerManifest],
        sibling_file_patterns: &["*.rpm.manifest"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Microsoft Update Manifest
    AssemblerConfig {
        datasource_ids: &[DatasourceId::MicrosoftUpdateManifestMum],
        sibling_file_patterns: &["*.mum"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Autotools (C/C++ build system)
    AssemblerConfig {
        datasource_ids: &[DatasourceId::AutotoolsConfigure],
        sibling_file_patterns: &["configure", "configure.ac"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Bazel (build system)
    AssemblerConfig {
        datasource_ids: &[DatasourceId::BazelBuild],
        sibling_file_patterns: &["BUILD"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Buck (build system)
    AssemblerConfig {
        datasource_ids: &[DatasourceId::BuckFile, DatasourceId::BuckMetadata],
        sibling_file_patterns: &["BUCK", ".buckconfig"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Ant/Ivy (Java dependency management)
    AssemblerConfig {
        datasource_ids: &[DatasourceId::AntIvyXml],
        sibling_file_patterns: &["ivy.xml"],
        mode: AssemblyMode::SiblingMerge,
    },
    // Meteor (JavaScript platform)
    AssemblerConfig {
        datasource_ids: &[DatasourceId::MeteorPackage],
        sibling_file_patterns: &["package.js"],
        mode: AssemblyMode::SiblingMerge,
    },
    // ── One-per-PackageData assemblers (database files with many packages) ──
    //
    // Alpine installed package database
    AssemblerConfig {
        datasource_ids: &[DatasourceId::AlpineInstalledDb],
        sibling_file_patterns: &["installed"],
        mode: AssemblyMode::OnePerPackageData,
    },
    // RPM installed package databases (BDB, NDB, SQLite)
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::RpmInstalledDatabaseBdb,
            DatasourceId::RpmInstalledDatabaseNdb,
            DatasourceId::RpmInstalledDatabaseSqlite,
        ],
        sibling_file_patterns: &["Packages", "Packages.db", "rpmdb.sqlite"],
        mode: AssemblyMode::OnePerPackageData,
    },
    // Debian installed package databases
    AssemblerConfig {
        datasource_ids: &[
            DatasourceId::DebianInstalledStatusDb,
            DatasourceId::DebianDistrolessInstalledDb,
        ],
        sibling_file_patterns: &["status"],
        mode: AssemblyMode::OnePerPackageData,
    },
];

/// Datasource IDs that are intentionally NOT assembled.
///
/// These are either:
/// - Non-package metadata (readme, about, os_release)
/// - Binary archives (require external extraction via ExtractCode before scanning)
/// - Supplementary metadata files (not primary package definitions)
///
/// This list serves as documentation; it is not used at runtime.
#[allow(dead_code)] // used only in tests (test_every_datasource_id_is_accounted_for)
pub static UNASSEMBLED_DATASOURCE_IDS: &[DatasourceId] = &[
    // Non-package metadata
    DatasourceId::Readme,
    DatasourceId::AboutFile,
    DatasourceId::EtcOsRelease,
    DatasourceId::Gitmodules,
    // Binary archives (require external extraction via ExtractCode before scanning)
    DatasourceId::AlpineApkArchive,
    DatasourceId::AndroidAarLibrary,
    DatasourceId::AndroidApk,
    DatasourceId::AppleDmg,
    DatasourceId::Axis2Mar,
    DatasourceId::ChromeCrx,
    DatasourceId::DebianDeb,
    DatasourceId::DebianOriginalSourceTarball,
    DatasourceId::DebianSourceMetadataTarball,
    DatasourceId::InstallshieldInstaller,
    DatasourceId::IosIpa,
    DatasourceId::IsoDiskImage,
    DatasourceId::JavaEarArchive,
    DatasourceId::JavaJar,
    DatasourceId::JavaWarArchive,
    DatasourceId::JbossSar,
    DatasourceId::MicrosoftCabinet,
    DatasourceId::MozillaXpi,
    DatasourceId::NsisInstaller,
    DatasourceId::RpmArchive,
    DatasourceId::SharShellArchive,
    DatasourceId::SquashfsDiskImage,
    // Supplementary metadata (not primary package definitions)
    DatasourceId::Axis2ModuleXml,
    DatasourceId::DebianControlExtractedDeb,
    DatasourceId::DebianInstalledFilesList,
    DatasourceId::DebianInstalledMd5Sums,
    DatasourceId::DebianMd5SumsInExtractedDeb,
    DatasourceId::DebianSourceControlDsc,
    DatasourceId::GemArchiveExtracted,
    DatasourceId::JavaEarApplicationXml,
    DatasourceId::JavaWarWebXml,
    DatasourceId::JbossServiceXml,
    DatasourceId::RpmPackageLicenses,
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use strum::IntoEnumIterator;

    #[test]
    fn test_every_datasource_id_is_accounted_for() {
        let mut assembled: HashSet<DatasourceId> = HashSet::new();
        for config in ASSEMBLERS {
            for &dsid in config.datasource_ids {
                assembled.insert(dsid);
            }
        }

        let unassembled: HashSet<DatasourceId> =
            UNASSEMBLED_DATASOURCE_IDS.iter().copied().collect();

        let overlap: Vec<_> = assembled.intersection(&unassembled).collect();
        assert!(
            overlap.is_empty(),
            "Datasource IDs in BOTH ASSEMBLERS and UNASSEMBLED: {overlap:?}"
        );

        let missing: Vec<_> = DatasourceId::iter()
            .filter(|dsid| !assembled.contains(dsid) && !unassembled.contains(dsid))
            .collect();

        assert!(
            missing.is_empty(),
            "Datasource IDs in NEITHER ASSEMBLERS nor UNASSEMBLED: {missing:?}\n\
             Add each to an AssemblerConfig in ASSEMBLERS, or to UNASSEMBLED_DATASOURCE_IDS."
        );
    }
}
