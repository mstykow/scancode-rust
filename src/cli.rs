use clap::{ArgGroup, Parser};

use crate::output::OutputFormat;

#[derive(Parser, Debug)]
#[command(
    author,
    version = env!("CARGO_PKG_VERSION"),
    long_version = concat!(
        env!("CARGO_PKG_VERSION"),
        "\n",
        "License detection uses data from ScanCode Toolkit (CC-BY-4.0). See NOTICE or --show_attribution."
    ),
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
                "custom_output",
                "show_attribution"
            ])
    )
)]
pub struct Cli {
    /// Directory path to scan
    #[arg(required = false)]
    pub dir_path: Vec<String>,

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

    /// Maximum recursion depth (0 means no depth limit)
    #[arg(short, long, default_value = "0")]
    pub max_depth: usize,

    #[arg(short = 'n', long, default_value_t = default_processes(), allow_hyphen_values = true)]
    pub processes: i32,

    #[arg(long, default_value_t = 120.0)]
    pub timeout: f64,

    #[arg(short, long, conflicts_with = "verbose")]
    pub quiet: bool,

    #[arg(short, long, conflicts_with = "quiet")]
    pub verbose: bool,

    #[arg(long, conflicts_with = "full_root")]
    pub strip_root: bool,

    #[arg(long, conflicts_with = "strip_root")]
    pub full_root: bool,

    /// Exclude patterns (ScanCode-compatible alias: --ignore)
    #[arg(long = "exclude", visible_alias = "ignore", value_delimiter = ',')]
    pub exclude: Vec<String>,

    #[arg(long, value_delimiter = ',')]
    pub include: Vec<String>,

    #[arg(long = "cache-dir", value_name = "PATH")]
    pub cache_dir: Option<String>,

    #[arg(long = "cache-clear")]
    pub cache_clear: bool,

    #[arg(long = "max-in-memory", value_name = "INT")]
    pub max_in_memory: Option<usize>,

    #[arg(long)]
    pub from_json: bool,

    /// Disable package assembly (merging related manifest/lockfiles into packages)
    #[arg(long)]
    pub no_assemble: bool,

    /// Path to license rules directory containing .LICENSE and .RULE files.
    /// If not specified, uses the built-in embedded license index.
    #[arg(long, value_name = "PATH", requires = "license")]
    pub license_rules_path: Option<String>,

    /// Include matched text in license detection output
    #[arg(long, requires = "license")]
    pub include_text: bool,

    #[arg(long)]
    pub filter_clues: bool,

    #[arg(long)]
    pub only_findings: bool,

    #[arg(long)]
    pub mark_source: bool,

    #[arg(long)]
    pub classify: bool,

    #[arg(long = "facet", value_name = "<facet>=<pattern>")]
    pub facet: Vec<String>,

    #[arg(long = "tallies-by-facet", requires = "facet")]
    pub tallies_by_facet: bool,

    /// Scan input for licenses
    #[arg(short = 'l', long)]
    pub license: bool,

    #[arg(short = 'c', long)]
    pub copyright: bool,

    /// Scan input for email addresses
    #[arg(short = 'e', long)]
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

    /// Show attribution notices for embedded license detection data
    #[arg(long)]
    pub show_attribution: bool,
}

fn default_processes() -> i32 {
    let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
    if cpus > 1 {
        (cpus - 1) as i32
    } else {
        1
    }
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
        let parsed = Cli::try_parse_from(["provenant", "samples"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_parses_json_pretty_output_option() {
        let parsed = Cli::try_parse_from(["provenant", "--json-pp", "scan.json", "samples"])
            .expect("cli parse should succeed");

        assert_eq!(parsed.output_json_pp.as_deref(), Some("scan.json"));
        assert_eq!(parsed.output_targets().len(), 1);
        assert_eq!(parsed.output_targets()[0].format, OutputFormat::JsonPretty);
    }

    #[test]
    fn test_allows_stdout_dash_as_output_target() {
        let parsed = Cli::try_parse_from(["provenant", "--json-pp", "-", "samples"])
            .expect("cli parse should allow stdout dash output target");

        assert_eq!(parsed.output_json_pp.as_deref(), Some("-"));
    }

    #[test]
    fn test_custom_template_and_output_must_be_paired() {
        let missing_template =
            Cli::try_parse_from(["provenant", "--custom-output", "result.txt", "samples"]);
        assert!(missing_template.is_err());

        let missing_output =
            Cli::try_parse_from(["provenant", "--custom-template", "tpl.tera", "samples"]);
        assert!(missing_output.is_err());
    }

    #[test]
    fn test_parses_processes_and_timeout_options() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "-n",
            "4",
            "--timeout",
            "30",
            "samples",
        ])
        .expect("cli parse should succeed");

        assert_eq!(parsed.processes, 4);
        assert_eq!(parsed.timeout, 30.0);
    }

    #[test]
    fn test_strip_root_conflicts_with_full_root() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "--strip-root",
            "--full-root",
            "samples",
        ]);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_parses_include_and_only_findings_and_filter_clues() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "--include",
            "src/**,Cargo.toml",
            "--only-findings",
            "--filter-clues",
            "samples",
        ])
        .expect("cli parse should succeed");

        assert_eq!(parsed.include, vec!["src/**", "Cargo.toml"]);
        assert!(parsed.only_findings);
        assert!(parsed.filter_clues);
    }

    #[test]
    fn test_parses_ignore_alias_for_exclude_patterns() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "--ignore",
            "*.git*,target/*",
            "samples",
        ])
        .expect("cli parse should accept --ignore alias");

        assert_eq!(parsed.exclude, vec!["*.git*", "target/*"]);
    }

    #[test]
    fn test_quiet_conflicts_with_verbose() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "--quiet",
            "--verbose",
            "samples",
        ]);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_parses_from_json_and_mark_source() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "--from-json",
            "--mark-source",
            "sample-scan.json",
        ])
        .expect("cli parse should succeed");

        assert!(parsed.from_json);
        assert_eq!(parsed.dir_path, vec!["sample-scan.json"]);
        assert!(parsed.mark_source);
    }

    #[test]
    fn test_parses_classify_facet_and_tallies_by_facet() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "--classify",
            "--facet",
            "dev=*.c",
            "--facet",
            "tests=*/tests/*",
            "--tallies-by-facet",
            "samples",
        ])
        .expect("cli parse should succeed");

        assert!(parsed.classify);
        assert_eq!(parsed.facet, vec!["dev=*.c", "tests=*/tests/*"]);
        assert!(parsed.tallies_by_facet);
    }

    #[test]
    fn test_tallies_by_facet_requires_facet_definitions() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "--tallies-by-facet",
            "samples",
        ]);

        assert!(parsed.is_err());
    }

    #[test]
    fn test_parses_copyright_flag() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "--copyright",
            "samples",
        ])
        .expect("cli parse should succeed");

        assert!(parsed.copyright);
    }

    #[test]
    fn test_parses_license_flag() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "--license",
            "samples",
        ])
        .expect("cli parse should succeed");

        assert!(parsed.license);
    }

    #[test]
    fn test_license_short_flag() {
        let parsed = Cli::try_parse_from(["provenant", "--json-pp", "scan.json", "-l", "samples"])
            .expect("cli parse should succeed");

        assert!(parsed.license);
    }

    #[test]
    fn test_include_text_requires_license() {
        let result = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "--include-text",
            "samples",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parses_short_scan_flags() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "-c",
            "-e",
            "-u",
            "samples",
        ])
        .expect("cli parse should support short scan flags");

        assert!(parsed.copyright);
        assert!(parsed.email);
        assert!(parsed.url);
    }

    #[test]
    fn test_parses_processes_compat_values_zero_and_minus_one() {
        let zero =
            Cli::try_parse_from(["provenant", "--json-pp", "scan.json", "-n", "0", "samples"])
                .expect("cli parse should accept processes=0");
        assert_eq!(zero.processes, 0);

        let parsed =
            Cli::try_parse_from(["provenant", "--json-pp", "scan.json", "-n", "-1", "samples"])
                .expect("cli parse should accept processes=-1");
        assert_eq!(parsed.processes, -1);
    }

    #[test]
    fn test_parses_cache_flags() {
        let parsed = Cli::try_parse_from([
            "provenant",
            "--json-pp",
            "scan.json",
            "--cache-dir",
            "/tmp/sc-cache",
            "--cache-clear",
            "--max-in-memory",
            "5000",
            "samples",
        ])
        .expect("cli parse should accept cache flags");

        assert_eq!(parsed.cache_dir.as_deref(), Some("/tmp/sc-cache"));
        assert!(parsed.cache_clear);
        assert_eq!(parsed.max_in_memory, Some(5000));
    }

    #[test]
    fn test_max_depth_default_matches_reference_behavior() {
        let parsed = Cli::try_parse_from(["provenant", "--json-pp", "scan.json", "samples"])
            .expect("cli parse should succeed");

        assert_eq!(parsed.max_depth, 0);
    }
}
