//! Package type identifiers for package parsers.
//!
//! Each variant uniquely identifies the package ecosystem/registry type.
//! These are used in Package URL (purl) type fields and in the JSON output
//! as the `"type"` field of package data.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Package ecosystem/registry type identifier.
///
/// Identifies the package manager or ecosystem a package belongs to
/// (e.g., npm, PyPI, Maven, Cargo). Used as the `"type"` field in
/// ScanCode Toolkit-compatible JSON output.
///
/// This enum includes both standard purl types and ScanCode-specific types
/// for file format recognizers (e.g., `Jar`, `War`) and metadata sources
/// (e.g., `About`, `Readme`). For the official list of standardized purl types, see:
/// <https://github.com/package-url/purl-spec/blob/main/purl-types-index.json>
///
/// # Serialization
///
/// Variants serialize to lowercase/kebab-case strings matching the
/// Python reference values. The JSON output is identical to the
/// Python ScanCode Toolkit.
///
/// # Examples
///
/// ```ignore
/// use scancode_rust::models::PackageType;
///
/// let pt = PackageType::Npm;
/// assert_eq!(pt.as_ref(), "npm");
/// assert_eq!(pt.to_string(), "npm");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PackageType {
    About,
    Alpine,
    Android,
    AndroidLib,
    Autotools,
    Axis2,
    Bazel,
    Bower,
    Buck,
    Cab,
    Cargo,
    Chef,
    Chrome,
    Cocoapods,
    Composer,
    Conan,
    Conda,
    Cpan,
    Cran,
    Dart,
    Deb,
    Dmg,
    Ear,
    Freebsd,
    Gem,
    Github,
    Golang,
    Haxe,
    Installshield,
    Ios,
    Iso,
    Ivy,
    Jar,
    JbossService,
    LinuxDistro,
    Maven,
    Meteor,
    Mozilla,
    Npm,
    Nsis,
    Nuget,
    Opam,
    Osgi,
    PnpmLock,
    Pubspec,
    Pypi,
    Readme,
    Rpm,
    Shar,
    Squashfs,
    Swift,
    War,
    WindowsUpdate,
    Unknown,
}

impl PackageType {
    /// Returns the string representation of this package type.
    ///
    /// This matches the serialized form used in JSON output.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::About => "about",
            Self::Alpine => "alpine",
            Self::Android => "android",
            Self::AndroidLib => "android_lib",
            Self::Autotools => "autotools",
            Self::Axis2 => "axis2",
            Self::Bazel => "bazel",
            Self::Bower => "bower",
            Self::Buck => "buck",
            Self::Cab => "cab",
            Self::Cargo => "cargo",
            Self::Chef => "chef",
            Self::Chrome => "chrome",
            Self::Cocoapods => "cocoapods",
            Self::Composer => "composer",
            Self::Conan => "conan",
            Self::Conda => "conda",
            Self::Cpan => "cpan",
            Self::Cran => "cran",
            Self::Dart => "dart",
            Self::Deb => "deb",
            Self::Dmg => "dmg",
            Self::Ear => "ear",
            Self::Freebsd => "freebsd",
            Self::Gem => "gem",
            Self::Github => "github",
            Self::Golang => "golang",
            Self::Haxe => "haxe",
            Self::Installshield => "installshield",
            Self::Ios => "ios",
            Self::Iso => "iso",
            Self::Ivy => "ivy",
            Self::Jar => "jar",
            Self::JbossService => "jboss-service",
            Self::LinuxDistro => "linux-distro",
            Self::Maven => "maven",
            Self::Meteor => "meteor",
            Self::Mozilla => "mozilla",
            Self::Npm => "npm",
            Self::Nsis => "nsis",
            Self::Nuget => "nuget",
            Self::Opam => "opam",
            Self::Osgi => "osgi",
            Self::PnpmLock => "pnpm-lock",
            Self::Pubspec => "pubspec",
            Self::Pypi => "pypi",
            Self::Readme => "readme",
            Self::Rpm => "rpm",
            Self::Shar => "shar",
            Self::Squashfs => "squashfs",
            Self::Swift => "swift",
            Self::War => "war",
            Self::WindowsUpdate => "windows-update",
            Self::Unknown => "unknown",
        }
    }
}

impl Serialize for PackageType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PackageType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from_str(&s).unwrap())
    }
}

impl AsRef<str> for PackageType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PackageType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "about" => Ok(PackageType::About),
            "alpine" => Ok(PackageType::Alpine),
            "android" => Ok(PackageType::Android),
            "android_lib" => Ok(PackageType::AndroidLib),
            "autotools" => Ok(PackageType::Autotools),
            "axis2" => Ok(PackageType::Axis2),
            "bazel" => Ok(PackageType::Bazel),
            "bower" => Ok(PackageType::Bower),
            "buck" => Ok(PackageType::Buck),
            "cab" => Ok(PackageType::Cab),
            "cargo" => Ok(PackageType::Cargo),
            "chef" => Ok(PackageType::Chef),
            "chrome" => Ok(PackageType::Chrome),
            "cocoapods" => Ok(PackageType::Cocoapods),
            "composer" => Ok(PackageType::Composer),
            "conan" => Ok(PackageType::Conan),
            "conda" => Ok(PackageType::Conda),
            "cpan" => Ok(PackageType::Cpan),
            "cran" => Ok(PackageType::Cran),
            "dart" => Ok(PackageType::Dart),
            "deb" => Ok(PackageType::Deb),
            "dmg" => Ok(PackageType::Dmg),
            "ear" => Ok(PackageType::Ear),
            "freebsd" => Ok(PackageType::Freebsd),
            "gem" => Ok(PackageType::Gem),
            "github" => Ok(PackageType::Github),
            "golang" => Ok(PackageType::Golang),
            "haxe" => Ok(PackageType::Haxe),
            "installshield" => Ok(PackageType::Installshield),
            "ios" => Ok(PackageType::Ios),
            "iso" => Ok(PackageType::Iso),
            "ivy" => Ok(PackageType::Ivy),
            "jar" => Ok(PackageType::Jar),
            "jboss-service" => Ok(PackageType::JbossService),
            "linux-distro" => Ok(PackageType::LinuxDistro),
            "maven" => Ok(PackageType::Maven),
            "meteor" => Ok(PackageType::Meteor),
            "mozilla" => Ok(PackageType::Mozilla),
            "npm" => Ok(PackageType::Npm),
            "nsis" => Ok(PackageType::Nsis),
            "nuget" => Ok(PackageType::Nuget),
            "opam" => Ok(PackageType::Opam),
            "osgi" => Ok(PackageType::Osgi),
            "pnpm-lock" => Ok(PackageType::PnpmLock),
            "pubspec" => Ok(PackageType::Pubspec),
            "pypi" => Ok(PackageType::Pypi),
            "readme" => Ok(PackageType::Readme),
            "rpm" => Ok(PackageType::Rpm),
            "shar" => Ok(PackageType::Shar),
            "squashfs" => Ok(PackageType::Squashfs),
            "swift" => Ok(PackageType::Swift),
            "war" => Ok(PackageType::War),
            "windows-update" => Ok(PackageType::WindowsUpdate),
            "unknown" => Ok(PackageType::Unknown),
            _ => Ok(PackageType::Unknown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let pt = PackageType::Npm;
        let json = serde_json::to_string(&pt).unwrap();
        assert_eq!(json, r#""npm""#);
    }

    #[test]
    fn test_deserialization() {
        let json = r#""npm""#;
        let pt: PackageType = serde_json::from_str(json).unwrap();
        assert_eq!(pt, PackageType::Npm);
    }

    #[test]
    fn test_as_str() {
        assert_eq!(PackageType::Npm.as_str(), "npm");
        assert_eq!(PackageType::Cargo.as_str(), "cargo");
        assert_eq!(PackageType::Pypi.as_str(), "pypi");
    }

    #[test]
    fn test_display() {
        assert_eq!(PackageType::Npm.to_string(), "npm");
    }

    #[test]
    fn test_as_ref() {
        let pt = PackageType::Npm;
        let s: &str = pt.as_ref();
        assert_eq!(s, "npm");
    }

    #[test]
    fn test_kebab_case_variants() {
        assert_eq!(PackageType::JbossService.as_str(), "jboss-service");
        assert_eq!(PackageType::LinuxDistro.as_str(), "linux-distro");
        assert_eq!(PackageType::PnpmLock.as_str(), "pnpm-lock");
        assert_eq!(PackageType::WindowsUpdate.as_str(), "windows-update");

        // Verify serialization matches
        let json = serde_json::to_string(&PackageType::JbossService).unwrap();
        assert_eq!(json, r#""jboss-service""#);

        let json = serde_json::to_string(&PackageType::LinuxDistro).unwrap();
        assert_eq!(json, r#""linux-distro""#);

        let json = serde_json::to_string(&PackageType::PnpmLock).unwrap();
        assert_eq!(json, r#""pnpm-lock""#);

        let json = serde_json::to_string(&PackageType::WindowsUpdate).unwrap();
        assert_eq!(json, r#""windows-update""#);
    }

    #[test]
    fn test_snake_case_variant() {
        assert_eq!(PackageType::AndroidLib.as_str(), "android_lib");

        let json = serde_json::to_string(&PackageType::AndroidLib).unwrap();
        assert_eq!(json, r#""android_lib""#);
    }

    #[test]
    fn test_deserialization_kebab_case() {
        let pt: PackageType = serde_json::from_str(r#""jboss-service""#).unwrap();
        assert_eq!(pt, PackageType::JbossService);

        let pt: PackageType = serde_json::from_str(r#""linux-distro""#).unwrap();
        assert_eq!(pt, PackageType::LinuxDistro);

        let pt: PackageType = serde_json::from_str(r#""pnpm-lock""#).unwrap();
        assert_eq!(pt, PackageType::PnpmLock);

        let pt: PackageType = serde_json::from_str(r#""windows-update""#).unwrap();
        assert_eq!(pt, PackageType::WindowsUpdate);
    }

    #[test]
    fn test_unknown_type_from_str_never_fails() {
        let pt = PackageType::from_str("nonexistent_type").unwrap();
        assert_eq!(pt, PackageType::Unknown);

        let pt = PackageType::from_str("").unwrap();
        assert_eq!(pt, PackageType::Unknown);
    }

    #[test]
    fn test_unknown_type_serializes_correctly() {
        assert_eq!(PackageType::Unknown.as_str(), "unknown");

        let json = serde_json::to_string(&PackageType::Unknown).unwrap();
        assert_eq!(json, r#""unknown""#);

        let pt: PackageType = serde_json::from_str(r#""unknown""#).unwrap();
        assert_eq!(pt, PackageType::Unknown);
    }

    #[test]
    fn test_unknown_type_deserialization() {
        let pt: PackageType = serde_json::from_str(r#""nonexistent_type""#).unwrap();
        assert_eq!(pt, PackageType::Unknown);
    }

    #[test]
    fn test_known_types_still_work() {
        let pt = PackageType::from_str("npm").unwrap();
        assert_eq!(pt, PackageType::Npm);

        let pt = PackageType::from_str("cargo").unwrap();
        assert_eq!(pt, PackageType::Cargo);

        let pt = PackageType::from_str("pypi").unwrap();
        assert_eq!(pt, PackageType::Pypi);
    }
}
