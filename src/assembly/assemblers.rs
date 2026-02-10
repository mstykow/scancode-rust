use super::AssemblerConfig;

/// All registered sibling-merge assembler configurations.
///
/// Each entry maps a set of datasource IDs to the sibling files that should
/// be merged together. The first pattern in `sibling_file_patterns` is the
/// primary manifest; subsequent patterns provide supplementary data.
pub static ASSEMBLERS: &[AssemblerConfig] = &[
    // npm: package.json + lockfiles
    AssemblerConfig {
        datasource_ids: &[
            "npm_package_json",
            "npm_package_lock_json",
            "npm_shrinkwrap_json",
            "yarn_lock",
            "pnpm_lock_yaml",
        ],
        sibling_file_patterns: &[
            "package.json",
            "package-lock.json",
            "npm-shrinkwrap.json",
            "yarn.lock",
            "pnpm-lock.yaml",
        ],
    },
    // Cargo: Cargo.toml + Cargo.lock
    AssemblerConfig {
        datasource_ids: &["cargo_toml", "cargo_lock"],
        sibling_file_patterns: &["Cargo.toml", "Cargo.lock"],
    },
    // CocoaPods: podspec + Podfile + Podfile.lock
    AssemblerConfig {
        datasource_ids: &[
            "cocoapods_podspec",
            "cocoapods_podspec_json",
            "cocoapods_podfile",
            "cocoapods_podfile_lock",
        ],
        sibling_file_patterns: &["*.podspec", "*.podspec.json", "Podfile", "Podfile.lock"],
    },
    // PHP Composer: composer.json + composer.lock
    AssemblerConfig {
        datasource_ids: &["php_composer_json", "php_composer_lock"],
        sibling_file_patterns: &["composer.json", "composer.lock"],
    },
    // Go: go.mod + go.sum
    AssemblerConfig {
        datasource_ids: &["go_mod", "go_sum"],
        sibling_file_patterns: &["go.mod", "go.sum"],
    },
    // Dart/Pubspec: pubspec.yaml + pubspec.lock
    AssemblerConfig {
        datasource_ids: &["pubspec_yaml", "pubspec_lock"],
        sibling_file_patterns: &["pubspec.yaml", "pubspec.lock"],
    },
    // Chef: metadata.json + metadata.rb
    AssemblerConfig {
        datasource_ids: &["chef_metadata_json", "chef_metadata_rb"],
        sibling_file_patterns: &["metadata.json", "metadata.rb"],
    },
    // Conan: conanfile.py + conanfile.txt + conan.lock + conandata.yml
    AssemblerConfig {
        datasource_ids: &[
            "conan_conanfile_py",
            "conan_conanfile_txt",
            "conan_lock",
            "conan_conandata_yml",
        ],
        sibling_file_patterns: &[
            "conanfile.py",
            "conanfile.txt",
            "conan.lock",
            "conandata.yml",
        ],
    },
];
