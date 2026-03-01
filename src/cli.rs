use clap::{ArgGroup, Parser};

use crate::output::OutputFormat;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = None,
    group(
        ArgGroup::new("output")
            .required(true)
            .args([
                "output_json",
                "output_json_pp",
                "output_json_lines",
                "output_yaml",
                "output_csv",
                "output_html",
                "output_html_app",
                "output_spdx_tv",
                "output_spdx_rdf",
                "output_cyclonedx",
                "output_cyclonedx_xml",
                "custom_output"
            ])
    )
)]
pub struct Cli {
    /// Directory path to scan
    pub dir_path: String,

    /// Write scan output as compact JSON to FILE
    #[arg(long = "json", value_name = "FILE", allow_hyphen_values = true)]
    pub output_json: Option<String>,

    /// Write scan output as pretty-printed JSON to FILE
    #[arg(long = "json-pp", value_name = "FILE", allow_hyphen_values = true)]
    pub output_json_pp: Option<String>,

    /// Write scan output as JSON Lines to FILE
    #[arg(long = "json-lines", value_name = "FILE", allow_hyphen_values = true)]
    pub output_json_lines: Option<String>,

    /// Write scan output as YAML to FILE
    #[arg(long = "yaml", value_name = "FILE", allow_hyphen_values = true)]
    pub output_yaml: Option<String>,

    /// [DEPRECATED in Python] Write scan output as CSV to FILE
    #[arg(long = "csv", value_name = "FILE", allow_hyphen_values = true)]
    pub output_csv: Option<String>,

    /// Write scan output as HTML report to FILE
    #[arg(long = "html", value_name = "FILE", allow_hyphen_values = true)]
    pub output_html: Option<String>,

    /// [DEPRECATED in Python] Write scan output as HTML app to FILE
    #[arg(
        long = "html-app",
        value_name = "FILE",
        hide = true,
        allow_hyphen_values = true
    )]
    pub output_html_app: Option<String>,

    /// Write scan output as SPDX tag/value to FILE
    #[arg(long = "spdx-tv", value_name = "FILE", allow_hyphen_values = true)]
    pub output_spdx_tv: Option<String>,

    /// Write scan output as SPDX RDF/XML to FILE
    #[arg(long = "spdx-rdf", value_name = "FILE", allow_hyphen_values = true)]
    pub output_spdx_rdf: Option<String>,

    /// Write scan output as CycloneDX JSON to FILE
    #[arg(long = "cyclonedx", value_name = "FILE", allow_hyphen_values = true)]
    pub output_cyclonedx: Option<String>,

    /// Write scan output as CycloneDX XML to FILE
    #[arg(
        long = "cyclonedx-xml",
        value_name = "FILE",
        allow_hyphen_values = true
    )]
    pub output_cyclonedx_xml: Option<String>,

    /// Write scan output to FILE formatted with the custom template
    #[arg(
        long = "custom-output",
        value_name = "FILE",
        requires = "custom_template",
        allow_hyphen_values = true
    )]
    pub custom_output: Option<String>,

    /// Use this template FILE with --custom-output
    #[arg(
        long = "custom-template",
        value_name = "FILE",
        requires = "custom_output"
    )]
    pub custom_template: Option<String>,

    /// Maximum recursion depth (0 means no recursion)
    #[arg(short, long, default_value = "50")]
    pub max_depth: usize,

    /// Exclude patterns (glob patterns like "*.tmp" or "node_modules")
    #[arg(short, long, value_delimiter = ',')]
    pub exclude: Vec<String>,

    /// Disable package assembly (merging related manifest/lockfiles into packages)
    #[arg(long)]
    pub no_assemble: bool,

    /// Scan input for email addresses
    #[arg(long)]
    pub email: bool,

    /// Report only up to INT emails found in a file. Use 0 for no limit.
    #[arg(long, default_value_t = 50, requires = "email")]
    pub max_email: usize,

    /// Scan input for URLs
    #[arg(short = 'u', long)]
    pub url: bool,

    /// Report only up to INT URLs found in a file. Use 0 for no limit.
    #[arg(long, default_value_t = 50, requires = "url")]
    pub max_url: usize,
}

#[derive(Debug, Clone)]
pub struct OutputTarget {
    pub format: OutputFormat,
    pub file: String,
    pub custom_template: Option<String>,
}

impl Cli {
    pub fn output_targets(&self) -> Vec<OutputTarget> {
        let mut targets = Vec::new();

        if let Some(file) = &self.output_json {
            targets.push(OutputTarget {
                format: OutputFormat::Json,
                file: file.clone(),
                custom_template: None,
            });
        }

        if let Some(file) = &self.output_json_pp {
            targets.push(OutputTarget {
                format: OutputFormat::JsonPretty,
                file: file.clone(),
                custom_template: None,
            });
        }

        if let Some(file) = &self.output_json_lines {
            targets.push(OutputTarget {
                format: OutputFormat::JsonLines,
                file: file.clone(),
                custom_template: None,
            });
        }

        if let Some(file) = &self.output_yaml {
            targets.push(OutputTarget {
                format: OutputFormat::Yaml,
                file: file.clone(),
                custom_template: None,
            });
        }

        if let Some(file) = &self.output_csv {
            targets.push(OutputTarget {
                format: OutputFormat::Csv,
                file: file.clone(),
                custom_template: None,
            });
        }

        if let Some(file) = &self.output_html {
            targets.push(OutputTarget {
                format: OutputFormat::Html,
                file: file.clone(),
                custom_template: None,
            });
        }

        if let Some(file) = &self.output_html_app {
            targets.push(OutputTarget {
                format: OutputFormat::HtmlApp,
                file: file.clone(),
                custom_template: None,
            });
        }

        if let Some(file) = &self.output_spdx_tv {
            targets.push(OutputTarget {
                format: OutputFormat::SpdxTv,
                file: file.clone(),
                custom_template: None,
            });
        }

        if let Some(file) = &self.output_spdx_rdf {
            targets.push(OutputTarget {
                format: OutputFormat::SpdxRdf,
                file: file.clone(),
                custom_template: None,
            });
        }

        if let Some(file) = &self.output_cyclonedx {
            targets.push(OutputTarget {
                format: OutputFormat::CycloneDxJson,
                file: file.clone(),
                custom_template: None,
            });
        }

        if let Some(file) = &self.output_cyclonedx_xml {
            targets.push(OutputTarget {
                format: OutputFormat::CycloneDxXml,
                file: file.clone(),
                custom_template: None,
            });
        }

        if let Some(file) = &self.custom_output {
            targets.push(OutputTarget {
                format: OutputFormat::CustomTemplate,
                file: file.clone(),
                custom_template: self.custom_template.clone(),
            });
        }

        targets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_at_least_one_output_option() {
        let parsed = Cli::try_parse_from(["scancode-rust", "samples"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_parses_json_pretty_output_option() {
        let parsed = Cli::try_parse_from(["scancode-rust", "--json-pp", "scan.json", "samples"])
            .expect("cli parse should succeed");

        assert_eq!(parsed.output_json_pp.as_deref(), Some("scan.json"));
        assert_eq!(parsed.output_targets().len(), 1);
        assert_eq!(parsed.output_targets()[0].format, OutputFormat::JsonPretty);
    }

    #[test]
    fn test_allows_stdout_dash_as_output_target() {
        let parsed = Cli::try_parse_from(["scancode-rust", "--json-pp", "-", "samples"])
            .expect("cli parse should allow stdout dash output target");

        assert_eq!(parsed.output_json_pp.as_deref(), Some("-"));
    }

    #[test]
    fn test_custom_template_and_output_must_be_paired() {
        let missing_template =
            Cli::try_parse_from(["scancode-rust", "--custom-output", "result.txt", "samples"]);
        assert!(missing_template.is_err());

        let missing_output =
            Cli::try_parse_from(["scancode-rust", "--custom-template", "tpl.tera", "samples"]);
        assert!(missing_output.is_err());
    }
}
