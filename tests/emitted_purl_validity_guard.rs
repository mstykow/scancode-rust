// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

//! Guards every PURL-shaped string in the checked-in expected fixtures against
//! being unparsable.
//!
//! Expected fixtures are generated from the parsers, so they are a broad sample
//! of what the parsers actually emit. Nothing else enforces PURL validity:
//! `normalize_purl` is a normalizer and returns unparsable input unchanged, and
//! parsers historically assembled PURLs with `format!`, splicing unvalidated
//! names straight in — which is how `pkg:pypi/::`, `pkg:apk/alpine/musl>=1.2.0`
//! and `pkg:osgi/my bundle?x@1.0.0` came to ship.
//!
//! This is deliberately a check rather than a runtime filter. Dropping an
//! unparsable PURL at the output boundary would erase the component from SBOM
//! output entirely — the PURL doubles as its identity there — and would discard
//! caller-supplied values on the `--from-json` path. The parsers are the right
//! place to be correct; this is the guard that keeps them so.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use packageurl::PackageUrl;
use serde_json::Value;

/// Fields whose values are PURLs, or PURLs carrying a `uuid` qualifier.
const PURL_FIELDS: &[&str] = &[
    "purl",
    "package_uid",
    "dependency_uid",
    "for_package_uid",
    "for_packages",
    "source_packages",
];

/// The one identity Provenant emits that is deliberately not a PURL: the
/// fallback used when a package has no resolvable coordinates. It is prefixed so
/// that a consumer fails loudly rather than mis-parsing it as one.
const NON_PURL_IDENTITY_PREFIX: &str = "generated-package:";

fn expected_fixture_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(expected_fixture_files(&path));
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.contains("expected") && (name.ends_with(".json") || name.ends_with(".expected")) {
            found.push(path);
        }
    }

    found
}

fn collect_purls(value: &Value, into: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if PURL_FIELDS.contains(&key.as_str()) {
                    match child {
                        Value::String(purl) => {
                            into.insert(purl.clone());
                        }
                        Value::Array(items) => {
                            for item in items.iter().filter_map(Value::as_str) {
                                into.insert(item.to_string());
                            }
                        }
                        _ => {}
                    }
                }
                collect_purls(child, into);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_purls(item, into);
            }
        }
        _ => {}
    }
}

#[test]
fn every_emitted_purl_in_expected_fixtures_parses() {
    let fixtures = expected_fixture_files(Path::new("testdata"));
    assert!(
        fixtures.len() > 100,
        "expected to find the fixture corpus, found {} files",
        fixtures.len()
    );

    let mut purls = BTreeSet::new();
    for fixture in &fixtures {
        let Ok(contents) = fs::read_to_string(fixture) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&contents) else {
            // Not every `.expected` file is JSON; those carry no PURL fields.
            continue;
        };
        collect_purls(&value, &mut purls);
    }

    assert!(
        purls.len() > 500,
        "expected a broad PURL sample, found {}",
        purls.len()
    );

    let unparsable: Vec<&String> = purls
        .iter()
        .filter(|purl| !purl.is_empty())
        .filter(|purl| !purl.starts_with(NON_PURL_IDENTITY_PREFIX))
        .filter(|purl| PackageUrl::from_str(purl).is_err())
        .collect();

    assert!(
        unparsable.is_empty(),
        "these emitted PURLs cannot be parsed:\n  {}",
        unparsable
            .iter()
            .map(|purl| purl.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

#[test]
fn every_emitted_purl_in_expected_fixtures_keeps_its_components() {
    // Component stability rather than string equality: re-emitting a parsed PURL
    // must yield the same type, namespace, name, version, qualifiers and
    // subpath. This catches a component landing in the wrong slot — a `#` in a
    // name becoming a subpath, or a `uuid` qualifier swallowed into one — while
    // tolerating the two places Provenant deliberately encodes differently from
    // the crate (`$`/`'` in versions, and golang namespace case).
    let mut purls = BTreeSet::new();
    for fixture in expected_fixture_files(Path::new("testdata")) {
        let Ok(contents) = fs::read_to_string(&fixture) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&contents) else {
            continue;
        };
        collect_purls(&value, &mut purls);
    }

    let mut unstable = Vec::new();
    for purl in purls
        .iter()
        .filter(|purl| !purl.is_empty())
        .filter(|purl| !purl.starts_with(NON_PURL_IDENTITY_PREFIX))
    {
        let Ok(parsed) = PackageUrl::from_str(purl) else {
            continue; // reported by the parse test above
        };
        let reparsed = match PackageUrl::from_str(&parsed.to_string()) {
            Ok(reparsed) => reparsed,
            Err(error) => {
                unstable.push(format!("{purl} -> re-parse failed: {error}"));
                continue;
            }
        };

        if parsed.ty() != reparsed.ty()
            || parsed.namespace() != reparsed.namespace()
            || parsed.name() != reparsed.name()
            || parsed.version() != reparsed.version()
            || parsed.subpath() != reparsed.subpath()
        {
            unstable.push(format!("{purl} -> {}", reparsed));
            continue;
        }

        let qualifiers: BTreeSet<(&str, &str)> = parsed
            .qualifiers()
            .iter()
            .map(|(key, value)| (key.as_ref(), value.as_ref()))
            .collect();
        let reparsed_qualifiers: BTreeSet<(&str, &str)> = reparsed
            .qualifiers()
            .iter()
            .map(|(key, value)| (key.as_ref(), value.as_ref()))
            .collect();
        if qualifiers != reparsed_qualifiers {
            unstable.push(format!("{purl} -> qualifiers changed"));
        }
    }

    assert!(
        unstable.is_empty(),
        "these PURLs do not survive a parse/emit round trip with their components intact:\n  {}",
        unstable.join("\n  ")
    );
}
