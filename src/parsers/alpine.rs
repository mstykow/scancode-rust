//! Parser for Alpine Linux package metadata files.
//!
//! Extracts installed package metadata from Alpine Linux package database files
//! using the APK package manager format.
//!
//! # Supported Formats
//! - `/lib/apk/db/installed` (Installed package database)
//!
//! # Key Features
//! - Installed package metadata extraction from system database
//! - Dependency tracking from provides/requires fields
//! - Author and maintainer information extraction
//! - License information parsing
//! - Package URL (purl) generation
//!
//! # Implementation Notes
//! - Uses RFC 822-like format parsing via `rfc822` module
//! - Database stored in text format with multi-paragraph records
//! - Graceful error handling with `warn!()` logs

use std::collections::HashMap;
use std::path::Path;

use log::warn;

use crate::models::{Dependency, FileReference, PackageData, Party};
use crate::parsers::rfc822;
use crate::parsers::utils::{create_default_package_data, read_file_to_string, split_name_email};

use super::PackageParser;

const PACKAGE_TYPE: &str = "alpine";

/// Parser for Alpine Linux installed package database
pub struct AlpineInstalledParser;

impl PackageParser for AlpineInstalledParser {
    const PACKAGE_TYPE: &'static str = PACKAGE_TYPE;

    fn is_match(path: &Path) -> bool {
        path.to_str()
            .map(|p| p.contains("/lib/apk/db/") && p.ends_with("installed"))
            .unwrap_or(false)
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        let content = match read_file_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read Alpine installed db {:?}: {}", path, e);
                return vec![create_default_package_data(
                    PACKAGE_TYPE,
                    Some("alpine_installed_db"),
                )];
            }
        };

        vec![parse_alpine_installed_db(&content)]
    }
}

fn parse_alpine_installed_db(content: &str) -> PackageData {
    let paragraphs = rfc822::parse_rfc822_paragraphs(content);

    let raw_paragraphs: Vec<&str> = content
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .collect();

    let mut all_packages = Vec::new();

    for (idx, para) in paragraphs.iter().enumerate() {
        let raw_text = raw_paragraphs.get(idx).copied().unwrap_or("");
        let pkg = parse_alpine_package_paragraph(&para.headers, raw_text);
        if pkg.name.is_some() {
            all_packages.push(pkg);
        }
    }

    if all_packages.is_empty() {
        return create_default_package_data(PACKAGE_TYPE, Some("alpine_installed_db"));
    }

    all_packages.into_iter().next().unwrap()
}

fn parse_alpine_package_paragraph(
    headers: &HashMap<String, Vec<String>>,
    raw_text: &str,
) -> PackageData {
    let name = rfc822::get_header_first(headers, "P");
    let version = rfc822::get_header_first(headers, "V");
    let description = rfc822::get_header_first(headers, "T");
    let homepage_url = rfc822::get_header_first(headers, "U");
    let architecture = rfc822::get_header_first(headers, "A");

    let namespace = Some("alpine".to_string());
    let mut parties = Vec::new();

    if let Some(maintainer) = rfc822::get_header_first(headers, "m") {
        let (name_opt, email_opt) = split_name_email(&maintainer);
        parties.push(Party {
            r#type: None,
            role: Some("maintainer".to_string()),
            name: name_opt,
            email: email_opt,
            url: None,
            organization: None,
            organization_url: None,
            timezone: None,
        });
    }

    let extracted_license_statement = rfc822::get_header_first(headers, "L");

    let source_packages = if let Some(origin) = rfc822::get_header_first(headers, "o") {
        vec![format!("pkg:alpine/{}", origin)]
    } else {
        Vec::new()
    };

    let mut dependencies = Vec::new();
    for dep in rfc822::get_header_all(headers, "D") {
        for dep_str in dep.split_whitespace() {
            if dep_str.starts_with("so:") || dep_str.starts_with("cmd:") {
                continue;
            }

            dependencies.push(Dependency {
                purl: Some(format!("pkg:alpine/{}", dep_str)),
                extracted_requirement: None,
                scope: Some("install".to_string()),
                is_runtime: Some(true),
                is_optional: Some(false),
                is_direct: Some(true),
                resolved_package: None,
                extra_data: None,
                is_pinned: Some(false),
            });
        }
    }

    let mut extra_data = HashMap::new();

    if let Some(checksum) = rfc822::get_header_first(headers, "C") {
        extra_data.insert("checksum".to_string(), checksum.into());
    }

    if let Some(size) = rfc822::get_header_first(headers, "S") {
        extra_data.insert("compressed_size".to_string(), size.into());
    }

    if let Some(installed_size) = rfc822::get_header_first(headers, "I") {
        extra_data.insert("installed_size".to_string(), installed_size.into());
    }

    if let Some(timestamp) = rfc822::get_header_first(headers, "t") {
        extra_data.insert("build_timestamp".to_string(), timestamp.into());
    }

    if let Some(commit) = rfc822::get_header_first(headers, "c") {
        extra_data.insert("git_commit".to_string(), commit.into());
    }

    let providers = extract_providers(raw_text);
    if !providers.is_empty() {
        let provider_list: Vec<serde_json::Value> =
            providers.into_iter().map(|s| s.into()).collect();
        extra_data.insert("providers".to_string(), provider_list.into());
    }

    let file_references = extract_file_references(raw_text);

    PackageData {
        datasource_id: Some("alpine_installed_db".to_string()),
        package_type: Some(PACKAGE_TYPE.to_string()),
        namespace: namespace.clone(),
        name: name.clone(),
        version: version.clone(),
        description,
        homepage_url,
        parties,
        extracted_license_statement,
        source_packages,
        dependencies,
        file_references,
        purl: name
            .as_ref()
            .and_then(|n| build_alpine_purl(n, version.as_deref(), architecture.as_deref())),
        extra_data: if extra_data.is_empty() {
            None
        } else {
            Some(extra_data)
        },
        ..Default::default()
    }
}

fn extract_file_references(raw_text: &str) -> Vec<FileReference> {
    let mut file_references = Vec::new();
    let mut current_dir = String::new();
    let mut current_file: Option<FileReference> = None;

    for line in raw_text.lines() {
        if line.is_empty() {
            continue;
        }

        if let Some((field_type, value)) = line.split_once(':') {
            let value = value.trim();
            match field_type {
                "F" => {
                    if let Some(file) = current_file.take() {
                        file_references.push(file);
                    }
                    current_dir = value.to_string();
                }
                "R" => {
                    if let Some(file) = current_file.take() {
                        file_references.push(file);
                    }

                    let path = if current_dir.is_empty() {
                        value.to_string()
                    } else {
                        format!("{}/{}", current_dir, value)
                    };

                    current_file = Some(FileReference {
                        path,
                        size: None,
                        sha1: None,
                        md5: None,
                        sha256: None,
                        sha512: None,
                        extra_data: None,
                    });
                }
                "Z" => {
                    if let Some(ref mut file) = current_file
                        && value.starts_with("Q1")
                    {
                        use base64::Engine;
                        if let Ok(decoded) =
                            base64::engine::general_purpose::STANDARD.decode(&value[2..])
                        {
                            let hex_string = decoded
                                .iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<String>();
                            file.sha1 = Some(hex_string);
                        }
                    }
                }
                "a" => {
                    if let Some(ref mut file) = current_file {
                        let mut extra = HashMap::new();
                        extra.insert(
                            "attributes".to_string(),
                            serde_json::Value::String(value.to_string()),
                        );
                        file.extra_data = Some(extra);
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(file) = current_file {
        file_references.push(file);
    }

    file_references
}

fn extract_providers(raw_text: &str) -> Vec<String> {
    let mut providers = Vec::new();

    for line in raw_text.lines() {
        if line.is_empty() {
            continue;
        }

        if let Some(value) = line.strip_prefix("p:") {
            providers.extend(value.split_whitespace().map(|s| s.to_string()));
        }
    }

    providers
}

fn build_alpine_purl(
    name: &str,
    version: Option<&str>,
    architecture: Option<&str>,
) -> Option<String> {
    use packageurl::PackageUrl;

    let mut purl = PackageUrl::new(PACKAGE_TYPE, name).ok()?;

    if let Some(ver) = version {
        purl.with_version(ver).ok()?;
    }

    if let Some(arch) = architecture {
        purl.add_qualifier("arch", arch).ok()?;
    }

    Some(purl.to_string())
}

/// Parser for Alpine Linux .apk package archives
pub struct AlpineApkParser;

impl PackageParser for AlpineApkParser {
    const PACKAGE_TYPE: &'static str = PACKAGE_TYPE;

    fn is_match(path: &Path) -> bool {
        path.extension().and_then(|e| e.to_str()) == Some("apk")
    }

    fn extract_packages(path: &Path) -> Vec<PackageData> {
        vec![match extract_apk_archive(path) {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to extract .apk archive {:?}: {}", path, e);
                create_default_package_data(PACKAGE_TYPE, Some("alpine_apk_archive"))
            }
        }]
    }
}

fn extract_apk_archive(path: &Path) -> Result<PackageData, String> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|e| format!("Failed to open .apk file: {}", e))?;

    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    for entry_result in archive
        .entries()
        .map_err(|e| format!("Failed to read tar entries: {}", e))?
    {
        let mut entry = entry_result.map_err(|e| format!("Failed to read tar entry: {}", e))?;

        let entry_path = entry
            .path()
            .map_err(|e| format!("Failed to get entry path: {}", e))?;

        if entry_path.ends_with(".PKGINFO") {
            let mut content = String::new();
            entry
                .read_to_string(&mut content)
                .map_err(|e| format!("Failed to read .PKGINFO: {}", e))?;

            return Ok(parse_pkginfo(&content));
        }
    }

    Err(".apk archive does not contain .PKGINFO file".to_string())
}

fn parse_pkginfo(content: &str) -> PackageData {
    let mut fields: HashMap<&str, Vec<&str>> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once(" = ") {
            fields.entry(key.trim()).or_default().push(value.trim());
        }
    }

    let name = fields
        .get("pkgname")
        .and_then(|v| v.first())
        .map(|s| s.to_string());
    let pkgver = fields.get("pkgver").and_then(|v| v.first());
    let version = pkgver.map(|s| s.to_string());
    let arch = fields
        .get("arch")
        .and_then(|v| v.first())
        .map(|s| s.to_string());
    let license = fields
        .get("license")
        .and_then(|v| v.first())
        .map(|s| s.to_string());
    let description = fields
        .get("pkgdesc")
        .and_then(|v| v.first())
        .map(|s| s.to_string());
    let homepage = fields
        .get("url")
        .and_then(|v| v.first())
        .map(|s| s.to_string());
    let origin = fields
        .get("origin")
        .and_then(|v| v.first())
        .map(|s| s.to_string());
    let maintainer_str = fields.get("maintainer").and_then(|v| v.first());

    let mut parties = Vec::new();
    if let Some(maint) = maintainer_str {
        let (maint_name, maint_email) = split_name_email(maint);
        parties.push(Party {
            r#type: Some("person".to_string()),
            role: Some("maintainer".to_string()),
            name: maint_name,
            email: maint_email,
            url: None,
            organization: None,
            organization_url: None,
            timezone: None,
        });
    }

    let purl = name
        .as_ref()
        .and_then(|n| build_alpine_purl(n, version.as_deref(), arch.as_deref()));

    let mut dependencies = Vec::new();
    if let Some(depends_list) = fields.get("depend") {
        for dep_str in depends_list {
            let dep_name = dep_str.split_whitespace().next().unwrap_or(dep_str);
            dependencies.push(Dependency {
                purl: Some(format!("pkg:alpine/{}", dep_name)),
                extracted_requirement: Some(dep_str.to_string()),
                scope: Some("runtime".to_string()),
                is_runtime: Some(true),
                is_optional: Some(false),
                is_pinned: None,
                is_direct: Some(true),
                resolved_package: None,
                extra_data: None,
            });
        }
    }

    PackageData {
        datasource_id: Some("alpine_apk_archive".to_string()),
        package_type: Some(PACKAGE_TYPE.to_string()),
        namespace: Some("alpine".to_string()),
        name,
        version,
        description,
        homepage_url: homepage,
        extracted_license_statement: license,
        parties,
        dependencies,
        purl,
        extra_data: origin.map(|o| {
            let mut map = HashMap::new();
            map.insert("origin".to_string(), serde_json::Value::String(o));
            map
        }),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_alpine_parser_is_match() {
        assert!(AlpineInstalledParser::is_match(&PathBuf::from(
            "/lib/apk/db/installed"
        )));
        assert!(AlpineInstalledParser::is_match(&PathBuf::from(
            "/var/lib/apk/db/installed"
        )));
        assert!(!AlpineInstalledParser::is_match(&PathBuf::from(
            "/lib/apk/db/status"
        )));
        assert!(!AlpineInstalledParser::is_match(&PathBuf::from(
            "installed"
        )));
    }

    #[test]
    fn test_parse_alpine_package_basic() {
        let content = "C:Q1v4QhLje3kWlC8DJj+ZfJTjlJRSU=
P:alpine-baselayout-data
V:3.2.0-r22
A:x86_64
S:11435
I:73728
T:Alpine base dir structure and init scripts
U:https://git.alpinelinux.org/cgit/aports/tree/main/alpine-baselayout
L:GPL-2.0-only
o:alpine-baselayout
m:Natanael Copa <ncopa@alpinelinux.org>
t:1655134784
c:cb70ca5c6d6db0399d2dd09189c5d57827bce5cd

";
        let pkg = parse_alpine_installed_db(content);
        assert_eq!(pkg.name, Some("alpine-baselayout-data".to_string()));
        assert_eq!(pkg.version, Some("3.2.0-r22".to_string()));
        assert_eq!(pkg.namespace, Some("alpine".to_string()));
        assert_eq!(
            pkg.description,
            Some("Alpine base dir structure and init scripts".to_string())
        );
        assert_eq!(
            pkg.homepage_url,
            Some("https://git.alpinelinux.org/cgit/aports/tree/main/alpine-baselayout".to_string())
        );
        assert_eq!(
            pkg.extracted_license_statement,
            Some("GPL-2.0-only".to_string())
        );
        assert_eq!(pkg.parties.len(), 1);
        assert_eq!(pkg.parties[0].name, Some("Natanael Copa".to_string()));
        assert_eq!(
            pkg.parties[0].email,
            Some("ncopa@alpinelinux.org".to_string())
        );
        assert!(
            pkg.purl
                .as_ref()
                .unwrap()
                .contains("alpine-baselayout-data")
        );
        assert!(pkg.purl.as_ref().unwrap().contains("arch=x86_64"));
    }

    #[test]
    fn test_parse_alpine_with_dependencies() {
        let content = "P:musl
V:1.2.3-r0
A:x86_64
D:scanelf so:libc.musl-x86_64.so.1

";
        let pkg = parse_alpine_installed_db(content);
        assert_eq!(pkg.name, Some("musl".to_string()));
        assert_eq!(pkg.dependencies.len(), 1);
        assert!(
            pkg.dependencies[0]
                .purl
                .as_ref()
                .unwrap()
                .contains("scanelf")
        );
    }

    #[test]
    fn test_build_alpine_purl() {
        let purl = build_alpine_purl("busybox", Some("1.31.1-r9"), Some("x86_64"));
        assert_eq!(
            purl,
            Some("pkg:alpine/busybox@1.31.1-r9?arch=x86_64".to_string())
        );

        let purl_no_arch = build_alpine_purl("package", Some("1.0"), None);
        assert_eq!(purl_no_arch, Some("pkg:alpine/package@1.0".to_string()));
    }

    #[test]
    fn test_parse_alpine_extra_data() {
        let content = "P:test-package
V:1.0
C:base64checksum==
S:12345
I:67890
t:1234567890
c:gitcommithash

";
        let pkg = parse_alpine_installed_db(content);
        assert!(pkg.extra_data.is_some());
        let extra = pkg.extra_data.unwrap();
        assert!(extra.contains_key("checksum"));
        assert!(extra.contains_key("compressed_size"));
        assert!(extra.contains_key("installed_size"));
        assert!(extra.contains_key("build_timestamp"));
        assert!(extra.contains_key("git_commit"));
    }

    #[test]
    fn test_parse_alpine_multiple_packages() {
        let content = "P:package1
V:1.0
A:x86_64

P:package2
V:2.0
A:aarch64

";
        let pkg = parse_alpine_installed_db(content);
        assert_eq!(pkg.name, Some("package1".to_string()));
    }

    #[test]
    fn test_parse_alpine_file_references() {
        let content = "P:test-pkg
V:1.0
F:usr/bin
R:test
Z:Q1WTc55xfvPogzA0YUV24D0Ym+MKE=
F:etc
R:config
Z:Q1pcfTfDNEbNKQc2s1tia7da05M8Q=

";
        let pkg = parse_alpine_installed_db(content);
        assert_eq!(pkg.file_references.len(), 2);
        assert_eq!(pkg.file_references[0].path, "usr/bin/test");
        assert!(pkg.file_references[0].sha1.is_some());
        assert_eq!(pkg.file_references[1].path, "etc/config");
        assert!(pkg.file_references[1].sha1.is_some());
    }

    #[test]
    fn test_parse_alpine_empty_fields() {
        let content = "P:minimal-package
V:1.0

";
        let pkg = parse_alpine_installed_db(content);
        assert_eq!(pkg.name, Some("minimal-package".to_string()));
        assert_eq!(pkg.version, Some("1.0".to_string()));
        assert!(pkg.description.is_none());
        assert!(pkg.homepage_url.is_none());
        assert_eq!(pkg.dependencies.len(), 0);
    }

    #[test]
    fn test_parse_alpine_origin_field() {
        let content = "P:busybox-ifupdown
V:1.35.0-r13
o:busybox
A:x86_64

";
        let pkg = parse_alpine_installed_db(content);
        assert_eq!(pkg.name, Some("busybox-ifupdown".to_string()));
        assert_eq!(pkg.source_packages.len(), 1);
        assert_eq!(pkg.source_packages[0], "pkg:alpine/busybox");
    }

    #[test]
    fn test_parse_alpine_url_field() {
        let content = "P:openssl
V:1.1.1q-r0
U:https://www.openssl.org
A:x86_64

";
        let pkg = parse_alpine_installed_db(content);
        assert_eq!(
            pkg.homepage_url,
            Some("https://www.openssl.org".to_string())
        );
    }

    #[test]
    fn test_parse_alpine_provider_field() {
        let content = "P:some-package
V:1.0
p:cmd:binary=1.0
p:so:libtest.so.1

";
        let pkg = parse_alpine_installed_db(content);
        assert!(pkg.extra_data.is_some());
        let extra = pkg.extra_data.unwrap();
        let providers = extra.get("providers").and_then(|v| v.as_array());
        assert!(providers.is_some());
        let provider_array = providers.unwrap();
        assert_eq!(provider_array.len(), 2);
        assert_eq!(provider_array[0].as_str(), Some("cmd:binary=1.0"));
        assert_eq!(provider_array[1].as_str(), Some("so:libtest.so.1"));
    }

    #[test]
    fn test_alpine_apk_parser_is_match() {
        assert!(AlpineApkParser::is_match(&PathBuf::from("package.apk")));
        assert!(AlpineApkParser::is_match(&PathBuf::from(
            "/path/to/app-1.0.apk"
        )));
        assert!(!AlpineApkParser::is_match(&PathBuf::from("package.tar.gz")));
        assert!(!AlpineApkParser::is_match(&PathBuf::from("installed")));
    }
}

crate::register_parser!(
    "Alpine Linux package (installed db and .apk archive)",
    &["**/lib/apk/db/installed", "**/*.apk"],
    "alpine",
    "",
    Some("https://wiki.alpinelinux.org/wiki/Apk_spec"),
);
