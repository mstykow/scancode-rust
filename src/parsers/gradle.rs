use std::fs;
use std::path::Path;

use log::warn;
use packageurl::PackageUrl;

use crate::models::{Dependency, PackageData};
use crate::parsers::PackageParser;

pub struct GradleParser;

impl PackageParser for GradleParser {
    const PACKAGE_TYPE: &'static str = "maven";

    fn is_match(path: &Path) -> bool {
        path.file_name().is_some_and(|name| {
            let name_str = name.to_string_lossy();
            name_str == "build.gradle" || name_str == "build.gradle.kts"
        })
    }

    fn extract_package_data(path: &Path) -> PackageData {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read {:?}: {}", path, e);
                return default_package_data();
            }
        };

        let tokens = lex(&content);
        let dependencies = extract_dependencies(&tokens);

        PackageData {
            package_type: Some(Self::PACKAGE_TYPE.to_string()),
            namespace: None,
            name: None,
            version: None,
            qualifiers: None,
            subpath: None,
            primary_language: None,
            description: None,
            release_date: None,
            parties: Vec::new(),
            keywords: Vec::new(),
            homepage_url: None,
            download_url: None,
            size: None,
            sha1: None,
            md5: None,
            sha256: None,
            sha512: None,
            bug_tracking_url: None,
            code_view_url: None,
            vcs_url: None,
            copyright: None,
            holder: None,
            declared_license_expression: None,
            declared_license_expression_spdx: None,
            license_detections: Vec::new(),
            other_license_expression: None,
            other_license_expression_spdx: None,
            other_license_detections: Vec::new(),
            extracted_license_statement: None,
            notice_text: None,
            source_packages: Vec::new(),
            file_references: Vec::new(),
            extra_data: None,
            dependencies,
            repository_homepage_url: None,
            repository_download_url: None,
            api_data_url: None,
            datasource_id: Some("build_gradle".to_string()),
            purl: None,
            is_private: false,
            is_virtual: false,
        }
    }
}

fn default_package_data() -> PackageData {
    PackageData {
        package_type: Some("maven".to_string()),
        namespace: None,
        name: None,
        version: None,
        qualifiers: None,
        subpath: None,
        primary_language: None,
        description: None,
        release_date: None,
        parties: Vec::new(),
        keywords: Vec::new(),
        homepage_url: None,
        download_url: None,
        size: None,
        sha1: None,
        md5: None,
        sha256: None,
        sha512: None,
        bug_tracking_url: None,
        code_view_url: None,
        vcs_url: None,
        copyright: None,
        holder: None,
        declared_license_expression: None,
        declared_license_expression_spdx: None,
        license_detections: Vec::new(),
        other_license_expression: None,
        other_license_expression_spdx: None,
        other_license_detections: Vec::new(),
        extracted_license_statement: None,
        notice_text: None,
        source_packages: Vec::new(),
        file_references: Vec::new(),
        extra_data: None,
        dependencies: Vec::new(),
        repository_homepage_url: None,
        repository_download_url: None,
        api_data_url: None,
        datasource_id: Some("build_gradle".to_string()),
        purl: None,
        is_private: false,
        is_virtual: false,
    }
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Str(String),
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    OpenBrace,
    CloseBrace,
    Colon,
    Comma,
    Equals,
}

fn lex(input: &str) -> Vec<Tok> {
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < len {
        let c = chars[i];

        if c == '/' && i + 1 < len && chars[i + 1] == '/' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        if c == '/' && i + 1 < len && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if c == '\'' {
            i += 1;
            let start = i;
            while i < len && chars[i] != '\'' {
                i += 1;
            }
            let val: String = chars[start..i].iter().collect();
            tokens.push(Tok::Str(val));
            if i < len {
                i += 1;
            }
            continue;
        }

        if c == '"' {
            i += 1;
            let start = i;
            while i < len && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < len {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let val: String = chars[start..i].iter().collect();
            tokens.push(Tok::Str(val));
            if i < len {
                i += 1;
            }
            continue;
        }

        match c {
            '(' => {
                tokens.push(Tok::OpenParen);
                i += 1;
            }
            ')' => {
                tokens.push(Tok::CloseParen);
                i += 1;
            }
            '[' => {
                tokens.push(Tok::OpenBracket);
                i += 1;
            }
            ']' => {
                tokens.push(Tok::CloseBracket);
                i += 1;
            }
            '{' => {
                tokens.push(Tok::OpenBrace);
                i += 1;
            }
            '}' => {
                tokens.push(Tok::CloseBrace);
                i += 1;
            }
            ':' => {
                tokens.push(Tok::Colon);
                i += 1;
            }
            ',' => {
                tokens.push(Tok::Comma);
                i += 1;
            }
            '=' => {
                tokens.push(Tok::Equals);
                i += 1;
            }
            _ if is_ident_start(c) => {
                let start = i;
                while i < len && is_ident_char(chars[i]) {
                    i += 1;
                }
                let val: String = chars[start..i].iter().collect();
                tokens.push(Tok::Ident(val));
            }
            _ => {
                i += 1;
            }
        }
    }

    tokens
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' || c == '$'
}

// ---------------------------------------------------------------------------
// Dependency block extraction
// ---------------------------------------------------------------------------

#[allow(clippy::collapsible_if)]
fn find_dependency_blocks(tokens: &[Tok]) -> Vec<Vec<Tok>> {
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        if let Tok::Ident(ref name) = tokens[i] {
            if name == "dependencies" && i + 1 < tokens.len() && tokens[i + 1] == Tok::OpenBrace {
                i += 2;
                let mut depth = 1;
                let start = i;
                while i < tokens.len() && depth > 0 {
                    match &tokens[i] {
                        Tok::OpenBrace => depth += 1,
                        Tok::CloseBrace => depth -= 1,
                        _ => {}
                    }
                    if depth > 0 {
                        i += 1;
                    }
                }
                blocks.push(tokens[start..i].to_vec());
                if i < tokens.len() {
                    i += 1;
                }
                continue;
            }
        }
        i += 1;
    }

    blocks
}

// ---------------------------------------------------------------------------
// Dependency extraction from blocks
// ---------------------------------------------------------------------------

struct RawDep {
    namespace: String,
    name: String,
    version: String,
    scope: String,
}

fn extract_dependencies(tokens: &[Tok]) -> Vec<Dependency> {
    let blocks = find_dependency_blocks(tokens);
    let mut dependencies = Vec::new();

    if let Some(block) = blocks.first() {
        let raw = parse_block(block);
        for rd in raw {
            if rd.name.is_empty() {
                continue;
            }
            if let Some(dep) = create_dependency(&rd.namespace, &rd.name, &rd.version, &rd.scope) {
                dependencies.push(dep);
            }
        }
    }

    dependencies
}

#[allow(clippy::collapsible_if)]
fn parse_block(tokens: &[Tok]) -> Vec<RawDep> {
    let mut deps = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        // Skip nested blocks (closures like `{ transitive = true }`)
        if tokens[i] == Tok::OpenBrace {
            let mut depth = 1;
            i += 1;
            while i < tokens.len() && depth > 0 {
                match &tokens[i] {
                    Tok::OpenBrace => depth += 1,
                    Tok::CloseBrace => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            continue;
        }

        let scope_name = match &tokens[i] {
            Tok::Ident(name) => name.clone(),
            _ => {
                i += 1;
                continue;
            }
        };

        if is_skip_keyword(&scope_name) {
            i += 1;
            continue;
        }

        let next = i + 1;

        // PATTERN: scope ( ... )  — parenthesized dependency
        if next < tokens.len() && tokens[next] == Tok::OpenParen {
            let paren_end = find_matching_paren(tokens, next);
            if let Some(end) = paren_end {
                let inner = &tokens[next + 1..end];
                parse_paren_content(&scope_name, inner, &mut deps);
                i = end + 1;
                continue;
            }
        }

        // PATTERN: scope group: ..., name: ..., version: ... (named params without parens)
        if next < tokens.len() {
            if let Tok::Ident(ref label) = tokens[next] {
                if label == "group" && next + 1 < tokens.len() && tokens[next + 1] == Tok::Colon {
                    if let Some((rd, consumed)) = parse_named_params(&scope_name, &tokens[next..]) {
                        deps.push(rd);
                        i = next + consumed;
                        continue;
                    }
                }
            }
        }

        // PATTERN: scope 'string:notation' (string notation)
        if next < tokens.len() {
            if let Tok::Str(ref val) = tokens[next] {
                if val.contains(':') {
                    // `scope 'str', { closure }` → skip (unparenthesized call with trailing closure)
                    if next + 1 < tokens.len()
                        && tokens[next + 1] == Tok::Comma
                        && next + 2 < tokens.len()
                        && tokens[next + 2] == Tok::OpenBrace
                    {
                        i = next + 1;
                        continue;
                    }
                    let is_multi = i + 2 < tokens.len()
                        && tokens[next + 1] == Tok::Comma
                        && matches!(tokens.get(next + 2), Some(Tok::Str(_)));
                    let effective_scope = if is_multi { "" } else { &scope_name };
                    let rd = parse_colon_string(val, effective_scope);
                    deps.push(rd);
                    i = next + 1;
                    while i < tokens.len() && tokens[i] == Tok::Comma {
                        i += 1;
                        if i < tokens.len() {
                            if let Tok::Str(ref v2) = tokens[i] {
                                if v2.contains(':') {
                                    deps.push(parse_colon_string(v2, ""));
                                    i += 1;
                                    continue;
                                }
                            }
                        }
                        break;
                    }
                    continue;
                }
            }
        }

        // PATTERN: scope ident.attr (variable reference / dotted identifier)
        // Note: Skip references starting with "dependencies." as Python's pygmars
        // relabels the "dependencies" token, breaking the DEPENDENCY-5 grammar rule.
        if next < tokens.len() {
            if let Tok::Ident(ref val) = tokens[next] {
                if val.contains('.') && !val.starts_with("dependencies.") {
                    if let Some(last_seg) = val.rsplit('.').next() {
                        if !last_seg.is_empty() {
                            deps.push(RawDep {
                                namespace: String::new(),
                                name: last_seg.to_string(),
                                version: String::new(),
                                scope: scope_name.clone(),
                            });
                            i = next + 1;
                            continue;
                        }
                    }
                }
            }
        }

        // PATTERN: scope project(':module') — project reference without parens
        if next < tokens.len() {
            if let Tok::Ident(ref name) = tokens[next] {
                if name == "project"
                    && next + 1 < tokens.len()
                    && tokens[next + 1] == Tok::OpenParen
                {
                    if let Some(end) = find_matching_paren(tokens, next + 1) {
                        let inner = &tokens[next + 2..end];
                        if let Some(rd) = parse_project_ref(inner) {
                            deps.push(rd);
                        }
                        i = end + 1;
                        continue;
                    }
                }
            }
        }

        i += 1;
    }

    deps
}

fn is_skip_keyword(name: &str) -> bool {
    matches!(
        name,
        "plugins"
            | "apply"
            | "ext"
            | "configurations"
            | "repositories"
            | "subprojects"
            | "allprojects"
            | "buildscript"
            | "pluginManager"
            | "publishing"
            | "sourceSets"
            | "tasks"
            | "task"
    )
}

#[allow(clippy::collapsible_if)]
fn parse_paren_content(scope: &str, tokens: &[Tok], deps: &mut Vec<RawDep>) {
    if tokens.is_empty() {
        return;
    }

    // Check for bracket-enclosed maps: [group: ..., name: ..., version: ...]
    if tokens[0] == Tok::OpenBracket {
        parse_bracket_maps(tokens, deps);
        return;
    }

    // Check for named parameters: group: 'x' or group = "x"
    if let Some(Tok::Ident(label)) = tokens.first() {
        if label == "group" && tokens.len() > 1 && tokens[1] == Tok::Colon {
            if let Some((rd, _)) = parse_named_params("", tokens) {
                deps.push(rd);
            }
            return;
        }
    }

    // Check for nested function call or project reference
    if let Some(Tok::Ident(inner_fn)) = tokens.first() {
        if tokens.len() > 1 && tokens[1] == Tok::OpenParen {
            if inner_fn == "project" {
                if let Some(end) = find_matching_paren(tokens, 1) {
                    let inner = &tokens[2..end];
                    if let Some(rd) = parse_project_ref(inner) {
                        deps.push(rd);
                    }
                }
                return;
            }

            if let Some(end) = find_matching_paren(tokens, 1) {
                let inner = &tokens[2..end];
                if let Some(Tok::Str(val)) = inner.first() {
                    if val.contains(':') {
                        deps.push(parse_colon_string(val, inner_fn));
                        return;
                    }
                }
            }
        }
    }

    // Simple string: ("g:n:v")
    if let Some(Tok::Str(val)) = tokens.first() {
        if val.contains(':') {
            deps.push(parse_colon_string(val, scope));
        }
    }
}

#[allow(clippy::collapsible_if)]
fn parse_bracket_maps(tokens: &[Tok], deps: &mut Vec<RawDep>) {
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] == Tok::OpenBracket {
            if let Some(end) = find_matching_bracket(tokens, i) {
                let map_tokens = &tokens[i + 1..end];
                if let Some(rd) = parse_map_entries(map_tokens) {
                    deps.push(rd);
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
}

#[allow(clippy::collapsible_if)]
fn parse_map_entries(tokens: &[Tok]) -> Option<RawDep> {
    let mut name = String::new();
    let mut version = String::new();
    let mut i = 0;

    while i < tokens.len() {
        if let Tok::Ident(ref label) = tokens[i] {
            if i + 2 < tokens.len() && tokens[i + 1] == Tok::Colon {
                if let Tok::Str(ref val) = tokens[i + 2] {
                    match label.as_str() {
                        "name" => name = val.clone(),
                        "version" => version = val.clone(),
                        _ => {}
                    }
                    i += 3;
                    if i < tokens.len() && tokens[i] == Tok::Comma {
                        i += 1;
                    }
                    continue;
                }
            }
        }
        i += 1;
    }

    if name.is_empty() {
        return None;
    }

    Some(RawDep {
        namespace: String::new(),
        name,
        version,
        scope: String::new(),
    })
}

#[allow(clippy::collapsible_if)]
fn parse_named_params(scope: &str, tokens: &[Tok]) -> Option<(RawDep, usize)> {
    let mut group = String::new();
    let mut name = String::new();
    let mut version = String::new();
    let mut i = 0;

    while i < tokens.len() {
        if let Tok::Ident(ref label) = tokens[i] {
            if i + 2 < tokens.len() && tokens[i + 1] == Tok::Colon {
                if let Tok::Str(ref val) = tokens[i + 2] {
                    match label.as_str() {
                        "group" => group = val.clone(),
                        "name" => name = val.clone(),
                        "version" => version = val.clone(),
                        _ => {
                            i += 3;
                            if i < tokens.len() && tokens[i] == Tok::Comma {
                                i += 1;
                            }
                            continue;
                        }
                    }
                    i += 3;
                    if i < tokens.len() && tokens[i] == Tok::Comma {
                        i += 1;
                    }
                    continue;
                }
            }
        }
        break;
    }

    if name.is_empty() {
        return None;
    }

    Some((
        RawDep {
            namespace: group,
            name,
            version,
            scope: scope.to_string(),
        },
        i,
    ))
}

fn parse_project_ref(tokens: &[Tok]) -> Option<RawDep> {
    if let Some(Tok::Str(val)) = tokens.first() {
        let module_name = val.trim_start_matches(':');
        let name = module_name.rsplit(':').next().unwrap_or(module_name);
        if name.is_empty() {
            return None;
        }
        return Some(RawDep {
            namespace: String::new(),
            name: name.to_string(),
            version: String::new(),
            scope: "project".to_string(),
        });
    }
    None
}

fn parse_colon_string(val: &str, scope: &str) -> RawDep {
    let parts: Vec<&str> = val.split(':').collect();
    let (namespace, name, version) = match parts.len() {
        n if n >= 4 => (
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
        ),
        3 => (
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
        ),
        2 => (parts[0].to_string(), parts[1].to_string(), String::new()),
        _ => (String::new(), val.to_string(), String::new()),
    };

    RawDep {
        namespace,
        name,
        version,
        scope: scope.to_string(),
    }
}

fn find_matching_paren(tokens: &[Tok], start: usize) -> Option<usize> {
    if tokens.get(start) != Some(&Tok::OpenParen) {
        return None;
    }
    let mut depth = 1;
    let mut i = start + 1;
    while i < tokens.len() && depth > 0 {
        match &tokens[i] {
            Tok::OpenParen => depth += 1,
            Tok::CloseParen => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_matching_bracket(tokens: &[Tok], start: usize) -> Option<usize> {
    if tokens.get(start) != Some(&Tok::OpenBracket) {
        return None;
    }
    let mut depth = 1;
    let mut i = start + 1;
    while i < tokens.len() && depth > 0 {
        match &tokens[i] {
            Tok::OpenBracket => depth += 1,
            Tok::CloseBracket => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return Some(i);
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// Dependency construction
// ---------------------------------------------------------------------------

fn create_dependency(
    namespace: &str,
    name: &str,
    version: &str,
    scope: &str,
) -> Option<Dependency> {
    if name.is_empty() {
        return None;
    }

    let mut purl = PackageUrl::new("maven", name).ok()?;

    if !namespace.is_empty() {
        purl.with_namespace(namespace).ok()?;
    }

    if !version.is_empty() {
        purl.with_version(version).ok()?;
    }

    let scope_lower = scope.to_lowercase();
    let is_runtime = !scope_lower.contains("test");
    let is_optional = scope_lower.contains("test");
    let is_pinned = !version.is_empty();

    let purl_string = purl.to_string().replace("$", "%24");
    Some(Dependency {
        purl: Some(purl_string),
        extracted_requirement: Some(version.to_string()),
        scope: Some(scope.to_string()),
        is_runtime: Some(is_runtime),
        is_optional: Some(is_optional),
        is_pinned: Some(is_pinned),
        is_direct: Some(true),
        resolved_package: None,
        extra_data: None,
    })
}

crate::register_parser!(
    "Gradle build script",
    &["**/build.gradle", "**/build.gradle.kts"],
    "maven",
    "Java",
    Some("https://gradle.org/"),
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_match() {
        assert!(GradleParser::is_match(Path::new("build.gradle")));
        assert!(GradleParser::is_match(Path::new("build.gradle.kts")));
        assert!(GradleParser::is_match(Path::new("project/build.gradle")));
        assert!(!GradleParser::is_match(Path::new("build.xml")));
        assert!(!GradleParser::is_match(Path::new("settings.gradle")));
    }

    #[test]
    fn test_extract_simple_dependencies() {
        let content = r#"
dependencies {
    compile 'org.apache.commons:commons-text:1.1'
    testCompile 'junit:junit:4.12'
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 2);

        let dep1 = &deps[0];
        assert_eq!(
            dep1.purl,
            Some("pkg:maven/org.apache.commons/commons-text@1.1".to_string())
        );
        assert_eq!(dep1.scope, Some("compile".to_string()));
        assert_eq!(dep1.is_runtime, Some(true));
        assert_eq!(dep1.is_pinned, Some(true));

        let dep2 = &deps[1];
        assert_eq!(dep2.purl, Some("pkg:maven/junit/junit@4.12".to_string()));
        assert_eq!(dep2.scope, Some("testCompile".to_string()));
        assert_eq!(dep2.is_runtime, Some(false));
        assert_eq!(dep2.is_optional, Some(true));
    }

    #[test]
    fn test_extract_parens_notation() {
        let content = r#"
dependencies {
    implementation("com.example:library:1.0.0")
    testImplementation("junit:junit:4.13")
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 2);
        assert_eq!(
            deps[0].purl,
            Some("pkg:maven/com.example/library@1.0.0".to_string())
        );
    }

    #[test]
    fn test_extract_named_parameters() {
        let content = r#"
dependencies {
    api group: 'com.google.guava', name: 'guava', version: '30.1-jre'
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 1);
        assert_eq!(
            deps[0].purl,
            Some("pkg:maven/com.google.guava/guava@30.1-jre".to_string())
        );
        assert_eq!(deps[0].scope, Some("api".to_string()));
    }

    #[test]
    fn test_multiple_dependency_blocks_first_only() {
        let content = r#"
dependencies {
    implementation 'org.scala-lang:scala-library:2.11.12'
}

dependencies {
    implementation 'commons-collections:commons-collections:3.2.2'
    testImplementation 'junit:junit:4.13'
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 1);
    }

    #[test]
    fn test_no_version() {
        let content = r#"
dependencies {
    compile 'org.example:library'
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].is_pinned, Some(false));
        assert_eq!(deps[0].extracted_requirement, Some("".to_string()));
    }

    #[test]
    fn test_nested_function_calls() {
        let content = r#"
dependencies {
    implementation(enforcedPlatform("com.fasterxml.jackson:jackson-bom:2.12.2"))
    testImplementation(platform("org.junit:junit-bom:5.7.2"))
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 2);
        assert_eq!(
            deps[0].purl,
            Some("pkg:maven/com.fasterxml.jackson/jackson-bom@2.12.2".to_string())
        );
        assert_eq!(deps[0].scope, Some("enforcedPlatform".to_string()));
        assert_eq!(deps[1].scope, Some("platform".to_string()));
    }

    #[test]
    fn test_map_format() {
        let content = r#"
dependencies {
    runtimeOnly(
        [group: 'org.jacoco', name: 'org.jacoco.ant', version: '0.7.4.201502262128'],
        [group: 'org.jacoco', name: 'org.jacoco.agent', version: '0.7.4.201502262128']
    )
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].scope, Some("".to_string()));
        assert_eq!(
            deps[0].purl,
            Some("pkg:maven/org.jacoco.ant@0.7.4.201502262128".to_string())
        );
    }

    #[test]
    fn test_project_references() {
        let content = r#"
dependencies {
    implementation(project(":documentation"))
    implementation(project(":basics"))
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].scope, Some("project".to_string()));
        assert_eq!(deps[0].purl, Some("pkg:maven/documentation".to_string()));
        assert_eq!(deps[1].purl, Some("pkg:maven/basics".to_string()));
    }

    #[test]
    fn test_string_interpolation() {
        let content = r#"
dependencies {
    compile "com.amazonaws:aws-java-sdk-core:${awsVer}"
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].extracted_requirement, Some("${awsVer}".to_string()));
        assert_eq!(
            deps[0].purl,
            Some("pkg:maven/com.amazonaws/aws-java-sdk-core@%24%7BawsVer%7D".to_string())
        );
    }

    #[test]
    fn test_multi_value_string_notation() {
        let content = r#"
dependencies {
    runtimeOnly 'org.springframework:spring-core:2.5',
            'org.springframework:spring-aop:2.5'
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].scope, Some("".to_string()));
        assert_eq!(deps[1].scope, Some("".to_string()));
    }

    #[test]
    fn test_kotlin_quoted_scope_not_extracted() {
        let content = r#"
dependencies {
    "js"("jquery:jquery:3.2.1@js")
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 0);
    }

    #[test]
    fn test_closure_after_dependency() {
        let content = r#"
dependencies {
    runtimeOnly('org.hibernate:hibernate:3.0.5') {
        transitive = true
    }
}
"#;
        let tokens = lex(content);
        let deps = extract_dependencies(&tokens);
        assert_eq!(deps.len(), 1);
        assert_eq!(
            deps[0].purl,
            Some("pkg:maven/org.hibernate/hibernate@3.0.5".to_string())
        );
        assert_eq!(deps[0].scope, Some("runtimeOnly".to_string()));
    }
}
