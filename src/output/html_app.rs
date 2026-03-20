use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use tera::{Context, Tera};

use crate::models::Output;

use super::OutputWriteConfig;
use super::shared::io_other;

pub(crate) fn write_html_app(
    output_file: &str,
    output: &Output,
    config: &OutputWriteConfig,
) -> io::Result<()> {
    let output_path = Path::new(output_file);
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("scan")
        .to_string();
    let assets_name = format!("{}_files", stem);
    let assets_dir = parent.join(&assets_name);

    if assets_dir.exists() {
        fs::remove_dir_all(&assets_dir)?;
    }
    fs::create_dir_all(&assets_dir)?;

    let data_js_path = assets_dir.join("data.js");
    let css_path = assets_dir.join("app.css");
    let js_path = assets_dir.join("app.js");

    let mut data_js = File::create(data_js_path)?;
    data_js.write_all(b"window.PROVENANT_DATA=")?;
    serde_json::to_writer(&mut data_js, output).map_err(io_other)?;
    data_js.write_all(b";\n")?;

    fs::write(css_path, HTML_APP_CSS)?;
    fs::write(js_path, HTML_APP_JS)?;

    let mut context = Context::new();
    context.insert("assets_dir", &assets_name);
    context.insert(
        "scanned_path",
        &config
            .scanned_path
            .clone()
            .unwrap_or_else(|| "<unknown>".to_string()),
    );
    context.insert("version", env!("CARGO_PKG_VERSION"));

    let rendered = Tera::one_off(HTML_APP_TEMPLATE, &context, true).map_err(io_other)?;
    fs::write(output_file, rendered.as_bytes())
}

const HTML_APP_TEMPLATE: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Provenant HTML app</title>
    <link rel="stylesheet" href="{{ assets_dir }}/app.css" />
  </head>
  <body>
    <header>
      <h1>Provenant HTML app</h1>
      <p>Scanned path: {{ scanned_path }}</p>
      <p>Version: {{ version }}</p>
    </header>
    <main id="app"></main>
    <script src="{{ assets_dir }}/data.js"></script>
    <script src="{{ assets_dir }}/app.js"></script>
  </body>
</html>
"#;

const HTML_APP_CSS: &str = r#"
body { font-family: system-ui, sans-serif; margin: 1.5rem; }
table { border-collapse: collapse; width: 100%; }
th, td { border: 1px solid #ddd; padding: .4rem; }
th { background: #f5f5f5; }
"#;

const HTML_APP_JS: &str = r#"
(function () {
  const root = document.getElementById('app');
  const data = window.PROVENANT_DATA || {};
  const files = data.files || [];
  const rows = files
    .map((f) => `<tr><td>${f.path || ''}</td><td>${f.type || ''}</td><td>${f.size || ''}</td></tr>`)
    .join('');
  root.innerHTML = `
    <h2>Files (${files.length})</h2>
    <table>
      <thead><tr><th>Path</th><th>Type</th><th>Size</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
  `;
})();
"#;
