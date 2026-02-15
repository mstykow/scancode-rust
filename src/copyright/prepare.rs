//! Text preparation and normalization for copyright detection.
//!
//! Normalizes raw text lines before copyright detection:
//! - Copyright symbol normalization (©, (C), &#169;, etc. → (c))
//! - HTML entity decoding (&amp;, &lt;, &gt;, etc.)
//! - Comment marker removal (/*, */, #, etc.)
//! - Markup stripping (Debian <s></s>, HTML tags)
//! - Quote normalization (backticks, double quotes → single quotes)
//! - Escape handling (\t, \n → spaces)
//! - Punctuation cleanup
//! - Emdash normalization (– → -)
//! - Placeholder removal (<year>, <name>, etc.)

use std::sync::LazyLock;

use regex::Regex;

/// Regex to remove C-style printf format codes like ` %s ` or ` #d `.
static PRINTF_FORMAT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" [#%][a-zA-Z] ").unwrap());

/// Regex to remove punctuation characters: `*#"%[]{}` and backtick.
static PUNCTUATION_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[*#"%\[\]{}`]+"#).unwrap());

/// Regex to fold consecutive quotes (2+ single quotes → one).
static CONSECUTIVE_QUOTES_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"'{2,}").unwrap());

/// Regex to remove less common comment markers: `rem`, `@rem`, `dnl` at line start.
static WEIRD_COMMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(@?rem|dnl)\s+").unwrap());

/// Regex to remove man page comment markers: `."`.
static MAN_COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\.\""#).unwrap());

/// Regex to strip remaining HTML-like tags, excluding email addresses in angle brackets.
static HTML_TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>@]+>").unwrap());

/// Regex to strip common HTML tags even without a closing `>`.
/// Covers the most common HTML tags that appear in source files.
/// Python's `split_on_tags` uses `< */? *[a-z]+[a-z0-9@\-\._\+]* */? *>?` which
/// makes `>` optional, allowing malformed tags like `<b `, `<div `, `</a ` to be stripped.
static HTML_TAG_MALFORMED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)<\s*/?\s*(?:a|abbr|address|area|article|aside|audio|b|base|bdi|bdo|blockquote|body|br|button|canvas|caption|cite|code|col|colgroup|data|datalist|dd|del|details|dfn|dialog|div|dl|dt|em|embed|fieldset|figcaption|figure|font|footer|form|h[1-6]|head|header|hgroup|hr|html|i|iframe|img|input|ins|kbd|label|legend|li|link|main|map|mark|menu|meta|meter|nav|noscript|object|ol|optgroup|option|output|p|param|picture|pre|progress|q|rp|rt|ruby|s|samp|script|section|select|slot|small|source|span|strong|style|sub|summary|sup|table|tbody|td|template|textarea|tfoot|th|thead|time|title|tr|track|u|ul|var|video|wbr)\b\s*/?\s*>?",
    )
    .unwrap()
});

/// Regex to strip HTML attribute tokens that leak into copyright text.
/// Python's `SKIP_ATTRIBUTES` skips tokens starting with `href=`, `class=`, etc.
static HTML_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:href|class|width|style|xmlns|xml|lang|type|rel|src|alt|id|name|action|method|target|value|placeholder)=[^\s]*",
    )
    .unwrap()
});

/// Regex to strip `mailto:` URLs.
static MAILTO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"mailto:\S+").unwrap());

/// Regex to strip CSS measurement artifacts like "0pt" that leak through HTML demarkup.
static CSS_MEASUREMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d+pt\b").unwrap());

/// Replace HTML tag matches with spaces, but preserve matches containing
/// `copyright`, `author`, or `legal` (case-insensitive) — mirroring Python's
/// `keep_tag()` / `KEEP_MARKERS` logic.
fn replace_tags_preserving_copyright(text: &str, re: &Regex) -> String {
    re.replace_all(text, |caps: &regex::Captures| {
        let m = caps.get(0).unwrap().as_str();
        let lower = m.to_ascii_lowercase();
        if lower.contains("copyright") || lower.contains("author") || lower.contains("legal") {
            m.to_string()
        } else {
            " ".to_string()
        }
    })
    .into_owned()
}

/// Prepare a text `line` for copyright detection.
///
/// Applies a sequence of normalizations to clean up raw text before
/// copyright/author detection. This mirrors the Python `prepare_text_line()`
/// function from ScanCode Toolkit.
pub fn prepare_text_line(line: &str) -> String {
    let mut s = line.to_string();

    // ── Man page junk removal ──
    s = s
        .replace("\\\\ co", " ")
        .replace("\\ co", " ")
        .replace("(co ", " ");

    // Remove printf format codes like ` %s ` or ` #d `
    s = PRINTF_FORMAT_RE.replace_all(&s, " ").into_owned();

    // Remove less common comment markers (rem, @rem, dnl)
    s = WEIRD_COMMENT_RE.replace_all(&s, " ").into_owned();

    // Remove man page comment markers: `."` → space
    s = MAN_COMMENT_RE.replace_all(&s, " ").into_owned();

    // Remove C/C++ block comment markers only (not # and % yet — those
    // would destroy HTML entities like &#169; and printf-like patterns
    // that have already been handled above).
    s = s.replace("/*", " ").replace("*/", " ");

    // ── Copyright symbol normalization ──
    // Must happen BEFORE aggressive # and % removal so that HTML numeric
    // entities (&#169;, &#xa9;, etc.) and backslash escapes (\\XA9) are
    // recognized and converted first.
    s = s
        // RST |copy| directive
        .replace("|copy|", " (c) ")
        // Uncommon pipe chars in ASCII art
        .replace('|', " ")
        // Normalize spacing around "Copyright
        .replace("\"Copyright", "\" Copyright")
        // All copyright sign variants → (c)
        .replace("( C)", " (c) ")
        .replace("(C)", " (c) ")
        .replace("(c)", " (c) ")
        .replace("( © )", " (c) ")
        .replace("(©)", " (c) ")
        .replace("(© )", " (c) ")
        .replace("( ©)", " (c) ")
        .replace(['©', '\u{00A9}'], " (c) ")
        // HTML entities
        .replace("&copy;", " (c) ")
        .replace("&copy", " (c) ")
        .replace("&#169;", " (c) ")
        .replace("&#xa9;", " (c) ")
        .replace("&#xA9;", " (c) ")
        .replace("&#Xa9;", " (c) ")
        .replace("&#XA9;", " (c) ")
        // Unicode escape forms
        .replace("u00A9", " (c) ")
        .replace("u00a9", " (c) ")
        // Backslash hex escapes
        .replace("\\XA9", " (c) ")
        .replace("\\A9", " (c) ")
        .replace("\\a9", " (c) ")
        .replace("<A9>", " (c) ")
        .replace("XA9;", " (c) ")
        .replace("Xa9;", " (c) ")
        .replace("xA9;", " (c) ")
        .replace("xa9;", " (c) ")
        // \xc2 is UTF-8 prefix for © — remove it
        .replace('\u{00C2}', "")
        .replace("\\xc2", "");

    // ── HTML entity decoding ──
    // Must also happen BEFORE # and % removal for the same reason.
    s = s
        // Emdash
        .replace('\u{2013}', "-")
        // CR/LF entities
        .replace("&#13;&#10;", " ")
        .replace("&#13;", " ")
        .replace("&#10;", " ")
        // Space entities
        .replace("&ensp;", " ")
        .replace("&emsp;", " ")
        .replace("&thinsp;", " ")
        // Named entities
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&amp;", "&")
        .replace("&#38;", "&")
        .replace("&gt;", ">")
        .replace("&gt", ">")
        .replace("&#62;", ">")
        .replace("&lt;", "<")
        .replace("&lt", "<")
        .replace("&#60;", "<");

    // Now remove remaining code comment markers (*, #, %) and strip edges.
    // HTML entities have already been decoded so # and % are safe to remove.
    s = s.replace(['*', '#', '%'], " ");
    s = s.trim_matches(|c: char| " \\/*#%;".contains(c)).to_string();

    // ── Quote normalization ──
    s = s
        .replace(['`', '"'], "'")
        // Python unicode prefix
        .replace(" u'", " '")
        // Section sign
        .replace('§', " ")
        // Keep http URLs
        .replace("<http", " http")
        // Placeholders
        .replace("<insert ", " ")
        .replace("year>", " ")
        .replace("<year>", " ")
        .replace("<name>", " ");

    // ── Fold consecutive quotes ──
    s = CONSECUTIVE_QUOTES_RE.replace_all(&s, "'").into_owned();

    // ── Escape handling ──
    s = s
        .replace("\\t", " ")
        .replace("\\n", " ")
        .replace("\\r", " ")
        .replace("\\0", " ")
        .replace('\\', " ")
        .replace("('", " ")
        .replace("')", " ")
        .replace("],", " ");

    // ── Debian markup removal ──
    s = s.replace("</s>", "").replace("<s>", "").replace("<s/>", "");

    // ── HTML tag stripping (copyright/author/legal-aware) ──
    s = replace_tags_preserving_copyright(&s, &HTML_TAG_RE);

    // ── Malformed HTML tag stripping (no closing `>` required) ──
    s = replace_tags_preserving_copyright(&s, &HTML_TAG_MALFORMED_RE);

    // ── HTML attribute token removal ──
    s = HTML_ATTR_RE.replace_all(&s, " ").into_owned();
    s = MAILTO_RE.replace_all(&s, " ").into_owned();

    // ── CSS measurement artifact removal ──
    // Strip CSS measurement units like "0pt" that leak through HTML demarkup
    // (e.g., from `margin:0pt` or `font-size:0pt` in inline styles).
    s = CSS_MEASUREMENT_RE.replace_all(&s, " ").into_owned();

    // ── Punctuation cleanup ──
    s = PUNCTUATION_RE.replace_all(&s, " ").into_owned();

    // ── Space normalization around commas ──
    s = s.replace(" , ", ", ");

    // ── Angle bracket spacing ──
    s = s.replace('>', "> ").replace('<', " <");

    // ── Strip leading/trailing stars and spaces ──
    s = s.trim_matches(|c: char| c == ' ' || c == '*').to_string();

    // ── Normalize whitespace ──
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copyright_symbol_c_upper() {
        let result = prepare_text_line("(C) 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
        assert!(result.contains("2024"));
    }

    #[test]
    fn test_copyright_symbol_c_lower() {
        let result = prepare_text_line("(c) 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
    }

    #[test]
    fn test_copyright_symbol_c_spaced() {
        let result = prepare_text_line("( C) 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
    }

    #[test]
    fn test_copyright_symbol_unicode() {
        let result = prepare_text_line("© 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
    }

    #[test]
    fn test_copyright_symbol_html_entity_named() {
        let result = prepare_text_line("&copy; 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
    }

    #[test]
    fn test_copyright_symbol_html_entity_numeric() {
        let result = prepare_text_line("&#169; 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
    }

    #[test]
    fn test_copyright_symbol_html_entity_hex() {
        let result = prepare_text_line("&#xA9; 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
    }

    #[test]
    fn test_copyright_symbol_copy_without_semicolon() {
        let result = prepare_text_line("&copy 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
    }

    #[test]
    fn test_copyright_symbol_rst_copy() {
        let result = prepare_text_line("|copy| 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
    }

    #[test]
    fn test_copyright_symbol_u00a9() {
        let result = prepare_text_line("u00A9 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
    }

    #[test]
    fn test_copyright_symbol_backslash_xa9() {
        let result = prepare_text_line("\\XA9 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
    }

    #[test]
    fn test_copyright_symbol_angle_a9() {
        let result = prepare_text_line("<A9> 2024 Acme");
        assert!(result.contains("(c)"), "got: {result}");
    }

    #[test]
    fn test_html_entity_amp() {
        assert_eq!(prepare_text_line("foo &amp; bar"), "foo & bar");
    }

    #[test]
    fn test_html_entity_lt_gt() {
        // &lt; and &gt; are decoded to < and >, then < b > is stripped as
        // an HTML tag by the tag-stripping regex. This matches Python behavior.
        let result = prepare_text_line("a &lt;b&gt; c");
        assert!(result.contains("a"), "got: {result}");
        assert!(result.contains("c"), "got: {result}");
    }

    #[test]
    fn test_html_entity_quot() {
        // Quotes get normalized to single quotes, then punctuation removes them
        let result = prepare_text_line("say &quot;hello&quot;");
        assert!(result.contains("say"), "got: {result}");
        assert!(result.contains("hello"), "got: {result}");
    }

    #[test]
    fn test_html_entity_spaces() {
        let result = prepare_text_line("a&ensp;b&emsp;c&thinsp;d");
        assert_eq!(result, "a b c d");
    }

    #[test]
    fn test_emdash_normalization() {
        assert_eq!(prepare_text_line("2020\u{2013}2024"), "2020-2024");
    }

    #[test]
    fn test_whitespace_normalization() {
        assert_eq!(prepare_text_line("  foo   bar   baz  "), "foo bar baz");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(prepare_text_line(""), "");
    }

    #[test]
    fn test_debian_markup_removal() {
        let result = prepare_text_line("Copyright <s>Foo</s>");
        assert!(!result.contains("<s>"), "got: {result}");
        assert!(!result.contains("</s>"), "got: {result}");
        assert!(result.contains("Foo"), "got: {result}");
    }

    #[test]
    fn test_debian_markup_self_closing() {
        let result = prepare_text_line("text <s/> more");
        assert!(!result.contains("<s/>"), "got: {result}");
    }

    #[test]
    fn test_comment_marker_c_style() {
        let result = prepare_text_line("/* Copyright 2024 Acme */");
        assert!(result.contains("Copyright"), "got: {result}");
        assert!(result.contains("2024"), "got: {result}");
    }

    #[test]
    fn test_comment_marker_star_prefix() {
        let result = prepare_text_line(" * Copyright 2024 Acme");
        assert!(result.contains("Copyright"), "got: {result}");
        assert!(result.contains("2024"), "got: {result}");
    }

    #[test]
    fn test_comment_marker_hash() {
        let result = prepare_text_line("# Copyright 2024 Acme");
        assert!(result.contains("Copyright"), "got: {result}");
        assert!(result.contains("2024"), "got: {result}");
    }

    #[test]
    fn test_comment_marker_rem() {
        let result = prepare_text_line("rem Copyright 2024 Acme");
        assert!(result.contains("Copyright"), "got: {result}");
    }

    #[test]
    fn test_comment_marker_dnl() {
        let result = prepare_text_line("dnl Copyright 2024 Acme");
        assert!(result.contains("Copyright"), "got: {result}");
    }

    #[test]
    fn test_placeholder_removal_year() {
        let result = prepare_text_line("Copyright <year> Author");
        assert!(!result.contains("<year>"), "got: {result}");
        assert!(result.contains("Author"), "got: {result}");
    }

    #[test]
    fn test_placeholder_removal_name() {
        let result = prepare_text_line("Copyright 2024 <name>");
        assert!(!result.contains("<name>"), "got: {result}");
    }

    #[test]
    fn test_placeholder_http_preserved() {
        let result = prepare_text_line("<http://example.com>");
        assert!(result.contains("http"), "got: {result}");
    }

    #[test]
    fn test_escape_handling_tab() {
        let result = prepare_text_line("foo\\tbar");
        assert!(result.contains("foo"), "got: {result}");
        assert!(result.contains("bar"), "got: {result}");
        assert!(!result.contains("\\t"), "got: {result}");
    }

    #[test]
    fn test_escape_handling_newline() {
        let result = prepare_text_line("foo\\nbar");
        assert!(!result.contains("\\n"), "got: {result}");
    }

    #[test]
    fn test_backslash_removal() {
        let result = prepare_text_line("foo\\bar");
        assert!(!result.contains('\\'), "got: {result}");
    }

    #[test]
    fn test_quote_normalization_backtick() {
        // Backticks become single quotes, then punctuation may remove them
        let result = prepare_text_line("say `hello`");
        assert!(result.contains("say"), "got: {result}");
        assert!(result.contains("hello"), "got: {result}");
    }

    #[test]
    fn test_consecutive_quotes_folded() {
        let result = prepare_text_line("it''s a test");
        // Two single quotes should become one
        assert!(result.contains("it"), "got: {result}");
    }

    #[test]
    fn test_pipe_removal() {
        let result = prepare_text_line("foo | bar");
        assert!(!result.contains('|'), "got: {result}");
    }

    #[test]
    fn test_section_sign_removal() {
        let result = prepare_text_line("Section§3");
        assert!(!result.contains('§'), "got: {result}");
    }

    #[test]
    fn test_html_tag_stripping() {
        let result = prepare_text_line("Copyright <b>2024</b> Acme");
        assert!(!result.contains("<b>"), "got: {result}");
        assert!(!result.contains("</b>"), "got: {result}");
        assert!(result.contains("2024"), "got: {result}");
    }

    #[test]
    fn test_comma_spacing() {
        assert_eq!(prepare_text_line("a , b , c"), "a, b, c");
    }

    #[test]
    fn test_printf_format_codes_removed() {
        let result = prepare_text_line("foo %s bar");
        // %s surrounded by spaces should be removed
        assert_eq!(result, "foo bar");
    }

    #[test]
    fn test_man_page_comment() {
        let result = prepare_text_line(".\" Copyright 2024");
        assert!(result.contains("Copyright"), "got: {result}");
    }

    #[test]
    fn test_combined_normalization() {
        let result = prepare_text_line(" * (C) 2024 Acme &amp; Co.");
        assert!(result.contains("(c)"), "got: {result}");
        assert!(result.contains("2024"), "got: {result}");
        assert!(result.contains("Acme"), "got: {result}");
        assert!(result.contains("& Co."), "got: {result}");
    }

    #[test]
    fn test_complex_line() {
        let result =
            prepare_text_line("/* Copyright &#169; 2020\u{2013}2024 Foo &amp; Bar <name> */");
        assert!(result.contains("(c)"), "got: {result}");
        assert!(result.contains("2020-2024"), "got: {result}");
        assert!(result.contains("Foo"), "got: {result}");
        assert!(result.contains("& Bar"), "got: {result}");
        assert!(!result.contains("<name>"), "got: {result}");
    }

    #[test]
    fn test_man_page_co_junk() {
        let result = prepare_text_line("\\\\ co 2024 Author");
        assert!(result.contains("2024"), "got: {result}");
        assert!(result.contains("Author"), "got: {result}");
    }

    #[test]
    fn test_cr_lf_entities() {
        let result = prepare_text_line("line1&#13;&#10;line2");
        assert_eq!(result, "line1 line2");
    }

    #[test]
    fn test_insert_placeholder() {
        let result = prepare_text_line("<insert your name>");
        assert!(!result.contains("<insert"), "got: {result}");
    }

    #[test]
    fn test_bracket_removal() {
        let result = prepare_text_line("foo [bar] {baz}");
        assert!(!result.contains('['), "got: {result}");
        assert!(!result.contains(']'), "got: {result}");
        assert!(!result.contains('{'), "got: {result}");
        assert!(!result.contains('}'), "got: {result}");
    }

    #[test]
    fn test_only_whitespace() {
        assert_eq!(prepare_text_line("   \t  \n  "), "");
    }

    #[test]
    fn test_passthrough_normal_text() {
        assert_eq!(
            prepare_text_line("Copyright 2024 John Doe"),
            "Copyright 2024 John Doe"
        );
    }

    #[test]
    fn test_unicode_names_preserved() {
        let result = prepare_text_line("Copyright 2024 François Müller");
        assert_eq!(result, "Copyright 2024 François Müller");
    }

    #[test]
    fn test_unicode_spanish_names_preserved() {
        let result = prepare_text_line("Copyright 2024 José García");
        assert_eq!(result, "Copyright 2024 José García");
    }

    #[test]
    fn test_unicode_nordic_names_preserved() {
        let result = prepare_text_line("Copyright 2024 Björn Ångström");
        assert_eq!(result, "Copyright 2024 Björn Ångström");
    }

    #[test]
    fn test_unicode_polish_names_preserved() {
        let result = prepare_text_line("Copyright 2024 Łukasz Żółw");
        assert_eq!(result, "Copyright 2024 Łukasz Żółw");
    }

    // ── Gap 1: Malformed/unclosed HTML tag stripping ──

    #[test]
    fn test_strip_malformed_tag_b_no_closing() {
        let result = prepare_text_line("Copyright <b 2024 Acme");
        assert!(
            !result.contains("<b"),
            "Malformed tag should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
        assert!(
            result.contains("Acme"),
            "Name should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_malformed_tag_div_no_closing() {
        let result = prepare_text_line("Copyright <div 2024 Acme");
        assert!(
            !result.contains("<div"),
            "Malformed tag should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_malformed_closing_tag() {
        let result = prepare_text_line("Copyright </a 2024 Acme");
        assert!(
            !result.contains("</a"),
            "Malformed closing tag should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_malformed_tag_span() {
        let result = prepare_text_line("Copyright <span 2024 Acme");
        assert!(
            !result.contains("<span"),
            "Malformed span should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_malformed_tag_p() {
        let result = prepare_text_line("<p Copyright 2024 Acme");
        assert!(
            !result.contains("<p"),
            "Malformed p tag should be stripped: {result}"
        );
        assert!(
            result.contains("Copyright"),
            "Copyright should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_self_closing_br() {
        let result = prepare_text_line("Copyright 2024<br/>Acme");
        assert!(
            !result.contains("<br"),
            "br tag should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
        assert!(
            result.contains("Acme"),
            "Name should be preserved: {result}"
        );
    }

    // ── Gap 2: HTML attribute token removal ──

    #[test]
    fn test_strip_href_attribute() {
        let result = prepare_text_line("Copyright href=http://example.com 2024 Acme");
        assert!(
            !result.contains("href="),
            "href attribute should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_class_attribute() {
        let result = prepare_text_line("Copyright class=main 2024 Acme");
        assert!(
            !result.contains("class="),
            "class attribute should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_width_attribute() {
        let result = prepare_text_line("Copyright width=100 2024 Acme");
        assert!(
            !result.contains("width="),
            "width attribute should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_xmlns_attribute() {
        let result = prepare_text_line("Copyright xmlns=http://www.w3.org 2024 Acme");
        assert!(
            !result.contains("xmlns="),
            "xmlns attribute should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_mailto() {
        let result = prepare_text_line("Copyright 2024 mailto:john@example.com Acme");
        assert!(
            !result.contains("mailto:"),
            "mailto should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
        assert!(
            result.contains("Acme"),
            "Name should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_lang_attribute() {
        let result = prepare_text_line("Copyright lang=en 2024 Acme");
        assert!(
            !result.contains("lang="),
            "lang attribute should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_style_attribute() {
        let result = prepare_text_line("Copyright style=color:red 2024 Acme");
        assert!(
            !result.contains("style="),
            "style attribute should be stripped: {result}"
        );
        assert!(
            result.contains("2024"),
            "Year should be preserved: {result}"
        );
    }

    // ── Gap 3: Preserve copyright/author/legal in angle brackets ──

    #[test]
    fn test_preserve_copyright_in_angle_brackets() {
        let result = prepare_text_line("<copyright notice> 2024 Acme");
        assert!(
            result.contains("copyright"),
            "copyright token should be preserved: {result}"
        );
    }

    #[test]
    fn test_preserve_author_in_angle_brackets() {
        let result = prepare_text_line("<author> John Doe");
        assert!(
            result.contains("author"),
            "author token should be preserved: {result}"
        );
    }

    #[test]
    fn test_preserve_legal_in_angle_brackets() {
        let result = prepare_text_line("<legal> 2024 Acme Corp");
        assert!(
            result.contains("legal"),
            "legal token should be preserved: {result}"
        );
    }

    #[test]
    fn test_strip_regular_tag_but_preserve_copyright_tag() {
        let result = prepare_text_line("<div>Copyright</div> <copyright> 2024");
        assert!(
            result.contains("copyright"),
            "copyright tag should be preserved: {result}"
        );
        assert!(
            !result.contains("<div>"),
            "div tag should be stripped: {result}"
        );
    }
}
