use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Directory path to scan
    pub dir_path: String,

    /// Output file path
    #[arg(default_value = "output.json", short)]
    pub output_file: String,

    /// Maximum recursion depth (0 means no recursion)
    #[arg(short, long, default_value = "50")]
    pub max_depth: usize,

    /// Exclude patterns (glob patterns like "*.tmp" or "node_modules")
    #[arg(short, long, value_delimiter = ',')]
    pub exclude: Vec<String>,
}
