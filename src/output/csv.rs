use std::collections::BTreeMap;
use std::io::{self, Write};

use serde_json::{Map, Value};

use crate::models::{FileType, Output};

use super::shared::{io_other, sorted_files};

type CsvRow = BTreeMap<String, String>;
type CsvRows = Vec<CsvRow>;
type CsvHeadersByGroup = BTreeMap<String, Vec<String>>;

pub(crate) fn write_csv(output: &Output, writer: &mut dyn Write) -> io::Result<()> {
    let (rows, headers_by_group) = flatten_rows(output);
    let mut headers = vec!["kind".to_string(), "path".to_string()];
    for group in ["info", "license", "copyright", "email", "url", "package"] {
        if let Some(group_headers) = headers_by_group.get(group) {
            headers.extend(group_headers.clone());
        }
    }

    let mut csv_writer = csv::Writer::from_writer(writer);
    csv_writer.write_record(&headers).map_err(io_other)?;

    for row in rows {
        let record: Vec<String> = headers
            .iter()
            .map(|header| row.get(header).cloned().unwrap_or_default())
            .collect();
        csv_writer.write_record(record).map_err(io_other)?;
    }

    csv_writer.flush().map_err(io_other)
}

fn flatten_rows(output: &Output) -> (CsvRows, CsvHeadersByGroup) {
    let mut rows = Vec::new();
    let mut headers_by_group: CsvHeadersByGroup = BTreeMap::new();

    for file in sorted_files(&output.files) {
        let normalized_path = normalize_csv_path(&file.path, file.file_type == FileType::Directory);

        let mut info = BTreeMap::new();
        info.insert("kind".to_string(), "info".to_string());
        info.insert("path".to_string(), normalized_path.clone());
        info.insert("name".to_string(), file.name.clone());
        info.insert(
            "type".to_string(),
            match file.file_type {
                FileType::File => "file",
                FileType::Directory => "directory",
            }
            .to_string(),
        );
        info.insert("size".to_string(), file.size.to_string());
        if let Some(mime_type) = &file.mime_type {
            info.insert("mime_type".to_string(), mime_type.clone());
        }
        if let Some(sha1) = &file.sha1 {
            info.insert("sha1".to_string(), sha1.clone());
        }
        info.insert("scan_errors".to_string(), file.scan_errors.join("\n"));
        push_csv_row("info", info, &mut rows, &mut headers_by_group);

        for detection in &file.license_detections {
            for m in &detection.matches {
                let mut lic = BTreeMap::new();
                lic.insert("kind".to_string(), "license".to_string());
                lic.insert("path".to_string(), normalized_path.clone());
                lic.insert(
                    "license_expression".to_string(),
                    detection.license_expression.clone(),
                );
                lic.insert("start_line".to_string(), m.start_line.to_string());
                lic.insert("end_line".to_string(), m.end_line.to_string());
                lic.insert(
                    "license_match__license_expression".to_string(),
                    m.license_expression.clone(),
                );
                lic.insert(
                    "license_match__license_expression_spdx".to_string(),
                    m.license_expression_spdx.clone(),
                );
                lic.insert(
                    "license_match__score".to_string(),
                    format!("{:.2}", m.score),
                );
                if let Some(rule_identifier) = &m.rule_identifier {
                    lic.insert(
                        "license_match__rule_identifier".to_string(),
                        rule_identifier.clone(),
                    );
                }
                push_csv_row("license", lic, &mut rows, &mut headers_by_group);
            }
        }

        for c in &file.copyrights {
            let mut row = BTreeMap::new();
            row.insert("kind".to_string(), "copyright".to_string());
            row.insert("path".to_string(), normalized_path.clone());
            row.insert("copyright".to_string(), c.copyright.clone());
            row.insert("start_line".to_string(), c.start_line.to_string());
            row.insert("end_line".to_string(), c.end_line.to_string());
            push_csv_row("copyright", row, &mut rows, &mut headers_by_group);
        }

        for h in &file.holders {
            let mut row = BTreeMap::new();
            row.insert("kind".to_string(), "holder".to_string());
            row.insert("path".to_string(), normalized_path.clone());
            row.insert("holder".to_string(), h.holder.clone());
            row.insert("start_line".to_string(), h.start_line.to_string());
            row.insert("end_line".to_string(), h.end_line.to_string());
            push_csv_row("copyright", row, &mut rows, &mut headers_by_group);
        }

        for a in &file.authors {
            let mut row = BTreeMap::new();
            row.insert("kind".to_string(), "author".to_string());
            row.insert("path".to_string(), normalized_path.clone());
            row.insert("author".to_string(), a.author.clone());
            row.insert("start_line".to_string(), a.start_line.to_string());
            row.insert("end_line".to_string(), a.end_line.to_string());
            push_csv_row("copyright", row, &mut rows, &mut headers_by_group);
        }

        for e in &file.emails {
            let mut row = BTreeMap::new();
            row.insert("kind".to_string(), "email".to_string());
            row.insert("path".to_string(), normalized_path.clone());
            row.insert("email".to_string(), e.email.clone());
            row.insert("start_line".to_string(), e.start_line.to_string());
            row.insert("end_line".to_string(), e.end_line.to_string());
            push_csv_row("email", row, &mut rows, &mut headers_by_group);
        }

        for u in &file.urls {
            let mut row = BTreeMap::new();
            row.insert("kind".to_string(), "url".to_string());
            row.insert("path".to_string(), normalized_path.clone());
            row.insert("url".to_string(), u.url.clone());
            row.insert("start_line".to_string(), u.start_line.to_string());
            row.insert("end_line".to_string(), u.end_line.to_string());
            push_csv_row("url", row, &mut rows, &mut headers_by_group);
        }

        for package in &file.package_data {
            let mut row = BTreeMap::new();
            row.insert("kind".to_string(), "package_data".to_string());
            row.insert("path".to_string(), normalized_path.clone());

            if let Ok(Value::Object(map)) = serde_json::to_value(package) {
                flatten_json_object_to_row(&map, "package__", &mut row);
            }
            push_csv_row("package", row, &mut rows, &mut headers_by_group);
        }
    }

    (rows, headers_by_group)
}

fn push_csv_row(
    group: &str,
    row: CsvRow,
    rows: &mut CsvRows,
    headers_by_group: &mut CsvHeadersByGroup,
) {
    let group_headers = headers_by_group.entry(group.to_string()).or_default();
    for key in row.keys() {
        if key == "kind" || key == "path" {
            continue;
        }
        if !group_headers.contains(key) {
            group_headers.push(key.clone());
        }
    }
    rows.push(row);
}

fn normalize_csv_path(path: &str, is_directory: bool) -> String {
    let mut normalized = path.trim_start_matches('/').to_string();
    if is_directory && !normalized.ends_with('/') {
        normalized.push('/');
    }
    normalized
}

fn flatten_json_object_to_row(map: &Map<String, Value>, prefix: &str, row: &mut CsvRow) {
    for (key, value) in map {
        let col = format!("{}{}", prefix, key);
        if key == "version"
            && let Value::String(version) = value
        {
            let version = if version.is_empty() || version.to_ascii_lowercase().starts_with('v') {
                version.clone()
            } else {
                format!("v {}", version)
            };
            row.insert(col, version);
            continue;
        }

        row.insert(col, value_to_string(value));
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => v.clone(),
        Value::Array(values) => values
            .iter()
            .map(value_to_string)
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}
