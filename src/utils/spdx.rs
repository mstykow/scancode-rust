use std::collections::HashSet;

use include_dir::{Dir, include_dir};
use once_cell::sync::Lazy;
use serde_json::{Value, from_str};

use crate::askalono::{Store, TextData};

const LICENSES_DIR: Dir = include_dir!("resources/licenses/json/details");

static LICENSE_STORE: Lazy<Store> = Lazy::new(|| {
    let mut store = Store::new();
    for file in LICENSES_DIR.files() {
        if let Some(string_content) = file.contents_utf8()
            && let Ok(value) = from_str::<Value>(string_content)
        {
            if value["isDeprecatedLicenseId"].as_bool().unwrap_or(false) {
                continue;
            }
            if let (Some(name), Some(text)) =
                (value["licenseId"].as_str(), value["licenseText"].as_str())
            {
                store.add_license(name.to_string(), TextData::new(text));
            }
        }
    }
    store
});

pub fn get_license_store() -> &'static Store {
    &LICENSE_STORE
}

/// Combines multiple license expressions into a single SPDX expression.
/// Deduplicates, sorts, and combines the expressions with " AND ".
pub fn combine_license_expressions(
    expressions: impl IntoIterator<Item = String>,
) -> Option<String> {
    let unique_expressions: HashSet<String> = expressions.into_iter().collect();
    if unique_expressions.is_empty() {
        return None;
    }

    let mut sorted_expressions: Vec<String> = unique_expressions.into_iter().collect();
    sorted_expressions.sort(); // Sort for consistent output

    // Join multiple expressions with AND, wrapping individual expressions in parentheses if needed
    let combined = sorted_expressions
        .iter()
        .map(|expr| {
            // If expression contains spaces and isn't already wrapped in parentheses,
            // it might have operators, so wrap it
            if expr.contains(' ') && !(expr.starts_with('(') && expr.ends_with(')')) {
                format!("({})", expr)
            } else {
                expr.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ");

    Some(combined)
}
