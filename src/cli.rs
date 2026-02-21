use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// maharajah — a local RAG engine for source code
#[derive(Parser, Debug)]
#[command(
    name = "maharajah",
    version,
    about = "Query your codebase using local LLMs via Ollama",
    long_about = None
)]
pub struct Cli {
    /// Path to a TOML configuration file
    /// (default: $XDG_CONFIG_HOME/maharajah/config.toml)
    #[arg(short, long, global = true, env = "MAHARAJAH_CONFIG", value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Override the Ollama base URL (e.g. http://localhost:11434)
    #[arg(long, global = true, env = "MAHARAJAH_OLLAMA_URL")]
    pub ollama_url: Option<String>,

    /// Override the embeddings model name (e.g. nomic-embed-text)
    #[arg(long, global = true, env = "MAHARAJAH_EMBED_MODEL")]
    pub embed_model: Option<String>,

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

    /// Ask a question about your codebase (full RAG pipeline)
    Query(QueryArgs),

    /// Find relevant code chunks by semantic similarity
    Find(FindArgs),

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

    /// Wipe and rebuild the index from scratch
    #[arg(long)]
    pub reindex: bool,
}

#[derive(Args, Debug)]
pub struct QueryArgs {
    /// The question to ask about the codebase
    pub question: String,

    /// Number of code chunks to retrieve as context
    #[arg(short = 'k', long, default_value_t = 5)]
    pub top_k: usize,

    /// Override the generation model
    #[arg(long)]
    pub model: Option<String>,

    /// Print retrieved context chunks before the answer
    #[arg(long)]
    pub show_context: bool,
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
