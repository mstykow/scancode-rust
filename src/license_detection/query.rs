//! Query processing - tokenized input for license matching.

#![allow(dead_code)]

/// A query represents tokenized input text to be matched against license rules.
///
/// # TODO
/// This is a placeholder. Fields will be added based on Python reference:
/// - Token IDs
/// - Line numbers
/// - Token positions
/// - Original text
pub struct Query;

/// A query run is an actively running query with additional context.
///
/// # TODO
/// This is a placeholder. Fields will be added based on Python reference.
pub struct QueryRun;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_placeholder() {
        let _ = Query;
    }

    #[test]
    fn test_query_run_placeholder() {
        let _ = QueryRun;
    }
}
