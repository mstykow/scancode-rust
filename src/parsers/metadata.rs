// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

/// Registered detection-surface metadata for auto-generating documentation.
///
/// This module provides the `ParserMetadata` type used by parser `metadata()`
/// trait methods and by `bin/generate_supported_formats.rs` to automatically
/// generate `docs/SUPPORTED_FORMATS.md`.
///
/// Fields are used by the xtask but not in library code,
/// so we allow dead_code warnings for library builds.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ParserMetadata {
    /// Human-readable description (e.g., "npm package.json manifest")
    pub description: &'static str,
    /// File patterns this parser matches (e.g., `["**/package.json"]`).
    ///
    /// Documentation of the intended surface, not an executable contract, and
    /// deliberately not asserted against `is_match`. The two cannot be held
    /// equal: many parsers gate on content or path context rather than the
    /// filename alone — an `.apk` is claimed only if its magic bytes match, a
    /// `METADATA` only inside a wheel's dist-info — so a pattern describes what
    /// a user should expect to be recognised, while `is_match` decides whether a
    /// specific file on disk actually is.
    ///
    /// Two consequences worth knowing when editing these:
    ///
    /// - A pattern here can be *broader* than `is_match`, and legitimately so.
    ///   Keep it recognisable to a user reading `docs/SUPPORTED_FORMATS.md`
    ///   rather than mechanically exact.
    /// - A surface that is not expressible as a glob at all — detector-driven,
    ///   scanner-gated, or resolved relative to the scan root — uses the
    ///   `<...>` convention instead (e.g.
    ///   `"<compiled Go binaries with Go build info>"`), which the generated
    ///   table renders as prose. Prefer that over a glob that would advertise
    ///   files the parser declines.
    pub file_patterns: &'static [&'static str],
    /// Package type identifier (e.g., "npm", "pypi", "maven")
    pub package_type: &'static str,
    /// Primary programming language (e.g., "JavaScript", "Python")
    pub primary_language: &'static str,
    /// Optional documentation URL
    pub documentation_url: Option<&'static str>,
}
