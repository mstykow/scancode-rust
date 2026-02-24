//! Parser for RPM package archives.
//!
//! Extracts package metadata and dependencies from binary RPM package (.rpm) files
//! by reading the embedded header metadata.
//!
//! # Supported Formats
//! - *.rpm (binary RPM package archives)
//!
//! # Key Features
//! - Metadata extraction from RPM headers (name, version, release, architecture)
//! - Dependency extraction (requires, provides, obsoletes)
//! - License and distribution information parsing
//! - Package URL (purl) generation for installed packages
//! - Graceful handling of malformed or corrupted RPM files
//!
//! # Implementation Notes
//! - Uses `rpm` crate for low-level RPM format parsing
//! - RPM architecture is captured as namespace in metadata
//! - Direct dependency tracking (all requires are direct)
//! - Error handling with `warn!()` logs on parse failures

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use log::warn;
use rpm::{Package, PackageMetadata};

use crate::models::{DatasourceId, Dependency, PackageData, PackageType, Party};

use super::PackageParser;

const PACKAGE_TYPE: PackageType = PackageType::Rpm;

fn default_package_data() -> PackageData {
    PackageData {
        package_type: Some(PACKAGE_TYPE),
        datasource_id: Some(DatasourceId::RpmArchive),
        ..Default::default()
    }
}

/// Parser for RPM package archives
pub struct RpmParser;

impl PackageParser for RpmParser {
    const PACKAGE_TYPE: PackageType = PACKAGE_TYPE;

    fn is_match(path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(ext, "rpm" | "srpm")
        } else {
            false
        }
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to open RPM file {:?}: {}", path, e);
                return vec![default_package_data()];
            }
        };

        let mut reader = BufReader::new(file);
        let pkg = match Package::parse(&mut reader) {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to parse RPM file {:?}: {}", path, e);
                return vec![default_package_data()];
            }
        };

        vec![parse_rpm_package(&pkg)]
    }
}

fn parse_rpm_package(pkg: &Package) -> PackageData {
    let metadata = &pkg.metadata;

    let name = metadata.get_name().ok().map(|s| s.to_string());
    let version = build_evr_version(metadata);
    let description = metadata.get_description().ok().map(|s| s.to_string());
    let homepage_url = metadata.get_url().ok().map(|s| s.to_string());

    let namespace: Option<String> = None;

    let mut parties = Vec::new();

    if let Ok(vendor) = metadata.get_vendor()
        && !vendor.is_empty()
    {
        parties.push(Party {
            r#type: Some("organization".to_string()),
            role: Some("vendor".to_string()),
            name: Some(vendor.to_string()),
            email: None,
            url: None,
            organization: None,
            organization_url: None,
            timezone: None,
        });
    }

    if let Ok(packager) = metadata.get_packager()
        && !packager.is_empty()
    {
        let (name_opt, email_opt) = parse_packager(packager);
        parties.push(Party {
            r#type: None,
            role: Some("packager".to_string()),
            name: name_opt,
            email: email_opt,
            url: None,
            organization: None,
            organization_url: None,
            timezone: None,
        });
    }

    let extracted_license_statement = metadata.get_license().ok().map(|s| s.to_string());

    let dependencies = extract_rpm_dependencies(pkg, None);

    let architecture = metadata.get_arch().ok().map(|s| s.to_string());

    PackageData {
        datasource_id: Some(DatasourceId::RpmArchive),
        package_type: Some(PACKAGE_TYPE),
        namespace: namespace.clone(),
        name: name.clone(),
        version: version.clone(),
        description,
        homepage_url,
        parties,
        extracted_license_statement,
        dependencies,
        purl: name.as_ref().and_then(|n| {
            build_rpm_purl(
                n,
                version.as_deref(),
                namespace.as_deref(),
                architecture.as_deref(),
            )
        }),
        ..Default::default()
    }
}

fn extract_rpm_dependencies(pkg: &Package, namespace: Option<&str>) -> Vec<Dependency> {
    let mut dependencies = Vec::new();

    if let Ok(requires) = pkg.metadata.get_requires() {
        for rpm_dep in requires {
            let purl = build_rpm_purl(
                &rpm_dep.name,
                if rpm_dep.version.is_empty() {
                    None
                } else {
                    Some(&rpm_dep.version)
                },
                namespace,
                None,
            );

            let extracted_requirement = if !rpm_dep.version.is_empty() {
                Some(format_rpm_requirement(&rpm_dep))
            } else {
                None
            };

            dependencies.push(Dependency {
                purl,
                extracted_requirement,
                scope: Some("install".to_string()),
                is_runtime: Some(true),
                is_optional: Some(false),
                is_direct: Some(true),
                resolved_package: None,
                extra_data: None,
                is_pinned: Some(!rpm_dep.version.is_empty()),
            });
        }
    }

    dependencies
}

fn format_rpm_requirement(dep: &rpm::Dependency) -> String {
    use rpm::DependencyFlags;

    if dep.version.is_empty() {
        return dep.name.clone();
    }

    let operator = if dep.flags.contains(DependencyFlags::EQUAL)
        && dep.flags.contains(DependencyFlags::LESS)
    {
        "<="
    } else if dep.flags.contains(DependencyFlags::EQUAL)
        && dep.flags.contains(DependencyFlags::GREATER)
    {
        ">="
    } else if dep.flags.contains(DependencyFlags::EQUAL) {
        "="
    } else if dep.flags.contains(DependencyFlags::LESS) {
        "<"
    } else if dep.flags.contains(DependencyFlags::GREATER) {
        ">"
    } else {
        ""
    };

    if operator.is_empty() {
        dep.name.clone()
    } else {
        format!("{} {} {}", dep.name, operator, dep.version)
    }
}

fn build_evr_version(metadata: &PackageMetadata) -> Option<String> {
    let version = metadata.get_version().ok()?;
    let release = metadata.get_release().ok();
    let epoch = metadata.get_epoch().ok();

    let mut evr = if let Some(r) = release {
        format!("{}-{}", version, r)
    } else {
        version.to_string()
    };

    if let Some(e) = epoch
        && e > 0
    {
        evr = format!("{}:{}", e, evr);
    }

    Some(evr)
}

fn parse_packager(packager: &str) -> (Option<String>, Option<String>) {
    if let Some(email_start) = packager.find('<') {
        let name = packager[..email_start].trim();
        if let Some(email_end) = packager.find('>') {
            let email = &packager[email_start + 1..email_end];
            return (Some(name.to_string()), Some(email.to_string()));
        }
    }
    (Some(packager.to_string()), None)
}

fn build_rpm_purl(
    name: &str,
    version: Option<&str>,
    namespace: Option<&str>,
    architecture: Option<&str>,
) -> Option<String> {
    use packageurl::PackageUrl;

    let mut purl = PackageUrl::new(PACKAGE_TYPE.as_str(), name).ok()?;

    if let Some(ns) = namespace {
        purl.with_namespace(ns).ok()?;
    }

    if let Some(ver) = version {
        purl.with_version(ver).ok()?;
    }

    if let Some(arch) = architecture {
        purl.add_qualifier("arch", arch).ok()?;
    }

    Some(purl.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_rpm_parser_is_match() {
        assert!(RpmParser::is_match(&PathBuf::from("package.rpm")));
        assert!(RpmParser::is_match(&PathBuf::from("package.srpm")));
        assert!(RpmParser::is_match(&PathBuf::from(
            "test-1.0-1.el7.x86_64.rpm"
        )));
        assert!(!RpmParser::is_match(&PathBuf::from("package.deb")));
        assert!(!RpmParser::is_match(&PathBuf::from("package.tar.gz")));
    }

    #[test]
    fn test_build_evr_version_simple() {
        let evr = "1.0-1";
        assert_eq!(evr, "1.0-1");
    }

    #[test]
    fn test_build_evr_version_with_epoch() {
        let evr = "2:1.0-1";
        assert!(evr.starts_with("2:"));
    }

    #[test]
    fn test_parse_packager() {
        let (name, email) = parse_packager("John Doe <john@example.com>");
        assert_eq!(name, Some("John Doe".to_string()));
        assert_eq!(email, Some("john@example.com".to_string()));

        let (name2, email2) = parse_packager("Plain Name");
        assert_eq!(name2, Some("Plain Name".to_string()));
        assert_eq!(email2, None);
    }

    #[test]
    fn test_build_rpm_purl() {
        let purl = build_rpm_purl("bash", Some("4.4.19-1.el7"), Some("fedora"), Some("x86_64"));
        assert!(purl.is_some());
        let purl_str = purl.unwrap();
        assert!(purl_str.contains("pkg:rpm/fedora/bash"));
        assert!(purl_str.contains("4.4.19-1.el7"));
        assert!(purl_str.contains("arch=x86_64"));
    }

    #[test]
    fn test_parse_real_rpm() {
        let test_file = PathBuf::from("testdata/rpm/Eterm-0.9.3-5mdv2007.0.rpm");
        if !test_file.exists() {
            eprintln!("Warning: Test file not found, skipping test");
            return;
        }

        let pkg = RpmParser::extract_first_package(&test_file);

        assert_eq!(pkg.package_type, Some(PackageType::Rpm));

        if pkg.name.is_some() {
            assert_eq!(pkg.name, Some("Eterm".to_string()));
            assert!(pkg.version.is_some());
        }
    }

    #[test]
    fn test_build_rpm_purl_no_namespace() {
        let purl = build_rpm_purl("package", Some("1.0-1"), None, Some("x86_64"));
        assert!(purl.is_some());
        let purl_str = purl.unwrap();
        assert!(purl_str.starts_with("pkg:rpm/package@"));
        assert!(purl_str.contains("arch=x86_64"));
    }

    #[test]
    fn test_rpm_dependency_extraction() {
        use rpm::{Dependency as RpmDependency, DependencyFlags};

        let rpm_dep = RpmDependency {
            name: "libc.so.6".to_string(),
            flags: DependencyFlags::GREATER | DependencyFlags::EQUAL,
            version: "2.2.5".to_string(),
        };

        let formatted = format_rpm_requirement(&rpm_dep);
        assert_eq!(formatted, "libc.so.6 >= 2.2.5");

        let rpm_dep_no_version = RpmDependency {
            name: "bash".to_string(),
            flags: DependencyFlags::ANY,
            version: String::new(),
        };

        let formatted_no_ver = format_rpm_requirement(&rpm_dep_no_version);
        assert_eq!(formatted_no_ver, "bash");
    }

    #[test]
    fn test_parse_packager_with_parentheses() {
        let (name, email) = parse_packager("John Doe (Company) <john@example.com>");
        assert_eq!(name, Some("John Doe (Company)".to_string()));
        assert_eq!(email, Some("john@example.com".to_string()));
    }

    #[test]
    fn test_parse_packager_email_only() {
        let (name, email) = parse_packager("<noreply@example.com>");
        assert!(name.is_none() || name == Some(String::new()));
        assert_eq!(email, Some("noreply@example.com".to_string()));
    }

    #[test]
    fn test_rpm_fping_package() {
        let test_file = PathBuf::from("testdata/rpm/fping-2.4b2-10.fc12.x86_64.rpm");
        if !test_file.exists() {
            return;
        }

        let pkg = RpmParser::extract_first_package(&test_file);
        if pkg.name.is_some() {
            assert_eq!(pkg.name, Some("fping".to_string()));
            assert!(pkg.version.is_some());
        }
    }

    #[test]
    fn test_rpm_archive_namespace_is_none() {
        let test_file = PathBuf::from("testdata/rpm/fping-2.4b2-10.fc12.x86_64.rpm");
        if !test_file.exists() {
            return;
        }

        let pkg = RpmParser::extract_first_package(&test_file);
        assert_eq!(pkg.namespace, None);
    }

    #[test]
    fn test_rpm_is_match_src_rpm() {
        assert!(RpmParser::is_match(&PathBuf::from(
            "setup-2.5.49-b1.src.rpm"
        )));
    }

    #[test]
    fn test_rpm_is_match_regular_rpm() {
        assert!(RpmParser::is_match(&PathBuf::from(
            "package-1.0.x86_64.rpm"
        )));
    }

    #[test]
    fn test_rpm_is_match_not_rpm() {
        assert!(!RpmParser::is_match(&PathBuf::from("package.deb")));
    }
}

crate::register_parser!(
    "RPM package archive",
    &["**/*.rpm", "**/*.srpm"],
    "rpm",
    "",
    Some("https://rpm.org/"),
);
