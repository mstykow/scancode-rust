//! File type recognizers for various package archives and binary formats.
//!
//! This module contains simple file-type recognizers that identify packages by
//! their file extensions or path patterns. These recognizers do NOT parse file
//! contents - they only tag files with the appropriate package_type and datasource_id.
//!
//! # Implementation Notes
//!
//! - All recognizers use the `file_recognizer!` macro to reduce boilerplate
//! - Recognizers return minimal PackageData with only package_type and datasource_id set
//! - These correspond to Python's misc.py NonAssemblableDatafileHandler classes
//! - No actual parsing is performed (Python also has `# TODO: parse me!!!`)
//! - Some recognizers use magic byte detection for disambiguation (Squashfs, NSIS, InstallShield)

use std::path::Path;

use super::PackageParser;
use crate::models::{DatasourceId, PackageData};
use crate::utils::magic;

/// Helper macro to define file-type recognizers with minimal boilerplate.
///
/// Each recognizer matches specific file patterns and returns a minimal
/// PackageData structure with only package_type and datasource_id populated.
///
/// # Arguments
///
/// * `$name` - Struct name for the recognizer
/// * `$pkg_type` - Package type string (e.g., "jar", "war", "meteor")
/// * `$datasource` - Datasource ID string (e.g., "java_jar", "meteor_package")
/// * `$match_fn` - Closure that takes a &Path and returns bool for matching
macro_rules! file_recognizer {
    ($name:ident, $pkg_type:expr, $datasource:expr, $match_fn:expr) => {
        pub struct $name;

        impl PackageParser for $name {
            const PACKAGE_TYPE: &'static str = $pkg_type;

            fn is_match(path: &Path) -> bool {
                ($match_fn)(path)
            }

            fn extract_packages(path: &Path) -> Vec<PackageData> {
                let _ = path;
                vec![PackageData {
                    package_type: Some($pkg_type.to_string()),
                    datasource_id: Some($datasource),
                    ..Default::default()
                }]
            }
        }
    };
}

// Java Archives

file_recognizer!(
    JavaJarRecognizer,
    "jar",
    DatasourceId::JavaJar,
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("jar")
);

file_recognizer!(
    IvyXmlRecognizer,
    "ivy",
    DatasourceId::AntIvyXml,
    |path: &Path| path.to_str().is_some_and(|p| p.ends_with("/ivy.xml"))
);

file_recognizer!(
    JavaWarRecognizer,
    "war",
    DatasourceId::JavaWarArchive,
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("war")
);

file_recognizer!(
    JavaWarWebXmlRecognizer,
    "war",
    DatasourceId::JavaWarWebXml,
    |path: &Path| path
        .to_str()
        .is_some_and(|p| p.ends_with("/WEB-INF/web.xml") || p.ends_with("WEB-INF/web.xml"))
);

file_recognizer!(
    JavaEarRecognizer,
    "ear",
    DatasourceId::JavaEarArchive,
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("ear")
);

file_recognizer!(
    JavaEarAppXmlRecognizer,
    "ear",
    DatasourceId::JavaEarApplicationXml,
    |path: &Path| path.to_str().is_some_and(
        |p| p.ends_with("/META-INF/application.xml") || p.ends_with("META-INF/application.xml")
    )
);

// Apache Axis2

file_recognizer!(
    Axis2ModuleXmlRecognizer,
    "axis2",
    DatasourceId::Axis2ModuleXml,
    |path: &Path| {
        path.to_str().is_some_and(|p| {
            let lower = p.to_lowercase();
            lower.ends_with("/meta-inf/module.xml") || lower.ends_with("meta-inf/module.xml")
        })
    }
);

file_recognizer!(
    Axis2MarRecognizer,
    "axis2",
    DatasourceId::Axis2Mar,
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("mar")
);

// JBoss

file_recognizer!(
    JBossSarRecognizer,
    "jboss-service",
    DatasourceId::JbossSar,
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("sar")
);

file_recognizer!(
    JBossServiceXmlRecognizer,
    "jboss-service",
    DatasourceId::JbossServiceXml,
    |path: &Path| {
        path.to_str().is_some_and(|p| {
            let lower = p.to_lowercase();
            lower.ends_with("/meta-inf/jboss-service.xml")
                || lower.ends_with("meta-inf/jboss-service.xml")
        })
    }
);

// Meteor

file_recognizer!(
    MeteorPackageRecognizer,
    "meteor",
    DatasourceId::MeteorPackage,
    |path: &Path| path.to_str().is_some_and(|p| p.ends_with("/package.js"))
);

// Mobile Apps

file_recognizer!(
    AndroidApkRecognizer,
    "android",
    DatasourceId::AndroidApk,
    |path: &Path| {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext == "apk")
            && magic::is_zip(path)
    }
);

file_recognizer!(
    AndroidLibraryRecognizer,
    "android_lib",
    DatasourceId::AndroidAarLibrary,
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("aar")
);

file_recognizer!(
    MozillaXpiRecognizer,
    "mozilla",
    DatasourceId::MozillaXpi,
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("xpi")
);

file_recognizer!(
    ChromeCrxRecognizer,
    "chrome",
    DatasourceId::ChromeCrx,
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("crx")
);

file_recognizer!(
    IosIpaRecognizer,
    "ios",
    DatasourceId::IosIpa,
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("ipa")
);

// Archives

file_recognizer!(
    CabArchiveRecognizer,
    "cab",
    DatasourceId::MicrosoftCabinet,
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("cab")
);

file_recognizer!(
    SharArchiveRecognizer,
    "shar",
    DatasourceId::SharShellArchive,
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("shar")
);

// Disk Images

file_recognizer!(
    AppleDmgRecognizer,
    "dmg",
    DatasourceId::AppleDmg,
    |path: &Path| {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext == "dmg" || ext == "sparseimage")
    }
);

file_recognizer!(
    IsoImageRecognizer,
    "iso",
    DatasourceId::IsoDiskImage,
    |path: &Path| {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext == "iso" || ext == "udf" || ext == "img")
    }
);

// Installers and Binary Formats (require magic byte detection)

file_recognizer!(
    SquashfsRecognizer,
    "squashfs",
    DatasourceId::SquashfsDiskImage,
    |path: &Path| magic::is_squashfs(path)
);

file_recognizer!(
    NsisRecognizer,
    "nsis",
    DatasourceId::NsisInstaller,
    |path: &Path| {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext == "exe")
            && magic::is_nsis_installer(path)
    }
);

file_recognizer!(
    InstallShieldRecognizer,
    "installshield",
    DatasourceId::InstallshieldInstaller,
    |path: &Path| {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext == "exe")
            && magic::is_zip(path)
    }
);

crate::register_parser!(
    "Misc file type recognizers (JAR, WAR, EAR, Android, iOS, Chrome, Mozilla, installers, disk images, etc.)",
    &[
        "**/*.jar",
        "**/ivy.xml",
        "**/*.war",
        "**/WEB-INF/web.xml",
        "**/*.ear",
        "**/META-INF/application.xml",
        "**/meta-inf/module.xml",
        "**/*.mar",
        "**/*.sar",
        "**/meta-inf/jboss-service.xml",
        "**/package.js",
        "**/*.apk",
        "**/*.aar",
        "**/*.xpi",
        "**/*.crx",
        "**/*.ipa",
        "**/*.cab",
        "**/*.shar",
        "**/*.dmg",
        "**/*.sparseimage",
        "**/*.iso",
        "**/*.udf",
        "**/*.img",
        "**/*.exe",
    ],
    "",
    "",
    None,
);
