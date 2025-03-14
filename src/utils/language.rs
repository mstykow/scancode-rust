use content_inspector::{ContentType, inspect};
use std::path::Path;

/// Detect programming language based on file extension and contents
pub fn detect_language(path: &Path, content: &[u8]) -> String {
    // Skip binary files
    if content.len() > 32 && inspect(content) != ContentType::UTF_8 {
        return "Binary".to_string();
    }

    // Check for shebang in script files
    if content.len() > 2 && content[0] == b'#' && content[1] == b'!' {
        let shebang_end = content
            .iter()
            .position(|&b| b == b'\n')
            .unwrap_or(content.len());
        let shebang = String::from_utf8_lossy(&content[0..shebang_end]);

        if shebang.contains("python") {
            return "Python".to_string();
        } else if shebang.contains("node") {
            return "JavaScript".to_string();
        } else if shebang.contains("ruby") {
            return "Ruby".to_string();
        } else if shebang.contains("perl") {
            return "Perl".to_string();
        } else if shebang.contains("php") {
            return "PHP".to_string();
        } else if shebang.contains("bash") || shebang.contains("sh") {
            return "Shell".to_string();
        }
    }

    // Check file extension
    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        match extension.to_lowercase().as_str() {
            "rs" => return "Rust".to_string(),
            "py" => return "Python".to_string(),
            "js" => return "JavaScript".to_string(),
            "ts" => return "TypeScript".to_string(),
            "html" | "htm" => return "HTML".to_string(),
            "css" => return "CSS".to_string(),
            "c" => return "C".to_string(),
            "cpp" | "cc" | "cxx" => return "C++".to_string(),
            "h" | "hpp" => return "C/C++ Header".to_string(),
            "java" => return "Java".to_string(),
            "go" => return "Go".to_string(),
            "rb" => return "Ruby".to_string(),
            "php" => return "PHP".to_string(),
            "pl" => return "Perl".to_string(),
            "swift" => return "Swift".to_string(),
            "md" | "markdown" => return "Markdown".to_string(),
            "json" => return "JSON".to_string(),
            "xml" => return "XML".to_string(),
            "yml" | "yaml" => return "YAML".to_string(),
            "sql" => return "SQL".to_string(),
            "sh" | "bash" => return "Shell".to_string(),
            "kt" | "kts" => return "Kotlin".to_string(),
            "dart" => return "Dart".to_string(),
            "scala" => return "Scala".to_string(),
            "cs" => return "C#".to_string(),
            "fs" => return "F#".to_string(),
            "r" => return "R".to_string(),
            "lua" => return "Lua".to_string(),
            "jl" => return "Julia".to_string(),
            "ex" | "exs" => return "Elixir".to_string(),
            "clj" => return "Clojure".to_string(),
            "hs" => return "Haskell".to_string(),
            "erl" => return "Erlang".to_string(),
            "sc" => return "SuperCollider".to_string(),
            "tex" => return "TeX".to_string(),
            _ => {}
        }
    }

    // Check file name for special cases
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if file_name == "dockerfile" || file_name.starts_with("dockerfile.") {
        return "Dockerfile".to_string();
    } else if file_name == "makefile" {
        return "Makefile".to_string();
    } else if file_name == "gemfile" {
        return "Ruby".to_string();
    } else if file_name == "rakefile" {
        return "Ruby".to_string();
    }

    // Content-based detection as a fallback for plain text files
    if inspect(content) == ContentType::UTF_8 {
        // Check for common patterns in the content
        let text_sample = String::from_utf8_lossy(&content[..std::cmp::min(content.len(), 1000)]);

        if text_sample.contains("<?php") {
            return "PHP".to_string();
        } else if text_sample.contains("<html") || text_sample.contains("<!DOCTYPE html") {
            return "HTML".to_string();
        } else if text_sample.contains("import React") || text_sample.contains("import {") {
            return "JavaScript/TypeScript".to_string();
        } else if text_sample.contains("def ") && text_sample.contains(":") {
            return "Python".to_string();
        } else if text_sample.contains("package ")
            && text_sample.contains("import ")
            && text_sample.contains("{")
        {
            return "Go".to_string();
        }

        return "Text".to_string();
    }

    "Unknown".to_string()
}
