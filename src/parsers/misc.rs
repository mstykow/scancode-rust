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
//!
//! # Skipped Recognizers (Require Magic Byte Detection)
//!
//! The following recognizers from Python's misc.py are intentionally SKIPPED because
//! they require checking file magic bytes, not just extensions:
//!
//! - **InstallShieldRecognizer**: Needs `filetypes=('zip installshield',)` check
//! - **NsisInstallerRecognizer**: Needs `filetypes=('nullsoft installer',)` check
//! - **SquashfsRecognizer**: Needs `filetypes=('squashfs filesystem',)` check
//! - **AndroidApkRecognizer**: Would conflict with AlpineApkParser (both match *.apk)
//!
//! These would create false positives if implemented with extension-only matching.

use std::path::Path;

use super::PackageParser;
use crate::models::PackageData;

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
                    datasource_id: Some($datasource.to_string()),
                    ..Default::default()
                }]
            }
        }
    };
}

// Java Archives

file_recognizer!(JavaJarRecognizer, "jar", "java_jar", |path: &Path| path
    .extension()
    .and_then(|e| e.to_str())
    == Some("jar"));

file_recognizer!(IvyXmlRecognizer, "ivy", "ant_ivy_xml", |path: &Path| path
    .to_str()
    .is_some_and(|p| p.ends_with("/ivy.xml")));

file_recognizer!(
    JavaWarRecognizer,
    "war",
    "java_war_archive",
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("war")
);

file_recognizer!(
    JavaWarWebXmlRecognizer,
    "war",
    "java_war_web_xml",
    |path: &Path| path
        .to_str()
        .is_some_and(|p| p.ends_with("/WEB-INF/web.xml") || p.ends_with("WEB-INF/web.xml"))
);

file_recognizer!(
    JavaEarRecognizer,
    "ear",
    "java_ear_archive",
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("ear")
);

file_recognizer!(
    JavaEarAppXmlRecognizer,
    "ear",
    "java_ear_application_xml",
    |path: &Path| path.to_str().is_some_and(
        |p| p.ends_with("/META-INF/application.xml") || p.ends_with("META-INF/application.xml")
    )
);

// Apache Axis2

file_recognizer!(
    Axis2ModuleXmlRecognizer,
    "axis2",
    "axis2_module_xml",
    |path: &Path| {
        path.to_str().is_some_and(|p| {
            let lower = p.to_lowercase();
            lower.ends_with("/meta-inf/module.xml") || lower.ends_with("meta-inf/module.xml")
        })
    }
);

file_recognizer!(Axis2MarRecognizer, "axis2", "axis2_mar", |path: &Path| path
    .extension()
    .and_then(|e| e.to_str())
    == Some("mar"));

// JBoss

file_recognizer!(
    JBossSarRecognizer,
    "jboss-service",
    "jboss_sar",
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("sar")
);

file_recognizer!(
    JBossServiceXmlRecognizer,
    "jboss-service",
    "jboss_service_xml",
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
    "meteor_package",
    |path: &Path| path.to_str().is_some_and(|p| p.ends_with("/package.js"))
);

// Mobile Apps

file_recognizer!(
    AndroidLibraryRecognizer,
    "android_lib",
    "android_aar_library",
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("aar")
);

file_recognizer!(
    MozillaXpiRecognizer,
    "mozilla",
    "mozilla_xpi",
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("xpi")
);

file_recognizer!(
    ChromeCrxRecognizer,
    "chrome",
    "chrome_crx",
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("crx")
);

file_recognizer!(IosIpaRecognizer, "ios", "ios_ipa", |path: &Path| path
    .extension()
    .and_then(|e| e.to_str())
    == Some("ipa"));

// Archives

file_recognizer!(
    CabArchiveRecognizer,
    "cab",
    "microsoft_cabinet",
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("cab")
);

file_recognizer!(
    SharArchiveRecognizer,
    "shar",
    "shar_shell_archive",
    |path: &Path| path.extension().and_then(|e| e.to_str()) == Some("shar")
);

// Disk Images

file_recognizer!(AppleDmgRecognizer, "dmg", "apple_dmg", |path: &Path| {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext == "dmg" || ext == "sparseimage")
});

file_recognizer!(
    IsoImageRecognizer,
    "iso",
    "iso_disk_image",
    |path: &Path| {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext == "iso" || ext == "udf" || ext == "img")
    }
);

crate::register_parser!(
    "Misc file type recognizers (JAR, WAR, EAR, Android, iOS, Chrome, Mozilla, etc.)",
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
    ],
    "",
    "",
    None,
);
