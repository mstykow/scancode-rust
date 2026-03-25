use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use chrono::Utc;
use glob::Pattern;

use crate::assembly;
use crate::license_detection::expression::{
    combine_expressions_and, expression_to_string, parse_expression, simplify_expression,
};
use crate::models::{
    DatasourceId, ExtraData, FacetTallies, FileInfo, FileType, Header, LicenseClarityScore, Match,
    OUTPUT_FORMAT_VERSION, Output, Package, Summary, SystemEnvironment, Tallies, TallyEntry,
};
use crate::scanner;
use crate::utils::spdx::combine_license_expressions;

#[cfg(test)]
mod classify_test;
#[cfg(test)]
mod facet_test;
#[cfg(test)]
mod generated_test;
#[cfg(test)]
mod output_test;
#[cfg(test)]
mod summary_test;
#[cfg(test)]
mod tallies_test;
#[cfg(test)]
mod test_utils;

pub(crate) struct CreateOutputOptions<'a> {
    pub(crate) facet_rules: &'a [FacetRule],
    pub(crate) include_summary: bool,
    pub(crate) include_license_clarity_score: bool,
    pub(crate) include_tallies: bool,
    pub(crate) include_tallies_of_key_files: bool,
    pub(crate) include_tallies_with_details: bool,
    pub(crate) include_tallies_by_facet: bool,
    pub(crate) include_generated: bool,
    pub(crate) scanned_root: Option<&'a Path>,
}

pub(crate) struct CreateOutputContext<'a> {
    pub(crate) total_dirs: usize,
    pub(crate) assembly_result: assembly::AssemblyResult,
    pub(crate) license_references: Vec<crate::models::LicenseReference>,
    pub(crate) license_rule_references: Vec<crate::models::LicenseRuleReference>,
    pub(crate) options: CreateOutputOptions<'a>,
}

pub(crate) fn create_output(
    start_time: chrono::DateTime<Utc>,
    end_time: chrono::DateTime<Utc>,
    scan_result: scanner::ProcessResult,
    context: CreateOutputContext<'_>,
) -> Output {
    let duration = (end_time - start_time).num_nanoseconds().unwrap_or(0) as f64 / 1_000_000_000.0;

    let extra_data = ExtraData {
        files_count: scan_result.files.len(),
        directories_count: context.total_dirs,
        excluded_count: scan_result.excluded_count,
        system_environment: SystemEnvironment {
            operating_system: sys_info::os_type().ok(),
            cpu_architecture: env::consts::ARCH.to_string(),
            platform: format!(
                "{}-{}-{}",
                sys_info::os_type().unwrap_or_else(|_| "unknown".to_string()),
                sys_info::os_release().unwrap_or_else(|_| "unknown".to_string()),
                env::consts::ARCH
            ),
            rust_version: rustc_version_runtime::version().to_string(),
        },
    };

    let errors: Vec<String> = scan_result
        .files
        .iter()
        .filter_map(|file| {
            if file.scan_errors.is_empty() {
                None
            } else {
                Some(
                    file.scan_errors
                        .iter()
                        .map(|error| format!("{}: {}", file.path, error))
                        .collect::<Vec<String>>(),
                )
            }
        })
        .flatten()
        .collect();

    let mut files = scan_result.files;
    let assembly::AssemblyResult {
        mut packages,
        dependencies,
    } = context.assembly_result;
    if context.options.include_generated {
        mark_generated_files(&mut files, context.options.scanned_root);
    } else {
        clear_generated_flags(&mut files);
    }
    if context.options.include_summary
        || context.options.include_license_clarity_score
        || context.options.include_tallies_of_key_files
    {
        classify_key_files(&mut files, &packages);
    }
    promote_package_metadata_from_key_files(&files, &mut packages);
    assign_facets(&mut files, context.options.facet_rules);
    let needs_detailed_tallies =
        context.options.include_tallies_with_details || context.options.include_tallies_by_facet;
    if needs_detailed_tallies {
        compute_detailed_tallies(&mut files);
    } else {
        clear_resource_tallies(&mut files);
    }
    let summary =
        if context.options.include_summary || context.options.include_license_clarity_score {
            compute_summary_with_options(
                &files,
                &packages,
                context.options.include_summary,
                context.options.include_license_clarity_score || context.options.include_summary,
            )
        } else {
            None
        };
    let tallies = if context.options.include_tallies || context.options.include_tallies_with_details
    {
        compute_tallies(&files)
    } else {
        None
    };
    let tallies_of_key_files = if context.options.include_tallies_of_key_files {
        compute_key_file_tallies(&files)
    } else {
        None
    };
    let tallies_by_facet = if context.options.include_tallies_by_facet {
        compute_tallies_by_facet(&files)
    } else {
        None
    };

    Output {
        summary,
        tallies,
        tallies_of_key_files,
        tallies_by_facet,
        headers: vec![Header {
            start_timestamp: start_time.to_rfc3339(),
            end_timestamp: end_time.to_rfc3339(),
            duration,
            extra_data,
            errors,
            output_format_version: OUTPUT_FORMAT_VERSION.to_string(),
        }],
        packages,
        dependencies,
        files,
        license_references: context.license_references,
        license_rule_references: context.license_rule_references,
    }
}

fn classify_key_files(files: &mut [FileInfo], packages: &[Package]) {
    let package_roots = build_package_roots(packages);
    let package_file_references = build_package_file_reference_map(files);
    let scan_roots = build_scan_roots(files);
    let package_data_top_level_dirs = build_package_data_top_level_dirs(files);

    for file in files.iter_mut() {
        let path = Path::new(&file.path);
        let is_scan_root_top_level = is_scan_top_level(path, &scan_roots);
        let is_referenced = file.for_packages.iter().any(|uid| {
            package_file_references
                .get(uid)
                .is_some_and(|refs| refs.contains(file.path.as_str()))
        });
        let is_root_top_level = file.for_packages.iter().any(|uid| {
            if file.file_type == FileType::File && !file.package_data.is_empty() {
                return false;
            }
            package_roots
                .get(uid)
                .and_then(|root| path.strip_prefix(root).ok())
                .is_some_and(|relative| relative.components().count() == 1)
        });
        let is_package_data_top_level = if file.file_type == FileType::Directory {
            package_data_top_level_dirs.contains(file.path.as_str())
        } else {
            (!file.package_data.is_empty() && file.is_manifest)
                || path
                    .parent()
                    .and_then(|parent| parent.to_str())
                    .is_some_and(|parent| package_data_top_level_dirs.contains(parent))
        };

        file.is_top_level = is_scan_root_top_level
            || is_referenced
            || is_root_top_level
            || is_package_data_top_level;

        if file.file_type != FileType::File {
            continue;
        }

        file.is_legal = is_legal_file(file);
        file.is_readme = is_readme_file(file);
        file.is_manifest = !file.package_data.is_empty() || is_manifest_file(&file.path);
        file.is_community = is_community_file(file);
        file.is_key_file =
            file.is_top_level && (file.is_legal || file.is_manifest || file.is_readme);
    }
}

fn build_package_data_top_level_dirs(files: &[FileInfo]) -> HashSet<String> {
    let mut top_level_dirs = HashSet::new();

    for file in files.iter().filter(|file| {
        file.file_type == FileType::File
            && !file.package_data.is_empty()
            && !file.for_packages.is_empty()
    }) {
        let path = Path::new(&file.path);
        if path.components().count() <= 2 {
            continue;
        }
        for ancestor in path.ancestors().skip(1) {
            let Some(ancestor_str) = ancestor.to_str() else {
                continue;
            };
            if ancestor_str.is_empty() {
                continue;
            }
            top_level_dirs.insert(ancestor_str.to_string());
        }
    }

    top_level_dirs
}

fn build_package_roots(packages: &[Package]) -> HashMap<String, PathBuf> {
    let mut roots = HashMap::new();
    for package in packages {
        if let Some(root) = package_root(package) {
            roots.insert(package.package_uid.clone(), root);
        }
    }
    roots
}

fn package_root(package: &Package) -> Option<PathBuf> {
    for datafile_path in &package.datafile_paths {
        let path = Path::new(datafile_path);

        if path.file_name().and_then(|n| n.to_str()) == Some("metadata.gz-extract") {
            return path.parent().map(|p| p.to_path_buf());
        }

        if path
            .components()
            .any(|c| c.as_os_str() == "data.gz-extract")
        {
            let mut current = path;
            while let Some(parent) = current.parent() {
                if parent.file_name().and_then(|n| n.to_str()) == Some("data.gz-extract") {
                    return parent.parent().map(|p| p.to_path_buf());
                }
                current = parent;
            }
        }

        if let Some(parent) = path.parent() {
            return Some(parent.to_path_buf());
        }
    }
    None
}

fn build_scan_roots(files: &[FileInfo]) -> Vec<PathBuf> {
    let parent_dirs: Vec<PathBuf> = files
        .iter()
        .filter(|file| file.file_type == FileType::File)
        .map(|file| {
            Path::new(&file.path)
                .parent()
                .unwrap_or_else(|| Path::new(""))
        })
        .map(Path::to_path_buf)
        .collect();

    let mut roots: Vec<PathBuf> = if parent_dirs.iter().any(|path| path.as_os_str().is_empty()) {
        vec![PathBuf::new()]
    } else {
        lowest_common_parent_path(&parent_dirs)
            .into_iter()
            .collect()
    };

    if roots.is_empty() {
        for file in files {
            let mut components = Path::new(&file.path).components();
            let Some(first) = components.next() else {
                continue;
            };

            let root = PathBuf::from(first.as_os_str());
            if !roots.contains(&root) {
                roots.push(root);
            }
        }
    }

    roots
}

fn lowest_common_parent_path(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut paths_iter = paths.iter();
    let first = paths_iter.next()?;
    let mut common_components: Vec<_> = first.components().collect();

    for path in paths_iter {
        let current_components: Vec<_> = path.components().collect();
        let shared_len = common_components
            .iter()
            .zip(current_components.iter())
            .take_while(|(left, right)| left == right)
            .count();
        common_components.truncate(shared_len);
        if common_components.is_empty() {
            break;
        }
    }

    (!common_components.is_empty()).then(|| {
        let mut common_path = PathBuf::new();
        for component in common_components {
            common_path.push(component.as_os_str());
        }
        common_path
    })
}

fn is_scan_top_level(path: &Path, scan_roots: &[PathBuf]) -> bool {
    if path.components().count() == 1 {
        return true;
    }

    scan_roots.iter().any(|root| {
        path == root
            || root.starts_with(path)
            || path
                .strip_prefix(root)
                .ok()
                .is_some_and(|relative| relative.components().count() == 1)
    })
}

fn build_package_file_reference_map(files: &[FileInfo]) -> HashMap<String, HashSet<String>> {
    let mut mapping: HashMap<String, HashSet<String>> = HashMap::new();

    for file in files {
        if file.package_data.is_empty() || file.for_packages.is_empty() {
            continue;
        }

        for package_uid in &file.for_packages {
            let refs = mapping.entry(package_uid.clone()).or_default();
            for pkg_data in &file.package_data {
                for file_ref in &pkg_data.file_references {
                    refs.insert(file_ref.path.clone());
                }
            }
        }
    }

    mapping
}

const LEGAL_STARTS_ENDS: &[&str] = &[
    "copying",
    "copyright",
    "copyrights",
    "copyleft",
    "notice",
    "license",
    "licenses",
    "licence",
    "licences",
    "licensing",
    "licencing",
    "legal",
    "eula",
    "agreement",
    "patent",
    "patents",
];

const MANIFEST_ENDS: &[&str] = &[
    ".about",
    "/bower.json",
    "/project.clj",
    ".podspec",
    "/composer.json",
    "/description",
    "/elm-package.json",
    "/+compact_manifest",
    "+manifest",
    ".gemspec",
    "/metadata",
    "/metadata.gz-extract",
    "/build.gradle",
    ".cabal",
    "/haxelib.json",
    "/package.json",
    ".nuspec",
    ".pod",
    "/meta.yml",
    "/dist.ini",
    "/pipfile",
    "/setup.cfg",
    "/setup.py",
    "/pkg-info",
    "/pyproject.toml",
    ".spec",
    "/cargo.toml",
    ".spdx",
    "/dependencies",
    "debian/copyright",
    "meta-inf/manifest.mf",
];

fn name_or_base_name_matches(file: &FileInfo, patterns: &[&str]) -> bool {
    let name = file.name.to_ascii_lowercase();
    let base_name = file.base_name.to_ascii_lowercase();

    patterns.iter().any(|pattern| {
        name.starts_with(pattern)
            || name.ends_with(pattern)
            || base_name.starts_with(pattern)
            || base_name.ends_with(pattern)
    })
}

fn is_legal_file(file: &FileInfo) -> bool {
    name_or_base_name_matches(file, LEGAL_STARTS_ENDS)
}

fn is_manifest_file(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    MANIFEST_ENDS.iter().any(|ending| lowered.ends_with(ending))
}

fn is_readme_file(file: &FileInfo) -> bool {
    name_or_base_name_matches(file, &["readme"])
}

fn is_community_file(file: &FileInfo) -> bool {
    let clean = |s: &str| s.replace(['_', '-'], "").to_ascii_lowercase();
    let candidates = [clean(&file.name), clean(&file.base_name)];
    [
        "changelog",
        "roadmap",
        "contributing",
        "codeofconduct",
        "authors",
        "security",
        "funding",
    ]
    .iter()
    .any(|prefix| {
        candidates
            .iter()
            .any(|candidate| candidate.starts_with(prefix) || candidate.ends_with(prefix))
    })
}

const FACETS: [&str; 6] = ["core", "dev", "tests", "docs", "data", "examples"];

#[derive(Clone)]
pub(crate) struct FacetRule {
    facet: String,
    pattern: Pattern,
}

pub(crate) fn build_facet_rules(facets: &[String]) -> Result<Vec<FacetRule>> {
    let mut rules = Vec::new();

    for facet_def in facets {
        let Some((raw_facet, raw_pattern)) = facet_def.split_once('=') else {
            return Err(anyhow!(
                "Invalid --facet option: missing <pattern> in \"{}\"",
                facet_def
            ));
        };

        let facet = raw_facet.trim().to_ascii_lowercase();
        let pattern_text = raw_pattern.trim();

        if facet.is_empty() {
            return Err(anyhow!(
                "Invalid --facet option: missing <facet> in \"{}\"",
                facet_def
            ));
        }

        if pattern_text.is_empty() {
            return Err(anyhow!(
                "Invalid --facet option: missing <pattern> in \"{}\"",
                facet_def
            ));
        }

        if !FACETS.contains(&facet.as_str()) {
            return Err(anyhow!(
                "Invalid --facet option: unknown <facet> in \"{}\". Valid values are: {}",
                facet_def,
                FACETS.join(", ")
            ));
        }

        let pattern = Pattern::new(pattern_text).map_err(|err| {
            anyhow!(
                "Invalid --facet option: bad glob pattern in \"{}\": {}",
                facet_def,
                err
            )
        })?;

        if !rules
            .iter()
            .any(|rule: &FacetRule| rule.facet == facet && rule.pattern.as_str() == pattern_text)
        {
            rules.push(FacetRule { facet, pattern });
        }
    }

    Ok(rules)
}

fn assign_facets(files: &mut [FileInfo], facet_rules: &[FacetRule]) {
    if facet_rules.is_empty() {
        return;
    }

    for file in files.iter_mut() {
        if file.file_type != FileType::File {
            file.facets.clear();
            continue;
        }

        let mut facets: Vec<String> = facet_rules
            .iter()
            .filter(|rule| rule.pattern.matches(&file.path) || rule.pattern.matches(&file.name))
            .map(|rule| rule.facet.clone())
            .collect();

        facets.sort();
        facets.dedup();

        file.facets = if facets.is_empty() {
            vec![FACETS[0].to_string()]
        } else {
            facets
        };
    }
}

fn promote_package_metadata_from_key_files(files: &[FileInfo], packages: &mut [Package]) {
    for package in packages.iter_mut() {
        let key_files: Vec<&FileInfo> = files
            .iter()
            .filter(|file| file.is_key_file && file.for_packages.contains(&package.package_uid))
            .collect();

        if key_files.is_empty() {
            continue;
        }

        if package.copyright.is_none() {
            package.copyright = key_files
                .iter()
                .flat_map(|file| file.copyrights.iter())
                .map(|copyright| copyright.copyright.clone())
                .next();
        }

        if package.holder.is_none() {
            let promoted_holders = unique(
                &key_files
                    .iter()
                    .flat_map(|file| file.holders.iter())
                    .map(|holder| holder.holder.clone())
                    .collect::<Vec<_>>(),
            );
            if promoted_holders.len() == 1 {
                package.holder = promoted_holders.into_iter().next();
            }
        }
    }
}

#[cfg(test)]
fn compute_summary(files: &[FileInfo], packages: &[Package]) -> Option<Summary> {
    compute_summary_with_options(files, packages, true, true)
}

fn compute_summary_with_options(
    files: &[FileInfo],
    packages: &[Package],
    include_summary_fields: bool,
    include_license_clarity_score: bool,
) -> Option<Summary> {
    let top_level_package_uids = top_level_package_uids(packages, files);
    let declared_holders = compute_declared_holders(files, packages);
    let declared_holder = (!declared_holders.is_empty()).then(|| declared_holders.join(", "));
    let primary_language = compute_primary_language(files, packages);
    let other_languages = compute_other_languages(files, primary_language.as_deref());
    let tallies = compute_summary_tallies(files, packages).unwrap_or_default();
    let (score_declared_license_expression, score_clarity) =
        compute_license_score(files, packages, &top_level_package_uids);

    if !include_summary_fields
        && !include_license_clarity_score
        && score_declared_license_expression.is_none()
        && declared_holder.is_none()
        && primary_language.is_none()
        && other_languages.is_empty()
    {
        return None;
    }

    let package_declared_license_expression = if include_summary_fields {
        package_declared_license_expression(packages, files, &top_level_package_uids)
    } else {
        None
    };
    let declared_license_expression = package_declared_license_expression
        .clone()
        .or_else(|| score_declared_license_expression.clone());
    let other_license_expressions = remove_tally_value(
        declared_license_expression.as_deref(),
        &tallies.detected_license_expression,
    );
    let mut other_holders = if declared_holders.is_empty() {
        tallies.holders.clone()
    } else {
        remove_tally_values(&declared_holders, &tallies.holders)
    };
    if packages.is_empty()
        && !declared_holders.is_empty()
        && files.iter().any(|file| {
            file.is_top_level && file.is_key_file && file.is_legal && !file.copyrights.is_empty()
        })
    {
        other_holders.retain(|entry| entry.value.is_some());
        if files
            .iter()
            .filter(|file| file.file_type == FileType::File)
            .all(|file| !file.is_key_file || file.is_legal || file.holders.is_empty())
        {
            other_holders.clear();
        }
    }
    if declared_holders.is_empty() && other_holders.iter().all(|entry| entry.value.is_none()) {
        other_holders.clear();
    }
    if !packages.is_empty() && declared_holders.is_empty() {
        other_holders.clear();
    }

    let license_clarity_score = if include_license_clarity_score {
        let mut score_clarity = score_clarity;
        if !score_clarity.declared_copyrights
            && ((!declared_holders.is_empty()
                && files.iter().any(|file| {
                    file.is_top_level
                        && file.is_key_file
                        && file.is_legal
                        && !file.copyrights.is_empty()
                }))
                || (packages.is_empty()
                    && files.iter().any(|file| {
                        file.is_key_file && file.is_legal && !file.copyrights.is_empty()
                    })))
        {
            score_clarity.declared_copyrights = true;
            score_clarity.score += 10;
        }
        Some(score_clarity)
    } else {
        None
    };

    Some(Summary {
        declared_license_expression,
        license_clarity_score,
        declared_holder: include_summary_fields.then(|| declared_holder.unwrap_or_default()),
        primary_language: include_summary_fields.then_some(primary_language).flatten(),
        other_license_expressions: if include_summary_fields {
            other_license_expressions
        } else {
            vec![]
        },
        other_holders: if include_summary_fields {
            other_holders
        } else {
            vec![]
        },
        other_languages: if include_summary_fields {
            other_languages
        } else {
            vec![]
        },
    })
}

const GENERATED_KEYWORDS_LOWERED: &[&str] = &[
    "generated by",
    "auto-generated",
    "automatically generated",
    "generated on",
    "last generated on",
    "do not edit this file",
    "it is machine generated",
    "automatically created by",
    "following schema fragment specifies the",
    "this code is generated",
    "generated by cython",
    "this file was automatically generated by",
    "this file is generated by",
    "generated file, do not edit",
    "this is an autogenerated file",
    "generated by the protocol buffer compiler",
    "generated code -- do not edit",
    "makefile.in generated by automake",
    "generated automatically by aclocal",
    "generated by gnu autoconf",
    "this file was automatically generated",
];

fn mark_generated_files(files: &mut [FileInfo], scanned_root: Option<&Path>) {
    for file in files.iter_mut() {
        if file.file_type != FileType::File {
            file.is_generated = Some(false);
            continue;
        }

        file.is_generated =
            Some(generated_file_hint_exists(&file.path, scanned_root).unwrap_or(false));
    }
}

fn clear_generated_flags(files: &mut [FileInfo]) {
    for file in files {
        file.is_generated = None;
    }
}

fn clear_resource_tallies(files: &mut [FileInfo]) {
    for file in files {
        file.tallies = None;
    }
}

fn generated_file_hint_exists(path: &str, scanned_root: Option<&Path>) -> Result<bool> {
    let path = resolve_generated_scan_path(path, scanned_root)?;
    Ok(!generated_code_hints(&path)?.is_empty())
}

fn generated_code_hints(path: &Path) -> Result<Vec<String>> {
    let content = fs::read(path)?;
    let text = String::from_utf8_lossy(&content);
    let mut hints = Vec::new();

    for line in text.lines().take(150) {
        let lowered = line.trim().to_ascii_lowercase();
        if GENERATED_KEYWORDS_LOWERED
            .iter()
            .any(|keyword| lowered.contains(keyword))
        {
            hints.push(lowered.chars().take(100).collect());
        }
    }

    Ok(hints)
}

fn resolve_generated_scan_path(path: &str, scanned_root: Option<&Path>) -> Result<PathBuf> {
    let relative_path = PathBuf::from(path);
    let candidates = [
        scanned_root.map(|root| root.join(&relative_path)),
        Some(relative_path.clone()),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(anyhow!("Generated detection path not found: {}", path))
}

fn package_declared_license_expression(
    packages: &[Package],
    files: &[FileInfo],
    top_level_package_uids: &HashSet<String>,
) -> Option<String> {
    combine_license_expressions(
        packages
            .iter()
            .filter(|package| top_level_package_uids.contains(&package.package_uid))
            .filter_map(|package| {
                package.declared_license_expression.clone().or_else(|| {
                    package.datafile_paths.iter().find_map(|datafile_path| {
                        files
                            .iter()
                            .find(|file| file.path == *datafile_path)
                            .and_then(|file| file.license_expression.clone())
                    })
                })
            }),
    )
    .map(|expr| canonicalize_summary_expression(&expr))
}

fn compute_license_score(
    files: &[FileInfo],
    packages: &[Package],
    top_level_package_uids: &HashSet<String>,
) -> (Option<String>, LicenseClarityScore) {
    let nested_package_roots = nested_summary_package_roots(packages, files);
    let key_files: Vec<&FileInfo> = files
        .iter()
        .filter(|file| is_summary_score_key_file(file, &nested_package_roots))
        .filter(|file| {
            file.for_packages.is_empty()
                || top_level_package_uids.is_empty()
                || file
                    .for_packages
                    .iter()
                    .any(|uid| top_level_package_uids.contains(uid))
        })
        .collect();
    let non_key_files: Vec<&FileInfo> = files
        .iter()
        .filter(|file| file.file_type == FileType::File)
        .filter(|file| !is_summary_score_key_file(file, &nested_package_roots))
        .collect();

    let key_file_expressions: Vec<String> = key_files
        .iter()
        .filter_map(|file| summary_license_expression(file))
        .collect();
    let primary_declared_license = get_primary_license(&key_file_expressions);

    let mut scoring = LicenseClarityScore {
        score: 0,
        declared_license: key_files
            .iter()
            .any(|file| !file.license_detections.is_empty()),
        identification_precision: key_files
            .iter()
            .flat_map(|file| file.license_detections.iter())
            .flat_map(|detection| detection.matches.iter())
            .any(is_good_match),
        has_license_text: key_files.iter().any(|file| key_file_has_license_text(file)),
        declared_copyrights: key_files
            .iter()
            .any(|file| !file.is_legal && !file.copyrights.is_empty()),
        conflicting_license_categories: false,
        ambiguous_compound_licensing: primary_declared_license.is_none(),
    };

    if scoring.declared_license {
        scoring.score += 40;
    }
    if scoring.identification_precision {
        scoring.score += 40;
    }
    if scoring.has_license_text {
        scoring.score += 10;
    }
    if scoring.declared_copyrights {
        scoring.score += 10;
    }

    let declared_license_expression = primary_declared_license
        .map(|expr| canonicalize_summary_expression(&expr))
        .or_else(|| {
            combine_license_expressions(unique(&key_file_expressions))
                .map(|expr| canonicalize_summary_expression(&expr))
        });

    scoring.conflicting_license_categories = declared_license_expression
        .as_deref()
        .is_some_and(is_permissive_expression)
        && non_key_files
            .iter()
            .filter_map(|file| summary_license_expression(file))
            .map(|expr| expr.to_ascii_lowercase())
            .any(|expr| is_conflicting_expression(&expr));

    if scoring.conflicting_license_categories {
        scoring.score = scoring.score.saturating_sub(20);
    }
    if scoring.ambiguous_compound_licensing {
        scoring.score = scoring.score.saturating_sub(10);
    }

    (declared_license_expression, scoring)
}

fn is_good_match(license_match: &Match) -> bool {
    let score = if license_match.score <= 1.0 {
        license_match.score * 100.0
    } else {
        license_match.score
    };
    match (license_match.match_coverage, license_match.rule_relevance) {
        (Some(coverage), Some(relevance)) => score >= 80.0 && coverage >= 80.0 && relevance >= 80,
        _ => score >= 80.0,
    }
}

fn is_score_key_file(file: &FileInfo) -> bool {
    if !file.is_key_file {
        return false;
    }

    if file.is_manifest {
        return is_score_manifest(file);
    }

    true
}

fn is_score_manifest(file: &FileInfo) -> bool {
    let path = file.path.to_ascii_lowercase();
    path == "cargo.toml"
        || path.ends_with("/cargo.toml")
        || path.ends_with("/pom.xml")
        || path.ends_with("/pom.properties")
        || path == "manifest.mf"
        || path.ends_with("/manifest.mf")
        || path == "metadata.gz-extract"
        || path.ends_with("/metadata.gz-extract")
        || path.ends_with(".gemspec")
}

fn unique(values: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut unique_values = Vec::new();

    for value in values {
        if seen.insert(value.clone()) {
            unique_values.push(value.clone());
        }
    }

    unique_values
}

fn get_primary_license(declared_license_expressions: &[String]) -> Option<String> {
    let unique_declared_license_expressions = unique(declared_license_expressions);
    if unique_declared_license_expressions.len() == 1 {
        return unique_declared_license_expressions.into_iter().next();
    }

    let (unique_joined_expressions, single_expressions) =
        group_license_expressions(&unique_declared_license_expressions);

    if unique_joined_expressions.len() == 1 {
        let joined_expression = unique_joined_expressions[0].clone();
        let joined_upper = joined_expression.to_ascii_uppercase();
        let all_other_expressions_accounted_for = unique_declared_license_expressions
            .iter()
            .filter(|expression| *expression != &joined_expression)
            .all(|expression| joined_upper.contains(expression.to_ascii_uppercase().as_str()));

        if all_other_expressions_accounted_for {
            return Some(joined_expression);
        }
    }

    if unique_joined_expressions.is_empty() {
        return (single_expressions.len() == 1).then(|| single_expressions[0].clone());
    }

    None
}

fn group_license_expressions(expressions: &[String]) -> (Vec<String>, Vec<String>) {
    let mut joined = Vec::new();
    let mut single = Vec::new();

    for expression in expressions {
        let upper = expression.to_ascii_uppercase();
        if upper.contains(" AND ") || upper.contains(" OR ") || upper.contains(" WITH ") {
            joined.push(expression.clone());
        } else {
            single.push(expression.clone());
        }
    }

    if joined.len() <= 1 {
        return (joined, single);
    }

    let mut unique_joined = Vec::new();
    for expression in joined {
        if !unique_joined.contains(&expression) {
            unique_joined.push(expression);
        }
    }

    (unique_joined, single)
}

fn remove_tally_value(value: Option<&str>, tallies: &[TallyEntry]) -> Vec<TallyEntry> {
    tallies
        .iter()
        .filter(|entry| {
            !entry
                .value
                .as_deref()
                .is_some_and(|entry_value| is_redundant_declared_license_tally(entry_value, value))
        })
        .cloned()
        .collect()
}

fn is_redundant_declared_license_tally(entry_value: &str, declared_value: Option<&str>) -> bool {
    let Some(declared_value) = declared_value else {
        return false;
    };

    if entry_value == declared_value {
        return true;
    }

    if declared_value.contains(" AND ")
        || declared_value.contains(" OR ")
        || declared_value.contains(" WITH ")
    {
        return false;
    }

    let normalized_declared = declared_value.trim().to_ascii_lowercase();
    let parts: Vec<String> = entry_value
        .replace(['(', ')'], " ")
        .split_whitespace()
        .filter(|part| !matches!(part.to_ascii_uppercase().as_str(), "AND" | "OR" | "WITH"))
        .map(|part| part.to_ascii_lowercase())
        .collect();

    !parts.is_empty() && parts.iter().all(|part| part == &normalized_declared)
}

fn remove_tally_values(values: &[String], tallies: &[TallyEntry]) -> Vec<TallyEntry> {
    let normalized_values: HashSet<String> = values
        .iter()
        .map(|value| normalize_summary_holder_value(value))
        .collect();

    tallies
        .iter()
        .filter(|entry| {
            !entry.value.as_ref().is_some_and(|value| {
                values.contains(value)
                    || normalized_values.contains(&normalize_summary_holder_value(value))
            })
        })
        .cloned()
        .collect()
}

fn canonicalize_summary_expression(expression: &str) -> String {
    let canonical = parse_expression(expression)
        .map(|parsed| expression_to_string(&simplify_expression(&parsed)))
        .or_else(|_| combine_expressions_and(&[expression], true))
        .unwrap_or_else(|_| expression.to_ascii_lowercase());

    if canonical.contains(" AND ") && !canonical.contains(" OR ") && !canonical.contains(" WITH ") {
        canonical
            .replace(['(', ')'], "")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        canonical
    }
}

fn normalize_summary_holder_value(value: &str) -> String {
    let normalized = canonicalize_summary_holder_display(value)
        .trim_end_matches(['.', ',', ';', ':'])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

    let key: String = normalized
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect();

    match key.as_str() {
        "google" | "googlellc" | "googleinc" => "google".to_string(),
        "microsoft" | "microsoftcorp" | "microsoftinc" | "microsoftcorporation" => {
            "microsoft".to_string()
        }
        _ => normalized,
    }
}

fn canonicalize_summary_holder_display(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");

    let key: String = normalized
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();

    match key.as_str() {
        "google" | "googlellc" | "googleinc" => "Google".to_string(),
        "microsoft" | "microsoftcorp" | "microsoftinc" | "microsoftcorporation" => {
            "Microsoft".to_string()
        }
        "sunmicrosystems" | "sunmicrosystemsinc" => "Sun Microsystems".to_string(),
        _ => normalized,
    }
}

fn summary_holder_from_copyright(copyright: &str) -> Option<String> {
    let mut value = copyright.trim();
    if value.is_empty() {
        return None;
    }

    if value.len() >= "copyright".len()
        && value[.."copyright".len()].eq_ignore_ascii_case("copyright")
    {
        value = value["copyright".len()..].trim_start();
    }

    if let Some(stripped) = value.strip_prefix("(c)") {
        value = stripped.trim_start();
    }
    if let Some(stripped) = value.strip_prefix('©') {
        value = stripped.trim_start();
    }

    let cleaned = value.trim_matches(|ch: char| ch.is_whitespace() || ch == '.' || ch == ',');
    if cleaned.is_empty() {
        return None;
    }

    if cleaned.starts_with("Holders ") || cleaned.contains("option either") {
        return None;
    }

    let cleaned = cleaned
        .strip_suffix(". Individual")
        .unwrap_or(cleaned)
        .trim();

    (!cleaned.is_empty()).then(|| cleaned.to_string())
}

fn clean_legal_holder_candidate(holder: &str) -> Option<String> {
    let cleaned = holder.trim();
    if cleaned.is_empty()
        || cleaned.contains("option either")
        || cleaned.starts_with("messages,")
        || cleaned.starts_with("together with instructions")
    {
        return None;
    }

    let cleaned = cleaned
        .strip_suffix(". Individual")
        .unwrap_or(cleaned)
        .trim();

    (!cleaned.is_empty()).then(|| cleaned.to_string())
}

fn summary_license_expression(file: &FileInfo) -> Option<String> {
    let detection_expressions = unique(
        &file
            .license_detections
            .iter()
            .map(|detection| detection.license_expression.clone())
            .collect::<Vec<_>>(),
    );

    if !detection_expressions.is_empty() {
        return if detection_expressions.len() == 1 {
            detection_expressions
                .into_iter()
                .next()
                .map(|expr| canonicalize_summary_expression(&expr))
        } else {
            combine_license_expressions(detection_expressions)
                .map(|expr| canonicalize_summary_expression(&expr))
        };
    }

    file.license_expression
        .as_deref()
        .map(canonicalize_summary_expression)
}

fn key_file_has_license_text(file: &FileInfo) -> bool {
    file.license_detections
        .iter()
        .flat_map(|detection| detection.matches.iter())
        .any(|m| {
            m.matched_length.unwrap_or_default() > 1 || m.match_coverage.unwrap_or_default() > 1.0
        })
}

fn is_permissive_expression(expression: &str) -> bool {
    ["apache", "mit", "bsd", "zlib", "isc", "cc0", "boost"]
        .iter()
        .any(|needle| expression.contains(needle))
}

fn is_conflicting_expression(expression: &str) -> bool {
    ["gpl", "agpl", "lgpl", "copyleft", "proprietary"]
        .iter()
        .any(|needle| expression.contains(needle))
}

fn compute_tallies(files: &[FileInfo]) -> Option<Tallies> {
    let detected_license_expression = tally_file_values(files, detected_license_values, true);
    let copyrights = tally_file_values(files, copyright_values, true);
    let holders = tally_file_values(files, holder_values, true);
    let authors = tally_file_values(files, author_values, true);
    let programming_language = tally_file_values(files, programming_language_values, false);

    let tallies = Tallies {
        detected_license_expression,
        copyrights,
        holders,
        authors,
        programming_language,
    };

    (!tallies.is_empty()).then_some(tallies)
}

fn compute_summary_tallies(files: &[FileInfo], packages: &[Package]) -> Option<Tallies> {
    let summary_origin_package_uids: HashSet<String> = summary_origin_packages(packages, files)
        .into_iter()
        .map(|package| package.package_uid.clone())
        .collect();
    let nested_package_roots = nested_summary_package_roots(packages, files);
    let detected_license_expression = tally_file_values_filtered(
        files,
        |file| {
            !file
                .package_data
                .iter()
                .any(|package_data| package_data.datasource_id == Some(DatasourceId::PypiSetupCfg))
        },
        summary_detected_license_values,
        true,
    );
    let copyrights = tally_file_values(files, copyright_values, true);
    let holders = if packages.is_empty() {
        tally_file_values(
            files,
            |file| {
                file.holders
                    .iter()
                    .map(|holder| holder.holder.clone())
                    .collect()
            },
            true,
        )
    } else {
        tally_file_values_filtered(
            files,
            |file| {
                file.is_community
                    || (file.is_top_level
                        && file.is_key_file
                        && !nested_package_roots
                            .iter()
                            .any(|root| Path::new(&file.path).starts_with(root))
                        && (file.for_packages.is_empty()
                            || summary_origin_package_uids.is_empty()
                            || file
                                .for_packages
                                .iter()
                                .any(|uid| summary_origin_package_uids.contains(uid))))
            },
            |file| {
                file.holders
                    .iter()
                    .map(|holder| holder.holder.clone())
                    .collect()
            },
            true,
        )
    };
    let authors = tally_file_values(files, author_values, true);
    let programming_language = tally_file_values(files, programming_language_values, false);

    let tallies = Tallies {
        detected_license_expression,
        copyrights,
        holders,
        authors,
        programming_language,
    };

    (!tallies.is_empty()).then_some(tallies)
}

fn compute_key_file_tallies(files: &[FileInfo]) -> Option<Tallies> {
    if !files
        .iter()
        .any(|file| file.file_type == FileType::File && file.is_key_file)
    {
        return None;
    }

    let tallies = Tallies {
        detected_license_expression: tally_file_values_filtered(
            files,
            |file| file.is_key_file,
            detected_license_values,
            false,
        ),
        copyrights: tally_file_values_filtered(
            files,
            |file| file.is_key_file,
            copyright_values,
            false,
        ),
        holders: tally_file_values_filtered(files, |file| file.is_key_file, holder_values, false),
        authors: tally_file_values_filtered(files, |file| file.is_key_file, author_values, false),
        programming_language: tally_file_values_filtered(
            files,
            |file| file.is_key_file,
            programming_language_values,
            false,
        ),
    };

    (!tallies.is_empty()).then_some(tallies)
}

fn compute_tallies_by_facet(files: &[FileInfo]) -> Option<Vec<FacetTallies>> {
    let mut buckets: HashMap<&'static str, Tallies> = FACETS
        .iter()
        .map(|facet| (*facet, Tallies::default()))
        .collect();

    for file in files.iter().filter(|file| file.file_type == FileType::File) {
        if file.facets.is_empty() {
            continue;
        }

        let Some(file_tallies) = file.tallies.as_ref() else {
            continue;
        };

        for facet in &file.facets {
            let Some(bucket) = buckets.get_mut(facet.as_str()) else {
                continue;
            };
            merge_non_null_tally_entries(
                &mut bucket.detected_license_expression,
                &file_tallies.detected_license_expression,
            );
            merge_non_null_tally_entries(&mut bucket.copyrights, &file_tallies.copyrights);
            merge_non_null_tally_entries(&mut bucket.holders, &file_tallies.holders);
            merge_non_null_tally_entries(&mut bucket.authors, &file_tallies.authors);
            merge_non_null_tally_entries(
                &mut bucket.programming_language,
                &file_tallies.programming_language,
            );
        }
    }

    Some(
        FACETS
            .iter()
            .map(|facet| FacetTallies {
                facet: (*facet).to_string(),
                tallies: buckets.remove(facet).unwrap_or_default(),
            })
            .collect(),
    )
}

fn compute_detailed_tallies(files: &mut [FileInfo]) {
    let mut children_by_parent: HashMap<String, Vec<usize>> = HashMap::new();
    let known_paths: HashSet<String> = files.iter().map(|file| file.path.clone()).collect();

    for (idx, file) in files.iter().enumerate() {
        let Some(parent) = parent_path(&file.path) else {
            continue;
        };
        if known_paths.contains(parent.as_str()) {
            children_by_parent.entry(parent).or_default().push(idx);
        }
    }

    let mut indices: Vec<usize> = (0..files.len()).collect();
    indices.sort_by_key(|&idx| std::cmp::Reverse(path_depth(&files[idx].path)));

    for idx in indices {
        let tallies = if files[idx].file_type == FileType::File {
            compute_direct_file_tallies(&files[idx])
        } else {
            aggregate_child_tallies(
                children_by_parent
                    .get(files[idx].path.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                files,
            )
        };
        files[idx].tallies = Some(tallies);
    }
}

fn parent_path(path: &str) -> Option<String> {
    Path::new(path)
        .parent()
        .and_then(|parent| parent.to_str())
        .filter(|parent| !parent.is_empty())
        .map(str::to_string)
}

fn path_depth(path: &str) -> usize {
    Path::new(path).components().count()
}

fn compute_direct_file_tallies(file: &FileInfo) -> Tallies {
    Tallies {
        detected_license_expression: build_direct_tally_entries(
            detected_license_values(file),
            true,
        ),
        copyrights: build_direct_tally_entries(copyright_values(file), true),
        holders: build_direct_tally_entries(holder_values(file), true),
        authors: build_direct_tally_entries(author_values(file), true),
        programming_language: build_direct_tally_entries(programming_language_values(file), false),
    }
}

fn aggregate_child_tallies(child_indices: &[usize], files: &[FileInfo]) -> Tallies {
    let mut detected_license_expression = HashMap::new();
    let mut copyrights = HashMap::new();
    let mut holders = HashMap::new();
    let mut authors = HashMap::new();
    let mut programming_language = HashMap::new();

    for &child_idx in child_indices {
        let Some(child_tallies) = files[child_idx].tallies.as_ref() else {
            continue;
        };

        merge_tally_entries(
            &mut detected_license_expression,
            &child_tallies.detected_license_expression,
        );
        merge_tally_entries(&mut copyrights, &child_tallies.copyrights);
        merge_tally_entries(&mut holders, &child_tallies.holders);
        merge_tally_entries(&mut authors, &child_tallies.authors);
        merge_tally_entries(
            &mut programming_language,
            &child_tallies.programming_language,
        );
    }

    Tallies {
        detected_license_expression: build_tally_entries(detected_license_expression),
        copyrights: build_tally_entries(copyrights),
        holders: build_tally_entries(holders),
        authors: build_tally_entries(authors),
        programming_language: build_tally_entries(programming_language),
    }
}

fn build_direct_tally_entries(values: Vec<String>, count_missing: bool) -> Vec<TallyEntry> {
    let mut counts: HashMap<Option<String>, usize> = HashMap::new();

    if values.is_empty() {
        if count_missing {
            counts.insert(None, 1);
        }
    } else {
        for value in values {
            *counts.entry(Some(value)).or_insert(0) += 1;
        }
    }

    build_tally_entries(counts)
}

fn merge_tally_entries(counts: &mut HashMap<Option<String>, usize>, entries: &[TallyEntry]) {
    for entry in entries {
        *counts.entry(entry.value.clone()).or_insert(0) += entry.count;
    }
}

fn merge_non_null_tally_entries(destination: &mut Vec<TallyEntry>, entries: &[TallyEntry]) {
    let mut counts: HashMap<Option<String>, usize> = destination
        .iter()
        .cloned()
        .map(|entry| (entry.value, entry.count))
        .collect();

    for entry in entries.iter().filter(|entry| entry.value.is_some()) {
        *counts.entry(entry.value.clone()).or_insert(0) += entry.count;
    }

    *destination = build_tally_entries(counts)
        .into_iter()
        .filter(|entry| entry.value.is_some())
        .collect();
}

fn tally_file_values<F>(
    files: &[FileInfo],
    values_for_file: F,
    count_missing_files: bool,
) -> Vec<TallyEntry>
where
    F: Fn(&FileInfo) -> Vec<String>,
{
    tally_file_values_filtered(files, |_| true, values_for_file, count_missing_files)
}

fn tally_file_values_filtered<P, F>(
    files: &[FileInfo],
    predicate: P,
    values_for_file: F,
    count_missing_files: bool,
) -> Vec<TallyEntry>
where
    P: Fn(&FileInfo) -> bool,
    F: Fn(&FileInfo) -> Vec<String>,
{
    let mut counts: HashMap<Option<String>, usize> = HashMap::new();

    for file in files
        .iter()
        .filter(|file| file.file_type == FileType::File && predicate(file))
    {
        let values = values_for_file(file);
        if values.is_empty() {
            if count_missing_files {
                *counts.entry(None).or_insert(0) += 1;
            }
            continue;
        }

        for value in values {
            *counts.entry(Some(value)).or_insert(0) += 1;
        }
    }

    build_tally_entries(counts)
}

fn detected_license_values(file: &FileInfo) -> Vec<String> {
    let detection_expressions: Vec<String> = file
        .license_detections
        .iter()
        .map(|detection| canonicalize_summary_expression(&detection.license_expression))
        .collect();

    if detection_expressions.is_empty() {
        return file
            .license_expression
            .as_deref()
            .map(canonicalize_summary_expression)
            .into_iter()
            .collect();
    }

    let unique_detection_expressions = unique(&detection_expressions);

    if unique_detection_expressions.len() == 1 {
        return unique_detection_expressions;
    }

    combine_license_expressions(unique_detection_expressions)
        .into_iter()
        .collect()
}

fn summary_detected_license_values(file: &FileInfo) -> Vec<String> {
    let detection_expressions: Vec<String> = file
        .license_detections
        .iter()
        .map(|detection| canonicalize_summary_expression(&detection.license_expression))
        .collect();

    if detection_expressions.is_empty() {
        return file
            .license_expression
            .as_deref()
            .map(canonicalize_summary_expression)
            .into_iter()
            .collect();
    }

    detection_expressions
}

fn copyright_values(file: &FileInfo) -> Vec<String> {
    file.copyrights
        .iter()
        .map(|copyright| copyright.copyright.clone())
        .collect()
}

fn holder_values(file: &FileInfo) -> Vec<String> {
    file.holders
        .iter()
        .map(|holder| holder.holder.clone())
        .collect()
}

fn author_values(file: &FileInfo) -> Vec<String> {
    file.authors
        .iter()
        .map(|author| author.author.clone())
        .collect()
}

fn programming_language_values(file: &FileInfo) -> Vec<String> {
    file.programming_language.clone().into_iter().collect()
}

fn build_tally_entries(counts: HashMap<Option<String>, usize>) -> Vec<TallyEntry> {
    let mut tallies: Vec<TallyEntry> = counts
        .into_iter()
        .map(|(value, count)| TallyEntry { value, count })
        .collect();

    tallies.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.value.cmp(&right.value))
    });
    tallies
}

fn compute_declared_holders(files: &[FileInfo], packages: &[Package]) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for holder in packages
        .iter()
        .filter_map(|package| package.holder.as_ref())
    {
        *counts
            .entry(canonicalize_summary_holder_display(holder))
            .or_insert(0) += 1;
    }

    let mut package_datafile_holders = Vec::new();

    if counts.is_empty() {
        for package in packages {
            for datafile_path in &package.datafile_paths {
                if let Some(file) = files.iter().find(|file| file.path == *datafile_path) {
                    if file.is_legal {
                        continue;
                    }
                    for holder in &file.holders {
                        let canonical_holder = canonicalize_summary_holder_display(&holder.holder);
                        if !package_datafile_holders.contains(&canonical_holder) {
                            package_datafile_holders.push(canonical_holder);
                        }
                    }
                }
            }
        }
    }

    if !package_datafile_holders.is_empty() {
        return package_datafile_holders;
    }

    if counts.is_empty() {
        let mut key_file_holders = Vec::new();
        for holder in files
            .iter()
            .filter(|file| file.is_key_file && !file.is_legal)
            .flat_map(|file| file.holders.iter())
            .map(|holder| canonicalize_summary_holder_display(&holder.holder))
        {
            if !key_file_holders.contains(&holder) {
                key_file_holders.push(holder);
            }
        }

        let mut codebase_holder_counts: HashMap<String, usize> = HashMap::new();
        for holder in files
            .iter()
            .flat_map(|file| file.holders.iter())
            .map(|holder| canonicalize_summary_holder_display(&holder.holder))
        {
            *codebase_holder_counts.entry(holder).or_insert(0) += 1;
        }

        let highest_count = key_file_holders
            .iter()
            .filter_map(|holder| codebase_holder_counts.get(holder).copied())
            .max();

        if let Some(highest_count) = highest_count {
            let highest_key_file_holders: Vec<String> = key_file_holders
                .iter()
                .filter(|holder| codebase_holder_counts.get(*holder) == Some(&highest_count))
                .cloned()
                .collect();
            if !highest_key_file_holders.is_empty() {
                return highest_key_file_holders;
            }
        }

        if !key_file_holders.is_empty() {
            return key_file_holders;
        }

        if packages.is_empty() {
            let mut legal_key_file_holders = Vec::new();
            for holder in files
                .iter()
                .filter(|file| file.is_key_file && file.is_legal)
                .flat_map(|file| {
                    let explicit_holders: Vec<String> = file
                        .holders
                        .iter()
                        .filter_map(|holder| clean_legal_holder_candidate(&holder.holder))
                        .map(|holder| canonicalize_summary_holder_display(&holder))
                        .collect();
                    if explicit_holders.is_empty() {
                        file.copyrights
                            .iter()
                            .filter_map(|copyright| {
                                summary_holder_from_copyright(&copyright.copyright)
                                    .map(|holder| canonicalize_summary_holder_display(&holder))
                            })
                            .collect::<Vec<_>>()
                    } else {
                        explicit_holders
                    }
                })
            {
                if !legal_key_file_holders.contains(&holder) {
                    legal_key_file_holders.push(holder);
                }
            }

            if !legal_key_file_holders.is_empty() {
                return legal_key_file_holders;
            }
        }
    }

    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(holder, _)| holder)
        .into_iter()
        .collect()
}

fn compute_primary_language(files: &[FileInfo], packages: &[Package]) -> Option<String> {
    let package_languages = unique(
        &summary_origin_packages(packages, files)
            .into_iter()
            .filter_map(summary_origin_package_primary_language)
            .collect::<Vec<_>>(),
    );

    if package_languages.len() == 1 {
        return package_languages.into_iter().next();
    }

    let mut counts: HashMap<String, usize> = HashMap::new();

    for language in files
        .iter()
        .filter_map(|file| file.programming_language.as_ref())
        .filter(|language| language.as_str() != "Text")
    {
        *counts.entry(language.clone()).or_insert(0) += 1;
    }

    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(language, _)| language)
}

fn summary_origin_package_primary_language(package: &Package) -> Option<String> {
    package
        .primary_language
        .clone()
        .or_else(|| match package.package_type {
            Some(crate::models::PackageType::Pypi) => Some("Python".to_string()),
            _ => None,
        })
}

fn summary_origin_packages<'a>(packages: &'a [Package], files: &[FileInfo]) -> Vec<&'a Package> {
    if packages.is_empty() {
        return Vec::new();
    }

    let top_level_roots = top_level_summary_package_roots(packages);
    if top_level_roots.is_empty() {
        return packages.iter().collect();
    }

    let top_level_packages: Vec<&Package> = packages
        .iter()
        .filter(|package| {
            package_root(package)
                .as_ref()
                .is_some_and(|root| top_level_roots.iter().any(|top_level| top_level == root))
        })
        .collect();

    if top_level_packages.is_empty() && !files.is_empty() {
        return packages.iter().collect();
    }

    top_level_packages
}

fn top_level_package_uids(packages: &[Package], files: &[FileInfo]) -> HashSet<String> {
    let top_level_packages = summary_origin_packages(packages, files);
    let key_package_uids: HashSet<String> = top_level_packages
        .iter()
        .filter(|package| {
            package.datafile_paths.iter().any(|datafile_path| {
                files.iter().any(|file| {
                    file.path == *datafile_path
                        && file.file_type == FileType::File
                        && file.is_top_level
                        && file.is_key_file
                })
            })
        })
        .map(|package| package.package_uid.clone())
        .collect();

    if key_package_uids.is_empty() {
        top_level_packages
            .into_iter()
            .map(|package| package.package_uid.clone())
            .collect()
    } else {
        key_package_uids
    }
}

fn top_level_summary_package_roots(packages: &[Package]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = packages.iter().filter_map(package_root).collect();
    roots.sort_by(|left, right| {
        left.components()
            .count()
            .cmp(&right.components().count())
            .then_with(|| left.cmp(right))
    });
    roots.dedup();

    let mut top_level_roots = Vec::new();
    for root in roots {
        if top_level_roots
            .iter()
            .any(|top_level| root.starts_with(top_level))
        {
            continue;
        }
        top_level_roots.push(root);
    }

    top_level_roots
}

fn nested_summary_package_roots(packages: &[Package], files: &[FileInfo]) -> Vec<PathBuf> {
    let top_level_roots = top_level_summary_package_roots(packages);
    let mut nested_roots: Vec<PathBuf> = packages
        .iter()
        .filter_map(package_root)
        .filter(|root| {
            top_level_roots
                .iter()
                .any(|top_level| root != top_level && root.starts_with(top_level))
        })
        .collect();

    nested_roots.extend(
        files
            .iter()
            .filter(|file| {
                file.file_type == FileType::File && file.is_manifest && !file.is_top_level
            })
            .map(|file| {
                Path::new(&file.path)
                    .parent()
                    .unwrap_or_else(|| Path::new(&file.path))
            })
            .map(Path::to_path_buf),
    );

    nested_roots.sort();
    nested_roots.dedup();
    nested_roots
}

fn is_summary_score_key_file(file: &FileInfo, nested_package_roots: &[PathBuf]) -> bool {
    file.file_type == FileType::File
        && file.is_top_level
        && is_score_key_file(file)
        && !nested_package_roots
            .iter()
            .any(|root| Path::new(&file.path).starts_with(root))
}

fn compute_other_languages(files: &[FileInfo], primary_language: Option<&str>) -> Vec<TallyEntry> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for language in files
        .iter()
        .filter(|file| file.file_type == FileType::File && !file.is_key_file)
        .filter_map(|file| file.programming_language.as_ref())
        .filter(|language| language.as_str() != "Text")
    {
        *counts.entry(language.clone()).or_insert(0) += 1;
    }

    let mut tallies: Vec<TallyEntry> = counts
        .into_iter()
        .filter(|(language, _)| Some(language.as_str()) != primary_language)
        .map(|(language, count)| TallyEntry {
            value: Some(language),
            count,
        })
        .collect();

    tallies.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.value.cmp(&right.value))
    });
    tallies
}
