use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub embed: EmbedConfig,
    pub db: DbConfig,
    pub index: IndexConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedConfig {
    /// HuggingFace model ID to use for embeddings.
    /// Defaults to "nomic-ai/CodeRankEmbed" (~550 MB, downloaded on first run).
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    /// Name of the table within LanceDB that stores chunks
    pub table_name: String,
    /// Embedding vector dimensionality (must match embed_model output)
    pub embedding_dim: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Default maximum lines per chunk when tree-sitter node is too large
    pub max_chunk_lines: usize,
    /// File extensions to auto-include when no --include glob is given
    pub default_extensions: Vec<String>,
    /// Glob patterns for paths to exclude from indexing
    pub default_excludes: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            embed: EmbedConfig {
                model_id: "nomic-ai/CodeRankEmbed".into(),
            },
            db: DbConfig {
                table_name: "chunks".into(),
                embedding_dim: 768,
            },
            index: IndexConfig {
                max_chunk_lines: 150,
                default_extensions: vec![
                    "rs".into(),
                    "py".into(),
                    "js".into(),
                    "cjs".into(),
                    "mjs".into(),
                    "jsx".into(),
                    "ts".into(),
                    "tsx".into(),
                    "go".into(),
                    "java".into(),
                    "cs".into(),
                    "fs".into(),
                    "fsx".into(),
                    "scala".into(),
                    "sc".into(),
                    "hs".into(),
                    "rb".into(),
                ],
                default_excludes: vec![
                    // Rust (cargo) — root and workspace members
                    "**/target/**".into(),
                    // JavaScript / TypeScript
                    "**/node_modules/**".into(),
                    // Python
                    "**/__pycache__/**".into(),
                    ".venv/**".into(),
                    "venv/**".into(),
                    "env/**".into(),
                    // Go
                    "vendor/**".into(),
                    // Haskell — Cabal and Stack
                    "dist-newstyle/**".into(),
                    ".stack-work/**".into(),
                    // Ruby
                    ".bundle/**".into(),
                    // Java / Scala — Gradle (cache + build output) and sbt (cache)
                    ".gradle/**".into(),
                    "**/build/**".into(),
                    ".sbt/**".into(),
                    // C# / F# (MSBuild)
                    "**/bin/Debug/**".into(),
                    "**/bin/Release/**".into(),
                    "**/obj/**".into(),
                ],
            },
        }
    }
}

/// Returns the default global config path: ~/.maharajah/maharajah.toml
pub fn global_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".maharajah")
        .join("maharajah.toml")
}

/// Returns the LanceDB directory path for a given target directory.
/// The database lives at <target_dir>/.maharajah/db (a directory, not a file).
pub fn db_path(target_dir: &Path) -> PathBuf {
    target_dir.join(".maharajah").join("db")
}

/// Ensures the global config file exists, creating it with defaults on first launch.
/// Does nothing if the file already exists.
pub fn ensure_global_config(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, DEFAULT_GLOBAL_CONFIG)?;
    Ok(())
}

const DEFAULT_GLOBAL_CONFIG: &str = r#"# maharajah global configuration
# This file was created automatically. Edit as needed.
# Project-level overrides go in maharajah.toml in the project directory.

[embed]
model_id = "nomic-ai/CodeRankEmbed"   # ~550 MB, downloaded from HuggingFace Hub on first run

[db]
table_name = "chunks"
embedding_dim = 768

[index]
max_chunk_lines = 150
default_extensions = ["rs", "py", "js", "cjs", "mjs", "jsx", "ts", "tsx", "go", "java", "cs", "fs", "fsx", "scala", "sc", "hs", "rb"]
default_excludes = [
    "**/target/**",
    "**/node_modules/**",
    "**/__pycache__/**",
    ".venv/**",
    "venv/**",
    "env/**",
    "vendor/**",
    "dist-newstyle/**",
    ".stack-work/**",
    ".bundle/**",
    ".gradle/**",
    "**/build/**",
    ".sbt/**",
    "**/bin/Debug/**",
    "**/bin/Release/**",
    "**/obj/**",
]
"#;

/// Load configuration using figment's layered system:
/// 1. Built-in Rust defaults (AppConfig::default)
/// 2. Global config file (~/.maharajah/maharajah.toml) — silently ignored if missing
/// 3. Project config file (<target-dir>/maharajah.toml) — only merged if Some
/// 4. Environment variables prefixed with MAHARAJAH_ (nested with __)
///    e.g. MAHARAJAH_EMBED__MODEL_ID=nomic-ai/CodeRankEmbed
pub fn load(global_config: &Path, project_config: Option<&Path>) -> Result<AppConfig> {
    let mut figment = Figment::from(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file(global_config));

    if let Some(proj) = project_config {
        figment = figment.merge(Toml::file(proj));
    }

    let config = figment
        .merge(Env::prefixed("MAHARAJAH_").split("__"))
        .extract()?;

    Ok(config)
}
