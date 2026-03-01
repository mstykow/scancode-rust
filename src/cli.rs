use clap::Parser;

use crate::output::OutputFormat;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Directory path to scan
    pub dir_path: String,

    /// Output file path
    #[arg(default_value = "output.json", short)]
    pub output_file: String,

    #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
    pub format: OutputFormat,

    #[arg(long, required_if_eq("format", "custom-template"))]
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
