use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// maharajah — a local semantic search engine for source code
#[derive(Parser, Debug)]
#[command(
    name = "maharajah",
    version,
    about = "Semantically search your codebase using UniXcoder embeddings",
    long_about = None
)]
pub struct Cli {
    /// Path to a TOML configuration file
    /// (default: ~/.maharajah/maharajah.toml)
    #[arg(short, long, global = true, env = "MAHARAJAH_CONFIG", value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Target project directory (default: current working directory)
    #[arg(short = 'D', long = "dir", global = true, value_name = "DIR")]
    pub target_dir: Option<PathBuf>,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Index source files into the vector database
    Index(IndexArgs),

    /// Find relevant code chunks by semantic similarity (content vectors only)
    Find(FindArgs),

    /// Search using both content and summary embeddings, merged with RRF
    Query(FindArgs),

    /// Manage the vector database (stats, clear)
    Db(DbArgs),

    /// Print the resolved configuration as JSON and exit
    Config,
}

#[derive(Args, Debug)]
pub struct IndexArgs {
    /// File glob patterns to include (repeatable, e.g. --include '**/*.rs')
    #[arg(short, long, value_name = "GLOB")]
    pub include: Vec<String>,

    /// File glob patterns to exclude (repeatable)
    #[arg(short = 'x', long, value_name = "GLOB")]
    pub exclude: Vec<String>,

    /// Maximum chunk size in source lines
    #[arg(long, default_value_t = 40)]
    pub chunk_lines: usize,

    /// Wipe and rebuild the index from scratch (required after schema migration)
    #[arg(long)]
    pub reindex: bool,
}

#[derive(Args, Debug)]
pub struct FindArgs {
    /// Natural language query to search for
    pub prompt: String,

    /// Maximum number of results to show
    #[arg(short = 'n', long, default_value_t = 10)]
    pub limit: usize,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Args, Debug)]
pub struct DbArgs {
    #[command(subcommand)]
    pub action: DbAction,
}

#[derive(Subcommand, Debug)]
pub enum DbAction {
    /// Show index statistics (file count, chunk count, embedding dimensions)
    Stats,
    /// Remove all indexed data (requires --yes)
    Clear {
        #[arg(long)]
        yes: bool,
    },
}

#[derive(clap::ValueEnum, Debug, Clone)]
pub enum OutputFormat {
    Text,
    Json,
}
