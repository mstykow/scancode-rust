//! Text tokenization and normalization.
//!
//! Tokenization converts text into a sequence of tokens that can be matched
//! against license rules. This module implements ScanCode-compatible tokenization.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

/// Common words that are ignored from matching such as HTML tags, XML entities, etc.
///
/// This is the Rust equivalent of the Python STOPWORDS frozenset from
/// reference/scancode-toolkit/src/licensedcode/stopwords.py
#[allow(dead_code)]
static STOPWORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();

    // common XML character references as &quot;
    for &word in &["amp", "apos", "gt", "lt", "nbsp", "quot"] {
        set.insert(word);
    }

    // common html tags as <a href=https://link ...> dfsdfsdf</a>
    for &word in &[
        "a",
        "abbr",
        "alt",
        "blockquote",
        "body",
        "br",
        "class",
        "div",
        "em",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "hr",
        "href",
        "img",
        "li",
        "ol",
        "p",
        "pre",
        "rel",
        "script",
        "span",
        "src",
        "td",
        "th",
        "tr",
        "ul",
    ] {
        set.insert(word);
    }

    // comment line markers
    set.insert("rem"); // batch files
    set.insert("dnl"); // autotools

    // doc book tags as <para>
    set.insert("para");
    set.insert("ulink");

    // Some HTML punctuations and entities all as &emdash;
    for &word in &[
        "bdquo", "bull", "bullet", "colon", "comma", "emdash", "emsp", "ensp", "ge", "hairsp",
        "ldquo", "ldquor", "le", "lpar", "lsaquo", "lsquo", "lsquor", "mdash", "ndash", "numsp",
        "period", "puncsp", "raquo", "rdquo", "rdquor", "rpar", "rsaquo", "rsquo", "rsquor",
        "sbquo", "semi", "thinsp", "tilde",
    ] {
        set.insert(word);
    }

    // some xml char entities
    set.insert("x3c");
    set.insert("x3e");

    // seen in many CSS
    for &word in &[
        "lists", "side", "nav", "height", "auto", "border", "padding", "width",
    ] {
        set.insert(word);
    }

    // seen in Perl PODs
    set.insert("head1");
    set.insert("head2");
    set.insert("head3");

    // common in C literals
    set.insert("printf");

    // common in shell
    set.insert("echo");

    set
});

/// Splits on whitespace and punctuation: keep only characters and numbers and + when in the middle or end of a word.
///
/// The pattern is equivalent to Python's: `[^_\W]+\+?[^_\W]*`
/// - `[^_\W]+` - one or more characters that are NOT underscore and NOT non-word (i.e., alphanumeric)
/// - `\+?` - optional plus sign (important for license names like "GPL2+")
/// - `[^_\W]*` - zero or more alphanumeric characters
///
/// This matches word-like sequences while preserving trailing `+` characters.
#[allow(dead_code)]
static QUERY_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[A-Za-z0-9]+\+?[A-Za-z0-9]*").expect("Invalid regex pattern"));

/// Tokenizes text to match index rules and queries.
///
/// Splits text into tokens using regex pattern, normalizes each token (lowercase),
/// and filters out empty strings and stopwords.
///
/// # Returns
/// A vector of token strings.
///
/// # Examples
/// ```
/// # use scancode_rust::license_detection::tokenize::tokenize;
/// let tokens = tokenize("Hello World!");
/// assert_eq!(tokens, vec!["hello", "world"]);
/// ```
#[allow(dead_code)]
pub fn tokenize(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut tokens = Vec::new();
    let lowercase_text = text.to_lowercase();

    for cap in QUERY_PATTERN.find_iter(&lowercase_text) {
        let token = cap.as_str();

        // Filter out empty strings and stopwords
        if !token.is_empty() && !STOPWORDS.contains(token) {
            tokens.push(token.to_string());
        }
    }

    tokens
}

/// Tokenizes text without filtering stopwords.
///
/// This is used for query text where stopwords are handled at a later stage.
///
/// # Returns
/// A vector of token strings.
///
/// # Examples
/// ```
/// # use scancode_rust::license_detection::tokenize::tokenize_without_stopwords;
/// let tokens = tokenize_without_stopwords("Hello World div");
/// assert_eq!(tokens, vec!["hello", "world", "div"]);
/// ```
#[allow(dead_code)]
pub fn tokenize_without_stopwords(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut tokens = Vec::new();
    let lowercase_text = text.to_lowercase();

    for cap in QUERY_PATTERN.find_iter(&lowercase_text) {
        let token = cap.as_str();

        // Filter out empty strings but keep stopwords
        if !token.is_empty() {
            tokens.push(token.to_string());
        }
    }

    tokens
}

/// Normalizes text before tokenization.
///
/// Currently a passthrough as the Python implementation doesn't do
/// special normalization beyond lowercasing in the tokenizer.
///
/// # Arguments
/// * `text` - The input text
///
/// # Returns
/// Normalized text
#[allow(dead_code)]
pub fn normalize_text(text: &str) -> String {
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_empty() {
        let result = tokenize("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_tokenize_simple() {
        let result = tokenize("Hello World");
        assert_eq!(result, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_with_punctuation() {
        let result = tokenize("Hello, World! This is a test.");
        // Note: 'a' is filtered because it's in STOPWORDS (it's an HTML tag)
        assert_eq!(result, vec!["hello", "world", "this", "is", "test"]);
    }

    #[test]
    fn test_tokenize_with_spaces() {
        let result = tokenize("some Text with   spAces!");
        assert_eq!(result, vec!["some", "text", "with", "spaces"]);
    }

    #[test]
    fn test_tokenize_with_plus() {
        let result = tokenize("GPL2+ and GPL3");
        assert_eq!(result, vec!["gpl2+", "and", "gpl3"]);
    }

    #[test]
    fn test_tokenize_filters_stopwords() {
        let result = tokenize("Hello div World p");
        assert_eq!(result, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_with_special_chars() {
        let result = tokenize("special+-_!@ chars");
        // Based on Python: ['special+', 'chars']
        assert_eq!(result, vec!["special+", "chars"]);
    }

    #[test]
    fn test_tokenize_with_underscores() {
        let result = tokenize("hello_world foo_bar");
        assert_eq!(result, vec!["hello", "world", "foo", "bar"]);
    }

    #[test]
    fn test_tokenize_with_numbers() {
        let result = tokenize("version 2.0 and 3.0");
        assert_eq!(result, vec!["version", "2", "0", "and", "3", "0"]);
    }

    #[test]
    fn test_tokenize_without_stopwords_keeps_html_tags() {
        let result = tokenize_without_stopwords("Hello div World p");
        assert_eq!(result, vec!["hello", "div", "world", "p"]);
    }

    #[test]
    fn test_tokenize_without_stopwords_empty() {
        let result = tokenize_without_stopwords("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_tokenization_with_plus_in_middle() {
        let result = tokenize("C++ and GPL+");
        assert_eq!(result, vec!["c+", "and", "gpl+"]);
    }

    #[test]
    fn test_tokenization_braces() {
        let result = tokenize("{{Hi}}some {{}}Text with{{noth+-_!@ing}}   {{junk}}spAces!");
        assert_eq!(
            result,
            vec![
                "hi", "some", "text", "with", "noth+", "ing", "junk", "spaces"
            ]
        );
    }

    #[test]
    fn test_normalize_text_passthrough() {
        let result = normalize_text("Hello   World");
        assert_eq!(result, "Hello   World");
    }

    #[test]
    fn test_tokenize_with_ampersand() {
        let result = tokenize("some &quot< markup &gt\"");
        assert_eq!(result, vec!["some", "markup"]);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_query_tokenizer_brace_case() {
        let result = tokenize("{{}some }}Text with   spAces! + _ -");
        assert_eq!(result, vec!["some", "text", "with", "spaces"]);
    }
}
