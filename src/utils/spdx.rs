use std::collections::HashSet;

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
