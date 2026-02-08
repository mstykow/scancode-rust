//! Parser for Swift Package Manager manifest files.
//!
//! Supports three input formats:
//! - `Package.swift.json` - Pre-generated JSON (recommended for CI/CD)
//! - `Package.swift.deplock` - DepLock JSON format
//! - `Package.swift` - Raw Swift source (auto-generates JSON with caching)
//!
//! # Automatic JSON Generation (Enhancement over Python ScanCode)
//!
//! This Rust implementation includes automatic JSON generation from raw `Package.swift`
//! files, which is an enhancement beyond the Python ScanCode reference implementation.
//!
//! **Python ScanCode behavior**: Requires users to manually run:
//! ```bash
//! swift package dump-package > Package.swift.json
//! ```
//!
//! **Rust ScanCode behavior**: Automatically generates JSON when Swift toolchain available,
//! with BLAKE3-based caching for performance.
//!
//! ## Design Decision: Graceful Degradation
//!
//! - **Swift toolchain available**: Automatically generates + caches JSON (~200ms first, <1ms cached)
//! - **Swift toolchain unavailable**: Warns and skips file (no crash, CI/CD unaffected)
//! - **Pre-generated JSON**: Always works, regardless of Swift availability
//!
//! This design allows:
//! - ✅ Better UX for developers with Swift installed
//! - ✅ No CI/CD complications (tests don't require Swift)
//! - ✅ Backward compatibility (pre-generated JSON workflow unchanged)
//! - ✅ Feature parity maintained (Python behavior is subset of Rust behavior)

use std::collections::HashMap;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::process::Command;

use log::warn;
use packageurl::PackageUrl;
use serde_json::Value;

use crate::models::{Dependency, PackageData};

use super::PackageParser;

/// Parses Swift Package Manager manifest files with automatic JSON generation.
///
/// # Supported File Formats
/// - `Package.swift.json` - Pre-generated JSON from `swift package dump-package`
/// - `Package.swift.deplock` - JSON format from DepLock tool
/// - `Package.swift` - Raw Swift source (auto-generates JSON if Swift available)
///
/// # Automatic JSON Generation
///
/// When scanning raw `Package.swift` files:
/// 1. Checks BLAKE3-based cache for previously generated JSON
/// 2. If cache miss, invokes `swift package dump-package` (requires Swift toolchain)
/// 3. Caches result for future scans
/// 4. Falls back gracefully if Swift unavailable (logs warning, returns empty package data)
///
/// # Performance
/// - **Pre-generated JSON**: <1ms (direct file read)
/// - **Raw Package.swift (cached)**: <1ms (cache hit)
/// - **Raw Package.swift (first time)**: ~100-500ms (Swift toolchain execution + cache write)
/// - **Raw Package.swift (no Swift)**: <1ms (immediate fallback)
///
/// # Example
/// ```no_run
/// use scancode_rust::parsers::{SwiftManifestJsonParser, PackageParser};
/// use std::path::Path;
///
/// // Works with pre-generated JSON
/// let json_path = Path::new("Package.swift.json");
/// let data1 = SwiftManifestJsonParser::extract_package_data(json_path);
///
/// // Also works with raw Package.swift (if Swift installed)
/// let swift_path = Path::new("Package.swift");
/// let data2 = SwiftManifestJsonParser::extract_package_data(swift_path);
/// ```
pub struct SwiftManifestJsonParser;

impl PackageParser for SwiftManifestJsonParser {
    const PACKAGE_TYPE: &'static str = "swift";

    fn extract_package_data(path: &Path) -> PackageData {
        let filename = path.file_name().and_then(|n| n.to_str());

        let is_json_file = filename
            .map(|n| n.ends_with(".swift.json") || n.ends_with(".swift.deplock"))
            .unwrap_or(false);
        let is_raw_swift = filename.map(|n| n == "Package.swift").unwrap_or(false);

        if is_json_file {
            let json_content = match read_swift_manifest_json(path) {
                Ok(content) => content,
                Err(e) => {
                    warn!(
                        "Failed to read or parse Swift manifest JSON at {:?}: {}",
                        path, e
                    );
                    return default_package_data();
                }
            };
            parse_swift_manifest(&json_content)
        } else if is_raw_swift {
            match dump_package_cached(path) {
                Ok(json_str) => match serde_json::from_str::<Value>(&json_str) {
                    Ok(json) => parse_swift_manifest(&json),
                    Err(e) => {
                        warn!(
                            "Swift toolchain generated invalid JSON for {:?}: {}",
                            path, e
                        );
                        default_package_data()
                    }
                },
                Err(e) => {
                    warn!(
                        "Cannot auto-generate Package.swift.json for {:?}: {}. \
                             Swift toolchain may not be installed. \
                             To scan this file, manually run: swift package dump-package > Package.swift.json",
                        path, e
                    );
                    default_package_data()
                }
            }
        } else {
            default_package_data()
        }
    }

    fn is_match(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.ends_with(".swift.json")
                    || name.ends_with(".swift.deplock")
                    || name == "Package.swift"
            })
    }
}

fn read_swift_manifest_json(path: &Path) -> Result<Value, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))
}

fn parse_swift_manifest(manifest: &Value) -> PackageData {
    let name = manifest
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from);

    let dependencies = get_dependencies(manifest.get("dependencies"));
    let platforms = manifest.get("platforms").cloned();

    let tools_version = manifest
        .get("toolsVersion")
        .and_then(|tv| tv.get("_version"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut extra_data = HashMap::new();
    if let Some(platforms_val) = platforms {
        extra_data.insert("platforms".to_string(), platforms_val);
    }
    if let Some(ref tv) = tools_version {
        extra_data.insert(
            "swift_tools_version".to_string(),
            serde_json::Value::String(tv.clone()),
        );
    }

    let purl = create_package_url(&name, &None);

    PackageData {
        package_type: Some(SwiftManifestJsonParser::PACKAGE_TYPE.to_string()),
        namespace: None,
        name,
        version: None,
        qualifiers: None,
        subpath: None,
        primary_language: Some("Swift".to_string()),
        description: None,
        release_date: None,
        parties: Vec::new(),
        keywords: Vec::new(),
        homepage_url: None,
        download_url: None,
        size: None,
        sha1: None,
        md5: None,
        sha256: None,
        sha512: None,
        bug_tracking_url: None,
        code_view_url: None,
        vcs_url: None,
        copyright: None,
        holder: None,
        declared_license_expression: None,
        declared_license_expression_spdx: None,
        license_detections: Vec::new(),
        other_license_expression: None,
        other_license_expression_spdx: None,
        other_license_detections: Vec::new(),
        extracted_license_statement: None,
        notice_text: None,
        source_packages: Vec::new(),
        file_references: Vec::new(),
        is_private: false,
        is_virtual: false,
        extra_data: if extra_data.is_empty() {
            None
        } else {
            Some(extra_data)
        },
        dependencies,
        repository_homepage_url: None,
        repository_download_url: None,
        api_data_url: None,
        datasource_id: Some("swift_package_manifest_json".to_string()),
        purl,
    }
}

fn get_dependencies(dependencies: Option<&Value>) -> Vec<Dependency> {
    let Some(deps_array) = dependencies.and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    let mut dependent_packages = Vec::new();

    for dependency in deps_array {
        let Some(source_control) = dependency.get("sourceControl").and_then(|v| v.as_array())
        else {
            continue;
        };

        let Some(source) = source_control.first() else {
            continue;
        };

        let identity = source
            .get("identity")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        let (namespace, dep_name) = extract_namespace_and_name(source, identity);
        let (version, is_pinned) = extract_version_requirement(source);
        let purl = create_dependency_purl(&namespace, &dep_name, &version, is_pinned);

        dependent_packages.push(Dependency {
            purl: Some(purl),
            extracted_requirement: version,
            scope: Some("dependencies".to_string()),
            is_runtime: Some(true),
            is_optional: Some(false),
            is_pinned: Some(is_pinned),
            is_direct: Some(true),
            resolved_package: None,
            extra_data: None,
        });
    }

    dependent_packages
}

fn extract_namespace_and_name(source: &Value, identity: &str) -> (Option<String>, String) {
    let url = source
        .get("location")
        .and_then(|loc| loc.get("remote"))
        .and_then(|remote| remote.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("urlString"))
        .and_then(|v| v.as_str());

    match url {
        Some(url_str) => get_namespace_and_name(url_str),
        None => (None, identity.to_string()),
    }
}

/// Parses a repository URL into (namespace, name).
///
/// Example: `https://github.com/apple/swift-argument-parser.git`
/// yields namespace=`"github.com/apple"`, name=`"swift-argument-parser"`
pub fn get_namespace_and_name(url: &str) -> (Option<String>, String) {
    let (hostname, path) = if let Some(stripped) = url.strip_prefix("https://") {
        let rest = stripped.trim_end_matches('/');
        match rest.find('/') {
            Some(idx) => (Some(&rest[..idx]), &rest[idx + 1..]),
            None => (Some(rest), ""),
        }
    } else if let Some(stripped) = url.strip_prefix("http://") {
        let rest = stripped.trim_end_matches('/');
        match rest.find('/') {
            Some(idx) => (Some(&rest[..idx]), &rest[idx + 1..]),
            None => (Some(rest), ""),
        }
    } else {
        (None, url)
    };

    let clean_path = path
        .strip_suffix(".git")
        .unwrap_or(path)
        .trim_end_matches('/');

    if let Some(host) = hostname {
        let canonical = format!("{}/{}", host, clean_path);
        match canonical.rsplit_once('/') {
            Some((ns, name)) => (Some(ns.to_string()), name.to_string()),
            None => (None, canonical),
        }
    } else {
        match clean_path.rsplit_once('/') {
            Some((ns, name)) => (Some(ns.to_string()), name.to_string()),
            None => (None, clean_path.to_string()),
        }
    }
}

/// Handles four requirement types:
/// - `exact`: `["1.0.0"]` -> version="1.0.0", is_pinned=true
/// - `range`: `[{"lowerBound": "1.0.0", "upperBound": "2.0.0"}]` -> version="vers:swift/>=1.0.0|<2.0.0", is_pinned=false
/// - `branch`: `["main"]` -> version="main", is_pinned=false
/// - `revision`: `["abc123"]` -> version="abc123", is_pinned=true
fn extract_version_requirement(source: &Value) -> (Option<String>, bool) {
    let Some(requirement) = source.get("requirement") else {
        return (None, false);
    };

    if let Some(exact) = requirement.get("exact").and_then(|v| v.as_array())
        && let Some(version) = exact.first().and_then(|v| v.as_str())
    {
        return (Some(version.to_string()), true);
    }

    if let Some(range) = requirement.get("range").and_then(|v| v.as_array())
        && let Some(bound) = range.first()
    {
        let lower = bound.get("lowerBound").and_then(|v| v.as_str());
        let upper = bound.get("upperBound").and_then(|v| v.as_str());
        if let (Some(lb), Some(ub)) = (lower, upper) {
            let vers = format!("vers:swift/>={lb}|<{ub}");
            return (Some(vers), false);
        }
    }

    if let Some(branch) = requirement.get("branch").and_then(|v| v.as_array())
        && let Some(branch_name) = branch.first().and_then(|v| v.as_str())
    {
        return (Some(branch_name.to_string()), false);
    }

    if let Some(revision) = requirement.get("revision").and_then(|v| v.as_array())
        && let Some(rev) = revision.first().and_then(|v| v.as_str())
    {
        return (Some(rev.to_string()), true);
    }

    (None, false)
}

fn create_dependency_purl(
    namespace: &Option<String>,
    name: &str,
    version: &Option<String>,
    is_pinned: bool,
) -> String {
    let mut purl = match PackageUrl::new(SwiftManifestJsonParser::PACKAGE_TYPE, name) {
        Ok(p) => p,
        Err(e) => {
            warn!(
                "Failed to create PackageUrl for swift dependency '{}': {}",
                name, e
            );
            return match (namespace, is_pinned.then_some(version.as_deref()).flatten()) {
                (Some(ns), Some(v)) => format!("pkg:swift/{}/{}@{}", ns, name, v),
                (Some(ns), None) => format!("pkg:swift/{}/{}", ns, name),
                (None, Some(v)) => format!("pkg:swift/{}@{}", name, v),
                (None, None) => format!("pkg:swift/{}", name),
            };
        }
    };

    if let Some(ns) = namespace
        && let Err(e) = purl.with_namespace(ns)
    {
        warn!(
            "Failed to set namespace '{}' for swift dependency '{}': {}",
            ns, name, e
        );
    }

    if is_pinned
        && let Some(v) = version
        && let Err(e) = purl.with_version(v)
    {
        warn!(
            "Failed to set version '{}' for swift dependency '{}': {}",
            v, name, e
        );
    }

    purl.to_string()
}

fn create_package_url(name: &Option<String>, version: &Option<String>) -> Option<String> {
    name.as_ref().and_then(|name| {
        let mut package_url = match PackageUrl::new(SwiftManifestJsonParser::PACKAGE_TYPE, name) {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    "Failed to create PackageUrl for swift package '{}': {}",
                    name, e
                );
                return None;
            }
        };

        if let Some(v) = version
            && let Err(e) = package_url.with_version(v)
        {
            warn!(
                "Failed to set version '{}' for swift package '{}': {}",
                v, name, e
            );
            return None;
        }

        Some(package_url.to_string())
    })
}

/// Invokes `swift package dump-package` to generate Package.swift.json.
///
/// Executes the Swift toolchain command to convert Package.swift into JSON format.
///
/// This function is used internally by `dump_package_cached()` to generate JSON
/// from raw Package.swift files. It requires the Swift toolchain to be installed
/// and available on PATH.
///
/// # Arguments
/// * `package_dir` - Directory containing Package.swift
///
/// # Returns
/// * `Ok(String)` - JSON string output from swift command
/// * `Err(String)` - Error message if Swift toolchain unavailable or command fails
///
/// # Note
/// This function is public for testing purposes but is not intended for direct use.
/// Use `dump_package_cached()` instead for automatic caching.
pub fn invoke_swift_dump_package(package_dir: &Path) -> Result<String, String> {
    let output = Command::new("swift")
        .args(["package", "dump-package"])
        .current_dir(package_dir)
        .output()
        .map_err(|e| {
            format!(
                "Failed to execute 'swift package dump-package' in {:?}: {}. \
                 Is the Swift toolchain installed and available on PATH?",
                package_dir, e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "'swift package dump-package' failed in {:?} (exit code: {:?}): {}",
            package_dir,
            output.status.code(),
            stderr.trim()
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("swift dump-package output is not valid UTF-8: {}", e))
}

/// Generates or retrieves cached Package.swift.json using BLAKE3 content hashing.
///
/// This is the primary entry point for converting raw Package.swift files to JSON.
/// It implements a content-based caching strategy where the cache key is the BLAKE3
/// hash of the Package.swift file contents.
///
/// # Caching Strategy
///
/// 1. **Cache Key**: BLAKE3 hash of Package.swift content (not file path)
/// 2. **Cache Hit**: Returns cached JSON (<1ms)
/// 3. **Cache Miss**: Executes `swift package dump-package`, validates JSON, caches result
/// 4. **Cache Location**: System cache directory (e.g., ~/.cache/scancode-rust/swift/)
///
/// # Performance
/// - **Cache hit**: <1ms (single file read)
/// - **Cache miss**: ~100-500ms (Swift toolchain execution + validation + cache write)
///
/// # Error Handling
///
/// Returns `Err` if:
/// - Package.swift file cannot be read
/// - Swift toolchain not installed or not on PATH
/// - `swift package dump-package` command fails
/// - Output is not valid JSON
/// - Cannot determine or create cache directory
///
/// Cache write failures are logged but not returned as errors (graceful degradation).
///
/// # Arguments
/// * `package_swift_path` - Path to Package.swift file
///
/// # Returns
/// * `Ok(String)` - Valid JSON string (from cache or freshly generated)
/// * `Err(String)` - Error message with context
///
/// # Example
/// ```no_run
/// use scancode_rust::parsers::swift_manifest_json::dump_package_cached;
/// use std::path::Path;
///
/// let swift_path = Path::new("path/to/Package.swift");
/// match dump_package_cached(swift_path) {
///     Ok(json) => println!("Got JSON: {}", json),
///     Err(e) => eprintln!("Swift toolchain unavailable: {}", e),
/// }
/// ```
pub fn dump_package_cached(package_swift_path: &Path) -> Result<String, String> {
    let content = fs::read_to_string(package_swift_path).map_err(|e| {
        format!(
            "Failed to read Package.swift at {:?}: {}",
            package_swift_path, e
        )
    })?;

    let hash = blake3::hash(content.as_bytes()).to_hex().to_string();

    let cache_dir = get_cache_dir()?;
    let cache_file = cache_dir.join(format!("{}.json", hash));

    if cache_file.exists() {
        match fs::read_to_string(&cache_file) {
            Ok(cached) => return Ok(cached),
            Err(e) => {
                warn!(
                    "Failed to read cache file {:?}, regenerating: {}",
                    cache_file, e
                );
            }
        }
    }

    let parent_dir = package_swift_path.parent().ok_or_else(|| {
        format!(
            "Cannot determine parent directory of {:?}",
            package_swift_path
        )
    })?;

    let json_output = invoke_swift_dump_package(parent_dir)?;

    serde_json::from_str::<Value>(&json_output)
        .map_err(|e| format!("swift dump-package produced invalid JSON: {}", e))?;

    if let Err(e) = write_cache_file(&cache_file, &json_output) {
        warn!("Failed to write cache file {:?}: {}", cache_file, e);
    }

    Ok(json_output)
}

fn get_cache_dir() -> Result<std::path::PathBuf, String> {
    let base = dirs_cache_dir().ok_or("Cannot determine cache directory")?;
    let cache_dir = base.join("scancode-rust").join("swift");

    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory {:?}: {}", cache_dir, e))?;

    Ok(cache_dir)
}

fn dirs_cache_dir() -> Option<std::path::PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        let path = std::path::PathBuf::from(xdg);
        if path.is_absolute() {
            return Some(path);
        }
    }

    home_dir().map(|home| {
        if cfg!(target_os = "macos") {
            home.join("Library").join("Caches")
        } else {
            home.join(".cache")
        }
    })
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
}

fn write_cache_file(path: &Path, content: &str) -> Result<(), String> {
    let parent = path.parent().ok_or("Cache file has no parent directory")?;

    // Write to temp then rename for atomicity
    let temp_path = parent.join(format!(
        ".tmp-{}-{}",
        std::process::id(),
        path.file_name().and_then(|n| n.to_str()).unwrap_or("cache")
    ));

    let mut file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temp file {:?}: {}", temp_path, e))?;

    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write temp file {:?}: {}", temp_path, e))?;

    fs::rename(&temp_path, path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        format!(
            "Failed to rename temp file {:?} to {:?}: {}",
            temp_path, path, e
        )
    })?;

    Ok(())
}

fn default_package_data() -> PackageData {
    PackageData {
        package_type: None,
        namespace: None,
        name: None,
        version: None,
        qualifiers: None,
        subpath: None,
        primary_language: None,
        description: None,
        release_date: None,
        parties: Vec::new(),
        keywords: Vec::new(),
        homepage_url: None,
        download_url: None,
        size: None,
        sha1: None,
        md5: None,
        sha256: None,
        sha512: None,
        bug_tracking_url: None,
        code_view_url: None,
        vcs_url: None,
        copyright: None,
        holder: None,
        declared_license_expression: None,
        declared_license_expression_spdx: None,
        license_detections: Vec::new(),
        other_license_expression: None,
        other_license_expression_spdx: None,
        other_license_detections: Vec::new(),
        extracted_license_statement: None,
        notice_text: None,
        source_packages: Vec::new(),
        file_references: Vec::new(),
        is_private: false,
        is_virtual: false,
        extra_data: None,
        dependencies: Vec::new(),
        repository_homepage_url: None,
        repository_download_url: None,
        api_data_url: None,
        datasource_id: None,
        purl: None,
    }
}

crate::register_parser!(
    "Swift Package Manager manifest (Package.swift, Package.swift.json, Package.swift.deplock)",
    &[
        "**/Package.swift",
        "**/Package.swift.json",
        "**/Package.swift.deplock"
    ],
    "swift",
    "Swift",
    Some("https://docs.swift.org/package-manager/PackageDescription/PackageDescription.html"),
);
