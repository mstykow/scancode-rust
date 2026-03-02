//! License expression parsing implementation.

use super::{LicenseExpression, ParseError};

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

/// Tokenize a license expression string into tokens.
pub(super) fn tokenize(expr: &str) -> Result<Vec<Token>, ParseError> {
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
                if c.is_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '+' {
                    let start = pos;
                    while pos < chars.len()
                        && (chars[pos].is_alphanumeric()
                            || chars[pos] == '-'
                            || chars[pos] == '.'
                            || chars[pos] == '_'
                            || chars[pos] == '+')
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
pub(super) fn parse_tokens(tokens: &[Token]) -> Result<LicenseExpression, ParseError> {
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
pub(super) fn parse_or(tokens: &[Token]) -> Result<(LicenseExpression, &[Token]), ParseError> {
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
pub(super) fn parse_and(tokens: &[Token]) -> Result<(LicenseExpression, &[Token]), ParseError> {
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
pub(super) fn parse_with(tokens: &[Token]) -> Result<(LicenseExpression, &[Token]), ParseError> {
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
pub(super) fn parse_primary(tokens: &[Token]) -> Result<(LicenseExpression, &[Token]), ParseError> {
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
    use super::super::{expression_to_string, LicenseExpression};
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
    fn test_parse_gpl_or_later_license() {
        let expr = parse_expression("gpl-2.0-plus").unwrap();
        assert_eq!(expr, LicenseExpression::License("gpl-2.0-plus".to_string()));
    }

    #[test]
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
    fn test_parse_deeply_nested_expression() {
        let input = "((MIT OR Apache-2.0) AND GPL-2.0) OR BSD-3-Clause";
        let expr = parse_expression(input).unwrap();
        assert!(matches!(expr, LicenseExpression::Or { .. }));
        let keys = expr.license_keys();
        assert_eq!(keys.len(), 4);
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
}
