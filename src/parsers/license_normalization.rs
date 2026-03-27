use std::sync::LazyLock;

use crate::parser_warn as warn;

use crate::license_detection::LicenseDetectionEngine;
use crate::license_detection::expression::{
    LicenseExpression, parse_expression, simplify_expression,
};
use crate::license_detection::index::LicenseIndex;
use crate::models::{LicenseDetection, Match};
use crate::utils::spdx::{ExpressionRelation, combine_license_expressions_with_relation};

pub(crate) const PARSER_DECLARED_MATCHER: &str = "parser-declared-license";

static PARSER_LICENSE_ENGINE: LazyLock<Option<LicenseDetectionEngine>> = LazyLock::new(|| {
    match LicenseDetectionEngine::from_embedded() {
        Ok(engine) => Some(engine),
        Err(error) => {
            warn!(
                "Failed to initialize embedded license engine for parser declared-license normalization: {}",
                error
            );
            None
        }
    }
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedDeclaredLicense {
    pub(crate) declared_license_expression: String,
    pub(crate) declared_license_expression_spdx: String,
}

impl NormalizedDeclaredLicense {
    pub(crate) fn new(
        declared_license_expression: impl Into<String>,
        declared_license_expression_spdx: impl Into<String>,
    ) -> Self {
        Self {
            declared_license_expression: declared_license_expression.into(),
            declared_license_expression_spdx: declared_license_expression_spdx.into(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DeclaredLicenseMatchMetadata<'a> {
    pub(crate) matched_text: &'a str,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
}

impl<'a> DeclaredLicenseMatchMetadata<'a> {
    pub(crate) fn new(matched_text: &'a str, start_line: usize, end_line: usize) -> Self {
        Self {
            matched_text,
            start_line,
            end_line,
        }
    }

    pub(crate) fn single_line(matched_text: &'a str) -> Self {
        Self::new(matched_text, 1, 1)
    }
}

pub(crate) fn empty_declared_license_data()
-> (Option<String>, Option<String>, Vec<LicenseDetection>) {
    (None, None, Vec::new())
}

pub(crate) fn normalize_spdx_declared_license(
    statement: Option<&str>,
) -> (Option<String>, Option<String>, Vec<LicenseDetection>) {
    let Some(statement) = statement.map(str::trim).filter(|value| !value.is_empty()) else {
        return empty_declared_license_data();
    };

    let Some(normalized) = normalize_spdx_expression(statement) else {
        return empty_declared_license_data();
    };

    build_declared_license_data(
        normalized,
        DeclaredLicenseMatchMetadata::single_line(statement),
    )
}

pub(crate) fn normalize_spdx_expression(statement: &str) -> Option<NormalizedDeclaredLicense> {
    let statement = statement.trim();
    if statement.is_empty() {
        return None;
    }

    let engine = PARSER_LICENSE_ENGINE.as_ref()?;
    let expression = parse_expression(statement).ok()?;
    let (declared_ast, declared_spdx_ast) = normalize_expression_ast(&expression, engine.index())?;
    let declared_ast = simplify_expression(&declared_ast);
    let declared_spdx_ast = simplify_expression(&declared_spdx_ast);

    Some(NormalizedDeclaredLicense::new(
        render_canonical_expression(&declared_ast),
        render_canonical_expression(&declared_spdx_ast),
    ))
}

pub(crate) fn normalize_declared_license_key(key: &str) -> Option<NormalizedDeclaredLicense> {
    let key = key.trim();
    if key.is_empty() {
        return None;
    }

    let engine = PARSER_LICENSE_ENGINE.as_ref()?;
    normalize_license_key(key, engine.index())
}

pub(crate) fn combine_normalized_licenses(
    licenses: Vec<NormalizedDeclaredLicense>,
    separator: &str,
) -> Option<NormalizedDeclaredLicense> {
    if licenses.is_empty() {
        return None;
    }

    if licenses.len() == 1 {
        return licenses.into_iter().next();
    }

    let relation = match separator {
        " AND " => ExpressionRelation::And,
        " OR " => ExpressionRelation::Or,
        _ => {
            let declared_expression = licenses
                .iter()
                .map(|license| license.declared_license_expression.clone())
                .collect::<Vec<_>>()
                .join(separator);
            let declared_spdx_expression = licenses
                .iter()
                .map(|license| license.declared_license_expression_spdx.clone())
                .collect::<Vec<_>>()
                .join(separator);

            return Some(NormalizedDeclaredLicense::new(
                declared_expression,
                declared_spdx_expression,
            ));
        }
    };

    let declared_expression = combine_license_expressions_with_relation(
        licenses
            .iter()
            .map(|license| license.declared_license_expression.clone()),
        relation,
    )?;
    let declared_spdx_expression = combine_license_expressions_with_relation(
        licenses
            .iter()
            .map(|license| license.declared_license_expression_spdx.clone()),
        relation,
    )?;

    Some(NormalizedDeclaredLicense::new(
        declared_expression,
        declared_spdx_expression,
    ))
}

pub(crate) fn build_declared_license_data(
    normalized: NormalizedDeclaredLicense,
    metadata: DeclaredLicenseMatchMetadata<'_>,
) -> (Option<String>, Option<String>, Vec<LicenseDetection>) {
    let detection = build_declared_license_detection(&normalized, metadata);

    (
        Some(normalized.declared_license_expression),
        Some(normalized.declared_license_expression_spdx),
        vec![detection],
    )
}

pub(crate) fn build_declared_license_data_from_pair(
    declared_license_expression: impl Into<String>,
    declared_license_expression_spdx: impl Into<String>,
    metadata: DeclaredLicenseMatchMetadata<'_>,
) -> (Option<String>, Option<String>, Vec<LicenseDetection>) {
    build_declared_license_data(
        NormalizedDeclaredLicense::new(
            declared_license_expression,
            declared_license_expression_spdx,
        ),
        metadata,
    )
}

pub(crate) fn build_declared_license_detection(
    normalized: &NormalizedDeclaredLicense,
    metadata: DeclaredLicenseMatchMetadata<'_>,
) -> LicenseDetection {
    LicenseDetection {
        license_expression: normalized.declared_license_expression.clone(),
        license_expression_spdx: normalized.declared_license_expression_spdx.clone(),
        matches: vec![Match {
            license_expression: normalized.declared_license_expression.clone(),
            license_expression_spdx: normalized.declared_license_expression_spdx.clone(),
            from_file: None,
            start_line: metadata.start_line,
            end_line: metadata.end_line,
            matcher: Some(PARSER_DECLARED_MATCHER.to_string()),
            score: 100.0,
            matched_length: Some(metadata.matched_text.split_whitespace().count()),
            match_coverage: Some(100.0),
            rule_relevance: Some(100),
            rule_identifier: None,
            rule_url: None,
            matched_text: Some(metadata.matched_text.to_string()),
        }],
        identifier: None,
    }
}

fn normalize_expression_ast(
    expression: &LicenseExpression,
    index: &LicenseIndex,
) -> Option<(LicenseExpression, LicenseExpression)> {
    match expression {
        LicenseExpression::License(key) => normalize_license_key(key, index).map(|normalized| {
            (
                LicenseExpression::License(normalized.declared_license_expression),
                LicenseExpression::License(normalized.declared_license_expression_spdx),
            )
        }),
        LicenseExpression::LicenseRef(key) => Some((
            LicenseExpression::LicenseRef(key.clone()),
            LicenseExpression::LicenseRef(key.clone()),
        )),
        LicenseExpression::And { left, right } => {
            let (left_declared, left_spdx) = normalize_expression_ast(left, index)?;
            let (right_declared, right_spdx) = normalize_expression_ast(right, index)?;

            Some((
                LicenseExpression::And {
                    left: Box::new(left_declared),
                    right: Box::new(right_declared),
                },
                LicenseExpression::And {
                    left: Box::new(left_spdx),
                    right: Box::new(right_spdx),
                },
            ))
        }
        LicenseExpression::Or { left, right } => {
            let (left_declared, left_spdx) = normalize_expression_ast(left, index)?;
            let (right_declared, right_spdx) = normalize_expression_ast(right, index)?;

            Some((
                LicenseExpression::Or {
                    left: Box::new(left_declared),
                    right: Box::new(right_declared),
                },
                LicenseExpression::Or {
                    left: Box::new(left_spdx),
                    right: Box::new(right_spdx),
                },
            ))
        }
        LicenseExpression::With { left, right } => {
            let (left_declared, left_spdx) = normalize_expression_ast(left, index)?;
            let (right_declared, right_spdx) = normalize_expression_ast(right, index)?;

            Some((
                LicenseExpression::With {
                    left: Box::new(left_declared),
                    right: Box::new(right_declared),
                },
                LicenseExpression::With {
                    left: Box::new(left_spdx),
                    right: Box::new(right_spdx),
                },
            ))
        }
    }
}

fn normalize_license_key(key: &str, index: &LicenseIndex) -> Option<NormalizedDeclaredLicense> {
    let normalized_key = key.trim();
    if normalized_key.is_empty() {
        return None;
    }

    if let Some(rid) = index
        .rid_by_spdx_key
        .get(&normalized_key.to_ascii_lowercase())
    {
        let rule_license_expression = index.rules_by_rid[*rid].license_expression.clone();
        if rule_license_expression.contains("unknown-spdx") {
            return None;
        }

        let canonical_spdx_key = index
            .licenses_by_key
            .get(&rule_license_expression)
            .and_then(|license| license.spdx_license_key.clone())
            .unwrap_or_else(|| normalized_key.to_string());

        let declared_license_expression =
            if normalized_key.eq_ignore_ascii_case(&canonical_spdx_key) {
                normalized_key.to_ascii_lowercase()
            } else {
                rule_license_expression
            };

        let declared_license_expression_spdx = index
            .licenses_by_key
            .get(&declared_license_expression)
            .and_then(|license| license.spdx_license_key.clone())
            .unwrap_or(canonical_spdx_key);

        return Some(NormalizedDeclaredLicense::new(
            declared_license_expression,
            declared_license_expression_spdx,
        ));
    }

    let normalized_scancode_key = normalized_key.to_ascii_lowercase();
    let license = index.licenses_by_key.get(&normalized_scancode_key)?;
    let declared_license_expression = license.key.clone();
    let declared_license_expression_spdx = license
        .spdx_license_key
        .clone()
        .unwrap_or_else(|| format!("LicenseRef-scancode-{}", declared_license_expression));

    Some(NormalizedDeclaredLicense::new(
        declared_license_expression,
        declared_license_expression_spdx,
    ))
}

#[derive(Clone, Copy)]
enum BooleanOperator {
    And,
    Or,
}

fn render_canonical_expression(expression: &LicenseExpression) -> String {
    match expression {
        LicenseExpression::License(key) => key.clone(),
        LicenseExpression::LicenseRef(key) => key.clone(),
        LicenseExpression::With { left, right } => format!(
            "{} WITH {}",
            render_canonical_expression(left),
            render_canonical_expression(right)
        ),
        LicenseExpression::And { .. } => {
            render_flat_boolean_chain(expression, BooleanOperator::And)
        }
        LicenseExpression::Or { .. } => render_flat_boolean_chain(expression, BooleanOperator::Or),
    }
}

fn render_flat_boolean_chain(expression: &LicenseExpression, operator: BooleanOperator) -> String {
    let mut parts = Vec::new();
    collect_boolean_chain(expression, operator, &mut parts);

    let separator = match operator {
        BooleanOperator::And => " AND ",
        BooleanOperator::Or => " OR ",
    };

    parts
        .into_iter()
        .map(|part| render_boolean_operand(part, operator))
        .collect::<Vec<_>>()
        .join(separator)
}

fn collect_boolean_chain<'a>(
    expression: &'a LicenseExpression,
    operator: BooleanOperator,
    parts: &mut Vec<&'a LicenseExpression>,
) {
    match (operator, expression) {
        (BooleanOperator::And, LicenseExpression::And { left, right })
        | (BooleanOperator::Or, LicenseExpression::Or { left, right }) => {
            collect_boolean_chain(left, operator, parts);
            collect_boolean_chain(right, operator, parts);
        }
        _ => parts.push(expression),
    }
}

fn render_boolean_operand(
    expression: &LicenseExpression,
    parent_operator: BooleanOperator,
) -> String {
    match expression {
        LicenseExpression::And { .. } => match parent_operator {
            BooleanOperator::And => render_canonical_expression(expression),
            BooleanOperator::Or => format!("({})", render_canonical_expression(expression)),
        },
        LicenseExpression::Or { .. } => match parent_operator {
            BooleanOperator::Or => render_canonical_expression(expression),
            BooleanOperator::And => format!("({})", render_canonical_expression(expression)),
        },
        _ => render_canonical_expression(expression),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_spdx_declared_license_identifier() {
        let (declared, declared_spdx, detections) = normalize_spdx_declared_license(Some("MIT"));

        assert_eq!(declared.as_deref(), Some("mit"));
        assert_eq!(declared_spdx.as_deref(), Some("MIT"));
        assert_eq!(detections.len(), 1);
        assert_eq!(
            detections[0].matches[0].matcher.as_deref(),
            Some(PARSER_DECLARED_MATCHER)
        );
    }

    #[test]
    fn test_normalize_spdx_declared_license_expression() {
        let (declared, declared_spdx, detections) =
            normalize_spdx_declared_license(Some("MIT OR Apache-2.0"));

        assert_eq!(declared.as_deref(), Some("mit OR apache-2.0"));
        assert_eq!(declared_spdx.as_deref(), Some("MIT OR Apache-2.0"));
        assert_eq!(detections.len(), 1);
    }

    #[test]
    fn test_normalize_spdx_declared_license_simplifies_absorbed_expression() {
        let (declared, declared_spdx, detections) =
            normalize_spdx_declared_license(Some("MIT AND (MIT OR Apache-2.0)"));

        assert_eq!(declared.as_deref(), Some("mit"));
        assert_eq!(declared_spdx.as_deref(), Some("MIT"));
        assert_eq!(detections.len(), 1);
    }

    #[test]
    fn test_normalize_declared_license_key_scancode() {
        let normalized = normalize_declared_license_key("mit").expect("normalized key");

        assert_eq!(normalized.declared_license_expression, "mit");
        assert_eq!(normalized.declared_license_expression_spdx, "MIT");
    }

    #[test]
    fn test_combine_normalized_licenses_with_or() {
        let combined = combine_normalized_licenses(
            vec![
                NormalizedDeclaredLicense::new("mit", "MIT"),
                NormalizedDeclaredLicense::new("apache-2.0", "Apache-2.0"),
            ],
            " OR ",
        )
        .expect("combined expression");

        assert_eq!(combined.declared_license_expression, "mit OR apache-2.0");
        assert_eq!(
            combined.declared_license_expression_spdx,
            "MIT OR Apache-2.0"
        );
    }

    #[test]
    fn test_combine_normalized_licenses_simplifies_absorbed_and_expression() {
        let combined = combine_normalized_licenses(
            vec![
                NormalizedDeclaredLicense::new("mit", "MIT"),
                NormalizedDeclaredLicense::new("mit OR apache-2.0", "MIT OR Apache-2.0"),
            ],
            " AND ",
        )
        .expect("combined expression");

        assert_eq!(combined.declared_license_expression, "mit");
        assert_eq!(combined.declared_license_expression_spdx, "MIT");
    }

    #[test]
    fn test_build_declared_license_detection_uses_parser_matcher() {
        let detection = build_declared_license_detection(
            &NormalizedDeclaredLicense::new("mit", "MIT"),
            DeclaredLicenseMatchMetadata::new("MIT", 4, 4),
        );

        assert_eq!(
            detection.matches[0].matcher.as_deref(),
            Some(PARSER_DECLARED_MATCHER)
        );
        assert_eq!(detection.matches[0].start_line, 4);
        assert_eq!(detection.matches[0].matched_text.as_deref(), Some("MIT"));
    }
}
