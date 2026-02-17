//! License expression parsing and manipulation.
//!
//! This module provides a parser for ScanCode license expressions, supporting:
//! - ScanCode license keys (e.g., `mit`, `gpl-2.0-plus`, `apache-2.0`)
//! - SPDX operators: `AND`, `OR`, `WITH` (case-insensitive)
//! - Parenthetical grouping
//! - The `LicenseRef-scancode-*` format for non-SPDX licenses
//!
//! The parser converts license expression strings into an AST (Abstract Syntax Tree)
//! and provides functions for validation and simplification.

use std::collections::HashSet;

/// Error type for license expression parsing.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum ParseError {
    /// Empty expression
    EmptyExpression,

    /// Unexpected token at position
    UnexpectedToken { token: String, position: usize },

    /// Mismatched parentheses
    MismatchedParentheses,

    /// Invalid license key format
    InvalidLicenseKey { key: String },

    /// Invalid operator
    InvalidOperator { operator: String },

    /// Generic parse error with message
    ParseError(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyExpression => write!(f, "Empty license expression"),
            Self::UnexpectedToken { token, position } => {
                write!(f, "Unexpected token '{}' at position {}", token, position)
            }
            Self::MismatchedParentheses => write!(f, "Mismatched parentheses"),
            Self::InvalidLicenseKey { key } => write!(f, "Invalid license key: {}", key),
            Self::InvalidOperator { operator } => write!(f, "Invalid operator: {}", operator),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

/// Token in a license expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Token {
    /// License key
    License(String),

    /// Operator: AND
    And,

    /// Operator: OR
    Or,

    /// Operator: WITH
    With,

    /// Opening parenthesis
    LeftParen,

    /// Closing parenthesis
    RightParen,
}

/// Result of license expression validation.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Expression is valid
    Valid,

    /// Expression has unknown license keys
    UnknownKeys { unknown: Vec<String> },

    /// Expression has other validation errors
    Invalid { errors: Vec<String> },
}

/// A parsed license expression represented as an AST.
#[derive(Debug, Clone, PartialEq)]
pub enum LicenseExpression {
    /// A single license key
    License(String),

    /// A LicenseRef-scancode-* reference
    LicenseRef(String),

    /// AND operation: left AND right
    And {
        left: Box<LicenseExpression>,
        right: Box<LicenseExpression>,
    },

    /// OR operation: left OR right
    Or {
        left: Box<LicenseExpression>,
        right: Box<LicenseExpression>,
    },

    /// WITH operation: left WITH right (exception)
    With {
        left: Box<LicenseExpression>,
        right: Box<LicenseExpression>,
    },
}

impl LicenseExpression {
    /// Extract all license keys from the expression.
    #[allow(dead_code)]
    pub fn license_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        self.collect_keys(&mut keys);
        keys.sort();
        keys.dedup();
        keys
    }

    #[allow(dead_code)]
    fn collect_keys(&self, keys: &mut Vec<String>) {
        match self {
            Self::License(key) => keys.push(key.clone()),
            Self::LicenseRef(key) => keys.push(key.clone()),
            Self::And { left, right } | Self::Or { left, right } | Self::With { left, right } => {
                left.collect_keys(keys);
                right.collect_keys(keys);
            }
        }
    }

    /// Create an AND expression combining multiple expressions.
    pub fn and(expressions: Vec<LicenseExpression>) -> Option<LicenseExpression> {
        if expressions.is_empty() {
            None
        } else if expressions.len() == 1 {
            Some(expressions.into_iter().next().unwrap())
        } else {
            let mut iter = expressions.into_iter();
            let mut result = iter.next().unwrap();
            for expr in iter {
                result = LicenseExpression::And {
                    left: Box::new(result),
                    right: Box::new(expr),
                };
            }
            Some(result)
        }
    }

    /// Create an OR expression combining multiple expressions.
    pub fn or(expressions: Vec<LicenseExpression>) -> Option<LicenseExpression> {
        if expressions.is_empty() {
            None
        } else if expressions.len() == 1 {
            Some(expressions.into_iter().next().unwrap())
        } else {
            let mut iter = expressions.into_iter();
            let mut result = iter.next().unwrap();
            for expr in iter {
                result = LicenseExpression::Or {
                    left: Box::new(result),
                    right: Box::new(expr),
                };
            }
            Some(result)
        }
    }
}

/// Parse a license expression string into a structured expression.
///
/// # Arguments
/// * `expr` - The license expression string to parse
///
/// # Returns
/// Ok with parsed LicenseExpression, or Err with ParseError
///
/// # Examples
/// ```
/// use scancode_rust::license_detection::expression::parse_expression;
///
/// let expr = parse_expression("MIT AND Apache-2.0").unwrap();
/// ```
pub fn parse_expression(expr: &str) -> Result<LicenseExpression, ParseError> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Err(ParseError::EmptyExpression);
    }

    let tokens = tokenize(trimmed)?;
    parse_tokens(&tokens)
}

/// Simplify a license expression by deduplicating license keys.
///
/// # Arguments
/// * `expr` - The expression to simplify
///
/// # Returns
/// Simplified expression with duplicate licenses removed, preserving order.
pub fn simplify_expression(expr: &LicenseExpression) -> LicenseExpression {
    match expr {
        LicenseExpression::License(key) => LicenseExpression::License(key.clone()),
        LicenseExpression::LicenseRef(key) => LicenseExpression::LicenseRef(key.clone()),
        LicenseExpression::With { left, right } => LicenseExpression::With {
            left: Box::new(simplify_expression(left)),
            right: Box::new(simplify_expression(right)),
        },
        LicenseExpression::And { .. } => {
            let mut unique = Vec::new();
            let mut seen = HashSet::new();
            collect_unique_and(expr, &mut unique, &mut seen);
            build_expression_from_list(&unique, true)
        }
        LicenseExpression::Or { .. } => {
            let mut unique = Vec::new();
            let mut seen = HashSet::new();
            collect_unique_or(expr, &mut unique, &mut seen);
            build_expression_from_list(&unique, false)
        }
    }
}

fn collect_unique_and(
    expr: &LicenseExpression,
    unique: &mut Vec<LicenseExpression>,
    seen: &mut HashSet<String>,
) {
    match expr {
        LicenseExpression::And { left, right } => {
            collect_unique_and(left, unique, seen);
            collect_unique_and(right, unique, seen);
        }
        LicenseExpression::Or { .. } => {
            let simplified = simplify_expression(expr);
            let key = expression_to_string(&simplified);
            if !seen.contains(&key) {
                seen.insert(key);
                unique.push(simplified);
            }
        }
        LicenseExpression::With { left, right } => {
            let simplified = LicenseExpression::With {
                left: Box::new(simplify_expression(left)),
                right: Box::new(simplify_expression(right)),
            };
            let key = expression_to_string(&simplified);
            if !seen.contains(&key) {
                seen.insert(key);
                unique.push(simplified);
            }
        }
        LicenseExpression::License(key) => {
            if !seen.contains(key) {
                seen.insert(key.clone());
                unique.push(LicenseExpression::License(key.clone()));
            }
        }
        LicenseExpression::LicenseRef(key) => {
            if !seen.contains(key) {
                seen.insert(key.clone());
                unique.push(LicenseExpression::LicenseRef(key.clone()));
            }
        }
    }
}

fn collect_unique_or(
    expr: &LicenseExpression,
    unique: &mut Vec<LicenseExpression>,
    seen: &mut HashSet<String>,
) {
    match expr {
        LicenseExpression::Or { left, right } => {
            collect_unique_or(left, unique, seen);
            collect_unique_or(right, unique, seen);
        }
        LicenseExpression::And { .. } => {
            let simplified = simplify_expression(expr);
            let key = expression_to_string(&simplified);
            if !seen.contains(&key) {
                seen.insert(key);
                unique.push(simplified);
            }
        }
        LicenseExpression::With { left, right } => {
            let simplified = LicenseExpression::With {
                left: Box::new(simplify_expression(left)),
                right: Box::new(simplify_expression(right)),
            };
            let key = expression_to_string(&simplified);
            if !seen.contains(&key) {
                seen.insert(key);
                unique.push(simplified);
            }
        }
        LicenseExpression::License(key) => {
            if !seen.contains(key) {
                seen.insert(key.clone());
                unique.push(LicenseExpression::License(key.clone()));
            }
        }
        LicenseExpression::LicenseRef(key) => {
            if !seen.contains(key) {
                seen.insert(key.clone());
                unique.push(LicenseExpression::LicenseRef(key.clone()));
            }
        }
    }
}

fn build_expression_from_list(unique: &[LicenseExpression], is_and: bool) -> LicenseExpression {
    match unique.len() {
        0 => panic!("build_expression_from_list called with empty list"),
        1 => unique[0].clone(),
        _ => {
            let mut iter = unique.iter();
            let mut result = iter.next().unwrap().clone();
            for expr in iter {
                result = if is_and {
                    LicenseExpression::And {
                        left: Box::new(result),
                        right: Box::new(expr.clone()),
                    }
                } else {
                    LicenseExpression::Or {
                        left: Box::new(result),
                        right: Box::new(expr.clone()),
                    }
                };
            }
            result
        }
    }
}

/// Validate a license expression against known license keys.
///
/// # Arguments
/// * `expr` - The expression to validate
/// * `known_keys` - Set of known valid license keys
///
/// # Returns
/// ValidationResult indicating if expression is valid
#[allow(dead_code)]
pub fn validate_expression(
    expr: &LicenseExpression,
    known_keys: &HashSet<String>,
) -> ValidationResult {
    let mut unknown = Vec::new();

    for key in expr.license_keys() {
        if !known_keys.contains(&key) {
            unknown.push(key);
        }
    }

    if unknown.is_empty() {
        ValidationResult::Valid
    } else {
        ValidationResult::UnknownKeys { unknown }
    }
}

/// Convert a license expression to its string representation.
///
/// # Arguments
/// * `expr` - The expression to convert
///
/// # Returns
/// String representation of the expression
///
/// # Parentheses
/// Parentheses are added when needed to preserve semantic meaning based on
/// operator precedence (WITH > AND > OR). This matches the Python
/// license-expression library behavior.
pub fn expression_to_string(expr: &LicenseExpression) -> String {
    expression_to_string_internal(expr, None)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Or = 1,
    And = 2,
    With = 3,
}

fn expression_to_string_internal(
    expr: &LicenseExpression,
    parent_prec: Option<Precedence>,
) -> String {
    match expr {
        LicenseExpression::License(key) => key.clone(),
        LicenseExpression::LicenseRef(key) => key.clone(),
        LicenseExpression::And { left, right } => {
            let left_str = expression_to_string_internal(left, Some(Precedence::And));
            let right_str = expression_to_string_internal(right, Some(Precedence::And));
            let result = format!("{} AND {}", left_str, right_str);
            if parent_prec.is_some_and(|p| p != Precedence::And) {
                format!("({})", result)
            } else {
                result
            }
        }
        LicenseExpression::Or { left, right } => {
            let left_str = expression_to_string_internal(left, Some(Precedence::Or));
            let right_str = expression_to_string_internal(right, Some(Precedence::Or));
            let result = format!("{} OR {}", left_str, right_str);
            if parent_prec.is_some_and(|p| p != Precedence::Or) {
                format!("({})", result)
            } else {
                result
            }
        }
        LicenseExpression::With { left, right } => {
            let left_str = expression_to_string_internal(left, Some(Precedence::With));
            let right_str = expression_to_string_internal(right, Some(Precedence::With));
            format!("{} WITH {}", left_str, right_str)
        }
    }
}

/// Relation for combining license expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombineRelation {
    /// Combine with AND operation
    And,
    /// Combine with OR operation
    #[allow(dead_code)]
    Or,
}

/// Combine multiple license expressions into a single expression.
///
/// This function parses each expression string, combines them using the specified
/// relation, and optionally deduplicates license keys.
///
/// # Arguments
/// * `expressions` - Slice of expression strings to combine
/// * `relation` - How to combine (AND or OR)
/// * `unique` - If true, deduplicate license keys
///
/// # Returns
/// Ok with combined expression string, or Err with parse error
///
/// # Examples
/// ```
/// use scancode_rust::license_detection::expression::{combine_expressions, CombineRelation};
///
/// let combined = combine_expressions(
///     &["mit", "gpl-2.0-plus"],
///     CombineRelation::And,
///     true
/// ).unwrap();
/// assert_eq!(combined, "mit AND gpl-2.0-plus");
/// ```
pub fn combine_expressions(
    expressions: &[&str],
    relation: CombineRelation,
    unique: bool,
) -> Result<String, ParseError> {
    if expressions.is_empty() {
        return Ok(String::new());
    }
    if expressions.len() == 1 {
        let parsed = parse_expression(expressions[0])?;
        return Ok(expression_to_string(&if unique {
            simplify_expression(&parsed)
        } else {
            parsed
        }));
    }

    let parsed_exprs: Vec<LicenseExpression> = expressions
        .iter()
        .map(|e| parse_expression(e))
        .collect::<Result<Vec<_>, _>>()?;

    let combined = match relation {
        CombineRelation::And => LicenseExpression::and(parsed_exprs),
        CombineRelation::Or => LicenseExpression::or(parsed_exprs),
    };

    match combined {
        Some(expr) => {
            let final_expr = if unique {
                simplify_expression(&expr)
            } else {
                expr
            };
            Ok(expression_to_string(&final_expr))
        }
        None => Ok(String::new()),
    }
}

/// Tokenize a license expression string into tokens.
fn tokenize(expr: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    let chars: Vec<char> = expr.chars().collect();

    while pos < chars.len() {
        let c = chars[pos];

        if c.is_whitespace() {
            pos += 1;
            continue;
        }

        match c {
            '(' => {
                tokens.push(Token::LeftParen);
                pos += 1;
            }
            ')' => {
                tokens.push(Token::RightParen);
                pos += 1;
            }
            _ => {
                if c.is_alphanumeric() || c == '-' || c == '.' || c == '_' {
                    let start = pos;
                    while pos < chars.len()
                        && (chars[pos].is_alphanumeric()
                            || chars[pos] == '-'
                            || chars[pos] == '.'
                            || chars[pos] == '_')
                    {
                        pos += 1;
                    }
                    let text: String = chars[start..pos].iter().collect();
                    let token = match_text_to_token(&text);
                    tokens.push(token);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        token: c.to_string(),
                        position: pos,
                    });
                }
            }
        }
    }

    Ok(tokens)
}

/// Match text to appropriate token.
fn match_text_to_token(text: &str) -> Token {
    let text_upper = text.to_uppercase();
    match text_upper.as_str() {
        "AND" => Token::And,
        "OR" => Token::Or,
        "WITH" => Token::With,
        _ => Token::License(text.to_lowercase()),
    }
}

/// Parse tokens into a LicenseExpression using recursive descent.
fn parse_tokens(tokens: &[Token]) -> Result<LicenseExpression, ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::EmptyExpression);
    }

    let (expr, remaining) = parse_or(tokens)?;
    if !remaining.is_empty() {
        return Err(ParseError::ParseError(format!(
            "Unexpected tokens after parsing: {:?}",
            remaining
        )));
    }

    Ok(expr)
}

/// Parse OR expressions (lowest precedence).
fn parse_or(tokens: &[Token]) -> Result<(LicenseExpression, &[Token]), ParseError> {
    let (mut expr, mut remaining) = parse_and(tokens)?;

    while let Some(Token::Or) = remaining.first() {
        remaining = &remaining[1..];
        let (right, rest) = parse_and(remaining)?;
        expr = LicenseExpression::Or {
            left: Box::new(expr),
            right: Box::new(right),
        };
        remaining = rest;
    }

    Ok((expr, remaining))
}

/// Parse AND expressions (medium precedence).
fn parse_and(tokens: &[Token]) -> Result<(LicenseExpression, &[Token]), ParseError> {
    let (mut expr, mut remaining) = parse_with(tokens)?;

    while let Some(Token::And) = remaining.first() {
        remaining = &remaining[1..];
        let (right, rest) = parse_with(remaining)?;
        expr = LicenseExpression::And {
            left: Box::new(expr),
            right: Box::new(right),
        };
        remaining = rest;
    }

    Ok((expr, remaining))
}

/// Parse WITH expressions (highest precedence for operators).
fn parse_with(tokens: &[Token]) -> Result<(LicenseExpression, &[Token]), ParseError> {
    let (mut expr, mut remaining) = parse_primary(tokens)?;

    while let Some(Token::With) = remaining.first() {
        remaining = &remaining[1..];
        let (right, rest) = parse_primary(remaining)?;
        expr = LicenseExpression::With {
            left: Box::new(expr),
            right: Box::new(right),
        };
        remaining = rest;
    }

    Ok((expr, remaining))
}

/// Parse primary expressions (license keys or parenthesized expressions).
fn parse_primary(tokens: &[Token]) -> Result<(LicenseExpression, &[Token]), ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::EmptyExpression);
    }

    match &tokens[0] {
        Token::LeftParen => {
            if tokens.len() < 2 {
                return Err(ParseError::MismatchedParentheses);
            }
            let (expr, remaining) = parse_or(&tokens[1..])?;
            if remaining.is_empty() || remaining[0] != Token::RightParen {
                return Err(ParseError::MismatchedParentheses);
            }
            Ok((expr, &remaining[1..]))
        }
        Token::License(key) => {
            let expr = if key.starts_with("licenseref-") {
                LicenseExpression::LicenseRef(key.clone())
            } else {
                LicenseExpression::License(key.clone())
            };
            Ok((expr, &tokens[1..]))
        }
        Token::RightParen => Err(ParseError::MismatchedParentheses),
        Token::And | Token::Or | Token::With => Err(ParseError::ParseError(format!(
            "Unexpected operator at start: {:?}",
            tokens[0]
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_license() {
        let expr = parse_expression("MIT").unwrap();
        assert_eq!(expr, LicenseExpression::License("mit".to_string()));
    }

    #[test]
    fn test_parse_simple_lowercase() {
        let expr = parse_expression("mit").unwrap();
        assert_eq!(expr, LicenseExpression::License("mit".to_string()));
    }

    #[test]
    fn test_parse_simple_mixed_case() {
        let expr = parse_expression("MiT").unwrap();
        assert_eq!(expr, LicenseExpression::License("mit".to_string()));
    }

    #[test]
    fn test_parse_and_expression() {
        let expr = parse_expression("MIT AND Apache-2.0").unwrap();
        assert!(matches!(expr, LicenseExpression::And { .. }));
        assert_eq!(expression_to_string(&expr), "mit AND apache-2.0");
    }

    #[test]
    fn test_parse_or_expression() {
        let expr = parse_expression("MIT OR Apache-2.0").unwrap();
        assert!(matches!(expr, LicenseExpression::Or { .. }));
        assert_eq!(expression_to_string(&expr), "mit OR apache-2.0");
    }

    #[test]
    fn test_parse_with_expression() {
        let expr = parse_expression("GPL-2.0 WITH Classpath-exception-2.0").unwrap();
        assert!(matches!(expr, LicenseExpression::With { .. }));
        assert_eq!(
            expression_to_string(&expr),
            "gpl-2.0 WITH classpath-exception-2.0"
        );
    }

    #[test]
    fn test_parse_parenthesized_expression() {
        let expr = parse_expression("(MIT OR Apache-2.0)").unwrap();
        assert!(matches!(expr, LicenseExpression::Or { .. }));
    }

    #[test]
    fn test_parse_complex_expression() {
        let expr =
            parse_expression("(GPL-2.0 WITH Classpath-exception-2.0) AND Apache-2.0").unwrap();
        assert!(matches!(expr, LicenseExpression::And { .. }));
    }

    #[test]
    fn test_parse_nested_parens() {
        let expr = parse_expression("((MIT OR Apache-2.0) AND GPL-2.0)").unwrap();
        assert!(matches!(expr, LicenseExpression::And { .. }));
    }

    #[test]
    fn test_parse_scancode_plus_license() {
        let expr = parse_expression("gpl-2.0-plus").unwrap();
        assert_eq!(expr, LicenseExpression::License("gpl-2.0-plus".to_string()));
    }

    #[test]
    fn test_parse_licenseref() {
        let expr = parse_expression("LicenseRef-scancode-custom-1").unwrap();
        assert_eq!(
            expr,
            LicenseExpression::LicenseRef("licenseref-scancode-custom-1".to_string())
        );
    }

    #[test]
    fn test_parse_various_whitespace() {
        let expr1 = parse_expression("MIT AND Apache-2.0").unwrap();
        let expr2 = parse_expression("MIT   AND   Apache-2.0").unwrap();
        assert_eq!(expr1, expr2);
    }

    #[test]
    fn test_parse_trailing_whitespace() {
        let expr = parse_expression("MIT   ").unwrap();
        assert_eq!(expr, LicenseExpression::License("mit".to_string()));
    }

    #[test]
    fn test_parse_leading_whitespace() {
        let expr = parse_expression("   MIT").unwrap();
        assert_eq!(expr, LicenseExpression::License("mit".to_string()));
    }

    #[test]
    fn test_parse_empty_expression() {
        let result = parse_expression("");
        assert!(matches!(result, Err(ParseError::EmptyExpression)));
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = parse_expression("   ");
        assert!(matches!(result, Err(ParseError::EmptyExpression)));
    }

    #[test]
    fn test_parse_mismatched_open_paren() {
        let result = parse_expression("(MIT AND Apache-2.0");
        assert!(matches!(result, Err(ParseError::MismatchedParentheses)));
    }

    #[test]
    fn test_parse_mismatched_close_paren() {
        let result = parse_expression("MIT AND Apache-2.0)");
        assert!(matches!(result, Err(ParseError::ParseError(_))));
    }

    #[test]
    fn test_parse_unexpected_character() {
        let result = parse_expression("MIT @ Apache-2.0");
        assert!(matches!(result, Err(ParseError::UnexpectedToken { .. })));
    }

    #[test]
    fn test_parse_multiple_licenses_or() {
        let expr = parse_expression("MIT OR Apache-2.0 OR GPL-2.0").unwrap();
        assert!(matches!(expr, LicenseExpression::Or { .. }));
    }

    #[test]
    fn test_parse_multiple_licenses_and() {
        let expr = parse_expression("MIT AND Apache-2.0 AND GPL-2.0").unwrap();
        assert!(matches!(expr, LicenseExpression::And { .. }));
    }

    #[test]
    fn test_contractor_precedence_and_or() {
        let expr = parse_expression("MIT OR Apache-2.0 AND GPL-2.0").unwrap();
        assert!(matches!(expr, LicenseExpression::Or { .. }));
    }

    #[test]
    fn test_license_keys_simple() {
        let expr = parse_expression("MIT").unwrap();
        let keys = expr.license_keys();
        assert_eq!(keys, vec!["mit"]);
    }

    #[test]
    fn test_license_keys_multiple() {
        let expr = parse_expression("MIT OR Apache-2.0 AND GPL-2.0").unwrap();
        let keys = expr.license_keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"mit".to_string()));
        assert!(keys.contains(&"apache-2.0".to_string()));
        assert!(keys.contains(&"gpl-2.0".to_string()));
    }

    #[test]
    fn test_license_keys_deduplication() {
        let expr = parse_expression("MIT AND MIT OR Apache-2.0").unwrap();
        let keys = expr.license_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"mit".to_string()));
        assert!(keys.contains(&"apache-2.0".to_string()));
    }

    #[test]
    fn test_expression_to_string_simple() {
        let expr = LicenseExpression::License("mit".to_string());
        assert_eq!(expression_to_string(&expr), "mit");
    }

    #[test]
    fn test_expression_to_string_and() {
        let expr = LicenseExpression::And {
            left: Box::new(LicenseExpression::License("mit".to_string())),
            right: Box::new(LicenseExpression::License("apache-2.0".to_string())),
        };
        assert_eq!(expression_to_string(&expr), "mit AND apache-2.0");
    }

    #[test]
    fn test_expression_to_string_or() {
        let expr = LicenseExpression::Or {
            left: Box::new(LicenseExpression::License("mit".to_string())),
            right: Box::new(LicenseExpression::License("apache-2.0".to_string())),
        };
        assert_eq!(expression_to_string(&expr), "mit OR apache-2.0");
    }

    #[test]
    fn test_expression_to_string_with() {
        let expr = LicenseExpression::With {
            left: Box::new(LicenseExpression::License("gpl-2.0".to_string())),
            right: Box::new(LicenseExpression::License(
                "classpath-exception-2.0".to_string(),
            )),
        };
        assert_eq!(
            expression_to_string(&expr),
            "gpl-2.0 WITH classpath-exception-2.0"
        );
    }

    #[test]
    fn test_expression_to_string_licenseref() {
        let expr = LicenseExpression::LicenseRef("licenseref-scancode-custom".to_string());
        assert_eq!(expression_to_string(&expr), "licenseref-scancode-custom");
    }

    #[test]
    fn test_validate_expression_valid() {
        let expr = parse_expression("MIT AND Apache-2.0").unwrap();
        let mut known = HashSet::new();
        known.insert("mit".to_string());
        known.insert("apache-2.0".to_string());

        let result = validate_expression(&expr, &known);
        assert_eq!(result, ValidationResult::Valid);
    }

    #[test]
    fn test_validate_expression_unknown_keys() {
        let expr = parse_expression("MIT AND UnknownKey").unwrap();
        let mut known = HashSet::new();
        known.insert("mit".to_string());

        let result = validate_expression(&expr, &known);
        assert!(matches!(result, ValidationResult::UnknownKeys { .. }));
        if let ValidationResult::UnknownKeys { unknown } = result {
            assert_eq!(unknown.len(), 1);
            assert_eq!(unknown[0], "unknownkey");
        }
    }

    #[test]
    fn test_simplify_expression_no_change() {
        let expr = parse_expression("MIT AND Apache-2.0").unwrap();
        let simplified = simplify_expression(&expr);
        assert_eq!(expression_to_string(&simplified), "mit AND apache-2.0");
    }

    #[test]
    fn test_simplify_expression_with_duplicates() {
        let expr = parse_expression("MIT OR MIT").unwrap();
        let simplified = simplify_expression(&expr);
        assert_eq!(expression_to_string(&simplified), "mit");
    }

    #[test]
    fn test_simplify_and_duplicates() {
        let expr = parse_expression("crapl-0.1 AND crapl-0.1").unwrap();
        let simplified = simplify_expression(&expr);
        assert_eq!(expression_to_string(&simplified), "crapl-0.1");
    }

    #[test]
    fn test_simplify_or_duplicates() {
        let expr = parse_expression("mit OR mit").unwrap();
        let simplified = simplify_expression(&expr);
        assert_eq!(expression_to_string(&simplified), "mit");
    }

    #[test]
    fn test_simplify_preserves_different_licenses() {
        let expr = parse_expression("mit AND apache-2.0").unwrap();
        let simplified = simplify_expression(&expr);
        assert_eq!(expression_to_string(&simplified), "mit AND apache-2.0");
    }

    #[test]
    fn test_simplify_complex_duplicates() {
        let expr = parse_expression("gpl-2.0-plus AND gpl-2.0-plus AND lgpl-2.0-plus").unwrap();
        let simplified = simplify_expression(&expr);
        assert_eq!(
            expression_to_string(&simplified),
            "gpl-2.0-plus AND lgpl-2.0-plus"
        );
    }

    #[test]
    fn test_simplify_three_duplicates() {
        let expr = parse_expression("fsf-free AND fsf-free AND fsf-free").unwrap();
        let simplified = simplify_expression(&expr);
        assert_eq!(expression_to_string(&simplified), "fsf-free");
    }

    #[test]
    fn test_simplify_with_expression_dedup() {
        let expr = parse_expression(
            "gpl-2.0 WITH classpath-exception-2.0 AND gpl-2.0 WITH classpath-exception-2.0",
        )
        .unwrap();
        let simplified = simplify_expression(&expr);
        assert_eq!(
            expression_to_string(&simplified),
            "gpl-2.0 WITH classpath-exception-2.0"
        );
    }

    #[test]
    fn test_simplify_nested_duplicates() {
        let expr = parse_expression("(mit AND apache-2.0) OR (mit AND apache-2.0)").unwrap();
        let simplified = simplify_expression(&expr);
        assert_eq!(expression_to_string(&simplified), "mit AND apache-2.0");
    }

    #[test]
    fn test_simplify_preserves_order() {
        let expr = parse_expression("apache-2.0 AND mit AND apache-2.0").unwrap();
        let simplified = simplify_expression(&expr);
        assert_eq!(expression_to_string(&simplified), "apache-2.0 AND mit");
    }

    #[test]
    fn test_simplify_mit_and_mit_and_apache() {
        let expr = parse_expression("mit AND mit AND apache-2.0").unwrap();
        let simplified = simplify_expression(&expr);
        assert_eq!(expression_to_string(&simplified), "mit AND apache-2.0");
    }

    #[test]
    fn test_and_helper_empty() {
        let result = LicenseExpression::and(vec![]);
        assert!(result.is_none());
    }

    #[test]
    fn test_and_helper_single() {
        let expr = LicenseExpression::License("mit".to_string());
        let result = LicenseExpression::and(vec![expr.clone()]).unwrap();
        assert_eq!(result, expr);
    }

    #[test]
    fn test_and_helper_multiple() {
        let exprs = vec![
            LicenseExpression::License("mit".to_string()),
            LicenseExpression::License("apache-2.0".to_string()),
        ];
        let result = LicenseExpression::and(exprs).unwrap();
        assert!(matches!(result, LicenseExpression::And { .. }));
    }

    #[test]
    fn test_or_helper_empty() {
        let result = LicenseExpression::or(vec![]);
        assert!(result.is_none());
    }

    #[test]
    fn test_or_helper_single() {
        let expr = LicenseExpression::License("mit".to_string());
        let result = LicenseExpression::or(vec![expr.clone()]).unwrap();
        assert_eq!(result, expr);
    }

    #[test]
    fn test_or_helper_multiple() {
        let exprs = vec![
            LicenseExpression::License("mit".to_string()),
            LicenseExpression::License("apache-2.0".to_string()),
        ];
        let result = LicenseExpression::or(exprs).unwrap();
        assert!(matches!(result, LicenseExpression::Or { .. }));
    }

    #[test]
    fn test_combine_expressions_empty() {
        let result = combine_expressions(&[], CombineRelation::And, true).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_combine_expressions_single() {
        let result = combine_expressions(&["mit"], CombineRelation::And, true).unwrap();
        assert_eq!(result, "mit");
    }

    #[test]
    fn test_combine_expressions_two_and() {
        let result =
            combine_expressions(&["mit", "gpl-2.0-plus"], CombineRelation::And, true).unwrap();
        assert_eq!(result, "mit AND gpl-2.0-plus");
    }

    #[test]
    fn test_combine_expressions_two_or() {
        let result =
            combine_expressions(&["mit", "apache-2.0"], CombineRelation::Or, true).unwrap();
        assert_eq!(result, "mit OR apache-2.0");
    }

    #[test]
    fn test_combine_expressions_multiple_and() {
        let result = combine_expressions(
            &["mit", "apache-2.0", "gpl-2.0-plus"],
            CombineRelation::And,
            true,
        )
        .unwrap();
        assert!(result.contains("mit"));
        assert!(result.contains("apache-2.0"));
        assert!(result.contains("gpl-2.0-plus"));
        assert_eq!(result.matches("AND").count(), 2);
    }

    #[test]
    fn test_combine_expressions_with_duplicates_unique() {
        let result =
            combine_expressions(&["mit", "mit", "apache-2.0"], CombineRelation::Or, true).unwrap();
        let expr = parse_expression(&result).unwrap();
        let keys = expr.license_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"mit".to_string()));
        assert!(keys.contains(&"apache-2.0".to_string()));
    }

    #[test]
    fn test_combine_expressions_with_duplicates_not_unique() {
        let result =
            combine_expressions(&["mit", "mit", "apache-2.0"], CombineRelation::Or, false).unwrap();
        let expr = parse_expression(&result).unwrap();
        assert_eq!(result, "mit OR mit OR apache-2.0");
        let keys = expr.license_keys();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_combine_expressions_complex_with_simplification() {
        let result = combine_expressions(
            &["mit OR apache-2.0", "gpl-2.0-plus"],
            CombineRelation::And,
            true,
        )
        .unwrap();
        assert_eq!(result, "(mit OR apache-2.0) AND gpl-2.0-plus");
        let expr = parse_expression(&result).unwrap();
        assert!(matches!(expr, LicenseExpression::And { .. }));
        let keys = expr.license_keys();
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn test_combine_expressions_parse_error() {
        let result = combine_expressions(&["mit", "@invalid@"], CombineRelation::And, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_expression_to_string_or_inside_and() {
        let or_expr = LicenseExpression::Or {
            left: Box::new(LicenseExpression::License("mit".to_string())),
            right: Box::new(LicenseExpression::License("apache-2.0".to_string())),
        };
        let and_expr = LicenseExpression::And {
            left: Box::new(or_expr),
            right: Box::new(LicenseExpression::License("gpl-2.0".to_string())),
        };
        assert_eq!(
            expression_to_string(&and_expr),
            "(mit OR apache-2.0) AND gpl-2.0"
        );
    }

    #[test]
    fn test_expression_to_string_and_inside_or() {
        let and_expr = LicenseExpression::And {
            left: Box::new(LicenseExpression::License("mit".to_string())),
            right: Box::new(LicenseExpression::License("apache-2.0".to_string())),
        };
        let or_expr = LicenseExpression::Or {
            left: Box::new(and_expr),
            right: Box::new(LicenseExpression::License("gpl-2.0".to_string())),
        };
        assert_eq!(
            expression_to_string(&or_expr),
            "(mit AND apache-2.0) OR gpl-2.0"
        );
    }

    #[test]
    fn test_expression_to_string_with_inside_or() {
        let with_expr = LicenseExpression::With {
            left: Box::new(LicenseExpression::License("gpl-2.0".to_string())),
            right: Box::new(LicenseExpression::License(
                "classpath-exception-2.0".to_string(),
            )),
        };
        let or_expr = LicenseExpression::Or {
            left: Box::new(with_expr),
            right: Box::new(LicenseExpression::License("mit".to_string())),
        };
        assert_eq!(
            expression_to_string(&or_expr),
            "gpl-2.0 WITH classpath-exception-2.0 OR mit"
        );
    }

    #[test]
    fn test_expression_to_string_with_inside_and() {
        let with_expr = LicenseExpression::With {
            left: Box::new(LicenseExpression::License("gpl-2.0".to_string())),
            right: Box::new(LicenseExpression::License(
                "classpath-exception-2.0".to_string(),
            )),
        };
        let and_expr = LicenseExpression::And {
            left: Box::new(with_expr),
            right: Box::new(LicenseExpression::License("mit".to_string())),
        };
        assert_eq!(
            expression_to_string(&and_expr),
            "gpl-2.0 WITH classpath-exception-2.0 AND mit"
        );
    }

    #[test]
    fn test_expression_to_string_nested_or_no_parens() {
        let or_expr = LicenseExpression::Or {
            left: Box::new(LicenseExpression::Or {
                left: Box::new(LicenseExpression::License("mit".to_string())),
                right: Box::new(LicenseExpression::License("apache-2.0".to_string())),
            }),
            right: Box::new(LicenseExpression::License("gpl-2.0".to_string())),
        };
        assert_eq!(
            expression_to_string(&or_expr),
            "mit OR apache-2.0 OR gpl-2.0"
        );
    }

    #[test]
    fn test_expression_to_string_nested_and_no_parens() {
        let and_expr = LicenseExpression::And {
            left: Box::new(LicenseExpression::And {
                left: Box::new(LicenseExpression::License("mit".to_string())),
                right: Box::new(LicenseExpression::License("apache-2.0".to_string())),
            }),
            right: Box::new(LicenseExpression::License("gpl-2.0".to_string())),
        };
        assert_eq!(
            expression_to_string(&and_expr),
            "mit AND apache-2.0 AND gpl-2.0"
        );
    }

    #[test]
    fn test_expression_to_string_roundtrip_or_and() {
        let input = "(mit OR apache-2.0) AND gpl-2.0";
        let expr = parse_expression(input).unwrap();
        let output = expression_to_string(&expr);
        assert_eq!(output, "(mit OR apache-2.0) AND gpl-2.0");
    }

    #[test]
    fn test_expression_to_string_roundtrip_or_with() {
        let input = "gpl-2.0 WITH classpath-exception-2.0 OR mit";
        let expr = parse_expression(input).unwrap();
        let output = expression_to_string(&expr);
        assert_eq!(output, "gpl-2.0 WITH classpath-exception-2.0 OR mit");
    }

    #[test]
    fn test_parse_gpl_or_later_license() {
        let expr = parse_expression("gpl-2.0-plus").unwrap();
        assert_eq!(expr, LicenseExpression::License("gpl-2.0-plus".to_string()));
    }
    #[test]
    #[ignore]
    fn test_parse_gpl_plus_license() {
        let expr = parse_expression("GPL-2.0+").unwrap();
        assert_eq!(expr, LicenseExpression::License("gpl-2.0+".to_string()));
    }

    #[test]
    fn test_parse_complex_nested_expression() {
        let input = "(MIT OR Apache-2.0) AND (GPL-2.0 OR BSD-3-Clause)";
        let expr = parse_expression(input).unwrap();
        assert!(matches!(expr, LicenseExpression::And { .. }));
        let keys = expr.license_keys();
        assert_eq!(keys.len(), 4);
    }

    #[test]
    fn test_parse_multiple_with_expressions() {
        let expr = parse_expression(
            "GPL-2.0 WITH Classpath-exception-2.0 AND GPL-2.0 WITH GCC-exception-2.0",
        )
        .unwrap();
        assert!(matches!(expr, LicenseExpression::And { .. }));
        let keys = expr.license_keys();
        assert!(keys.contains(&"gpl-2.0".to_string()));
        assert!(keys.contains(&"classpath-exception-2.0".to_string()));
        assert!(keys.contains(&"gcc-exception-2.0".to_string()));
    }

    #[test]
    fn test_parse_with_inside_and_inside_or() {
        let expr = parse_expression("MIT OR (Apache-2.0 AND GPL-2.0 WITH Classpath-exception-2.0)")
            .unwrap();
        assert!(matches!(expr, LicenseExpression::Or { .. }));
    }

    #[test]
    fn test_parse_operator_at_start_error() {
        let result = parse_expression("AND MIT");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_operator_at_end_error() {
        let result = parse_expression("MIT AND");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_double_operator_error() {
        let result = parse_expression("MIT AND AND Apache-2.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_license_with_dots() {
        let expr = parse_expression("LicenseRef-scancode-1.0").unwrap();
        assert_eq!(
            expr,
            LicenseExpression::LicenseRef("licenseref-scancode-1.0".to_string())
        );
    }

    #[test]
    fn test_combine_expressions_with_existing_and() {
        let result = combine_expressions(
            &["mit AND apache-2.0", "gpl-2.0"],
            CombineRelation::And,
            true,
        )
        .unwrap();
        assert!(result.contains("mit"));
        assert!(result.contains("apache-2.0"));
        assert!(result.contains("gpl-2.0"));
    }

    #[test]
    fn test_combine_expressions_with_existing_or() {
        let result =
            combine_expressions(&["mit OR apache-2.0", "gpl-2.0"], CombineRelation::Or, true)
                .unwrap();
        assert!(result.contains("mit"));
        assert!(result.contains("apache-2.0"));
        assert!(result.contains("gpl-2.0"));
    }

    #[test]
    fn test_expression_to_string_and_inside_with() {
        let and_expr = LicenseExpression::And {
            left: Box::new(LicenseExpression::License("mit".to_string())),
            right: Box::new(LicenseExpression::License("apache-2.0".to_string())),
        };
        let with_expr = LicenseExpression::With {
            left: Box::new(and_expr),
            right: Box::new(LicenseExpression::License("exception".to_string())),
        };
        assert_eq!(
            expression_to_string(&with_expr),
            "(mit AND apache-2.0) WITH exception"
        );
    }

    #[test]
    fn test_parse_deeply_nested_expression() {
        let input = "((MIT OR Apache-2.0) AND GPL-2.0) OR BSD-3-Clause";
        let expr = parse_expression(input).unwrap();
        assert!(matches!(expr, LicenseExpression::Or { .. }));
        let keys = expr.license_keys();
        assert_eq!(keys.len(), 4);
    }

    #[test]
    fn test_validate_expression_empty_known_keys() {
        let expr = parse_expression("MIT AND Apache-2.0").unwrap();
        let known = HashSet::new();

        let result = validate_expression(&expr, &known);
        assert!(matches!(result, ValidationResult::UnknownKeys { .. }));
        if let ValidationResult::UnknownKeys { unknown } = result {
            assert_eq!(unknown.len(), 2);
        }
    }

    #[test]
    fn test_parse_case_insensitive_operators() {
        let expr1 = parse_expression("MIT and Apache-2.0").unwrap();
        let expr2 = parse_expression("MIT AND Apache-2.0").unwrap();
        let expr3 = parse_expression("MIT And Apache-2.0").unwrap();
        assert_eq!(expression_to_string(&expr1), "mit AND apache-2.0");
        assert_eq!(expression_to_string(&expr2), "mit AND apache-2.0");
        assert_eq!(expression_to_string(&expr3), "mit AND apache-2.0");
    }

    #[test]
    fn test_parse_or_case_insensitive() {
        let expr1 = parse_expression("MIT or Apache-2.0").unwrap();
        let expr2 = parse_expression("MIT OR Apache-2.0").unwrap();
        assert_eq!(expression_to_string(&expr1), "mit OR apache-2.0");
        assert_eq!(expression_to_string(&expr2), "mit OR apache-2.0");
    }

    #[test]
    fn test_parse_with_case_insensitive() {
        let expr1 = parse_expression("GPL-2.0 with Classpath-exception-2.0").unwrap();
        let expr2 = parse_expression("GPL-2.0 WITH Classpath-exception-2.0").unwrap();
        assert_eq!(
            expression_to_string(&expr1),
            "gpl-2.0 WITH classpath-exception-2.0"
        );
        assert_eq!(
            expression_to_string(&expr2),
            "gpl-2.0 WITH classpath-exception-2.0"
        );
    }

    #[test]
    fn test_expression_to_string_or_inside_with() {
        let or_expr = LicenseExpression::Or {
            left: Box::new(LicenseExpression::License("mit".to_string())),
            right: Box::new(LicenseExpression::License("apache-2.0".to_string())),
        };
        let with_expr = LicenseExpression::With {
            left: Box::new(or_expr),
            right: Box::new(LicenseExpression::License("exception".to_string())),
        };
        assert_eq!(
            expression_to_string(&with_expr),
            "(mit OR apache-2.0) WITH exception"
        );
    }

    #[test]
    fn test_expression_to_string_with_no_outer_parens() {
        let with_expr = LicenseExpression::With {
            left: Box::new(LicenseExpression::License("gpl-2.0-plus".to_string())),
            right: Box::new(LicenseExpression::License(
                "classpath-exception-2.0".to_string(),
            )),
        };
        assert_eq!(
            expression_to_string(&with_expr),
            "gpl-2.0-plus WITH classpath-exception-2.0"
        );
    }

    #[test]
    fn test_expression_to_string_with_as_right_operand_of_or() {
        let with_expr = LicenseExpression::With {
            left: Box::new(LicenseExpression::License("gpl-2.0".to_string())),
            right: Box::new(LicenseExpression::License(
                "classpath-exception-2.0".to_string(),
            )),
        };
        let or_expr = LicenseExpression::Or {
            left: Box::new(LicenseExpression::License("mit".to_string())),
            right: Box::new(with_expr),
        };
        assert_eq!(
            expression_to_string(&or_expr),
            "mit OR gpl-2.0 WITH classpath-exception-2.0"
        );
    }

    #[test]
    fn test_expression_to_string_with_as_right_operand_of_and() {
        let with_expr = LicenseExpression::With {
            left: Box::new(LicenseExpression::License("gpl-2.0".to_string())),
            right: Box::new(LicenseExpression::License(
                "classpath-exception-2.0".to_string(),
            )),
        };
        let and_expr = LicenseExpression::And {
            left: Box::new(LicenseExpression::License("mit".to_string())),
            right: Box::new(with_expr),
        };
        assert_eq!(
            expression_to_string(&and_expr),
            "mit AND gpl-2.0 WITH classpath-exception-2.0"
        );
    }

    #[test]
    fn test_expression_to_string_complex_precedence() {
        let input = "mit OR apache-2.0 AND gpl-2.0";
        let expr = parse_expression(input).unwrap();
        assert_eq!(
            expression_to_string(&expr),
            "mit OR (apache-2.0 AND gpl-2.0)"
        );
    }
}
