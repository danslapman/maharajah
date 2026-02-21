pub mod chunker;
pub mod parser;
pub mod walker;

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::cli::IndexArgs;
use crate::config::AppConfig;
use crate::db::store::{ChunkRecord, Store};
use crate::embed::ollama::OllamaEmbedder;
use crate::error::Result;

pub async fn run(
    config: &AppConfig,
    db_path: &Path,
    target_dir: &Path,
    args: IndexArgs,
) -> Result<()> {
    let store = Store::open_or_create(
        db_path,
        config.db.embedding_dim,
        &config.db.table_name,
        args.reindex,
    )
    .await?;

    let embedder = OllamaEmbedder::new(config.ollama.clone())?;

    let mut exclude = args.exclude.clone();
    exclude.extend_from_slice(&config.index.default_excludes);
    let files = walker::collect_files(
        target_dir,
        &args.include,
        &exclude,
        &config.index.default_extensions,
    );

    let total = files.len();
    let (indexed, skipped) =
        index_files(&store, &embedder, target_dir, &files, args.reindex, config.index.max_chunk_lines).await?;

    println!(
        "Done. {total} files found: {indexed} indexed, {skipped} skipped (unchanged or binary)."
    );

    Ok(())
}

/// Incrementally refresh the index for the target directory using default settings.
/// Returns (files_indexed, files_skipped).
pub async fn refresh(
    config: &AppConfig,
    db_path: &Path,
    target_dir: &Path,
) -> Result<(usize, usize)> {
    let store = Store::open_or_create(
        db_path,
        config.db.embedding_dim,
        &config.db.table_name,
        false,
    )
    .await?;

    let embedder = OllamaEmbedder::new(config.ollama.clone())?;

    let files = walker::collect_files(
        target_dir,
        &[],
        &config.index.default_excludes,
        &config.index.default_extensions,
    );

    index_files(&store, &embedder, target_dir, &files, false, config.index.max_chunk_lines).await
}

async fn index_files(
    store: &Store,
    embedder: &OllamaEmbedder,
    target_dir: &Path,
    files: &[PathBuf],
    reindex: bool,
    max_chunk_lines: usize,
) -> Result<(usize, usize)> {
    let mut indexed = 0usize;
    let mut skipped = 0usize;

    for path in files {
        let file_bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Warning: could not read {}: {}", path.display(), e);
                continue;
            }
        };

        let current_hash = compute_hash(&file_bytes);

        // Use path relative to target_dir as the stored key
        let rel_path = path
            .strip_prefix(target_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();

        if !reindex {
            if let Some(stored_hash) = store.get_file_hash(&rel_path).await? {
                if stored_hash == current_hash {
                    skipped += 1;
                    continue;
                }
                // Hash changed — remove stale chunks
                store.delete_file(&rel_path).await?;
            }
        }

        let content = match String::from_utf8(file_bytes) {
            Ok(s) => s,
            Err(_) => {
                // Skip binary files
                skipped += 1;
                continue;
            }
        };

        let chunks = parser::parse_file(path, &content, max_chunk_lines);

        let mut records = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            let vector = match embedder.embed(&chunk.content).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Warning: embed failed for {}: {}", rel_path, e);
                    continue;
                }
            };

            records.push(ChunkRecord {
                id: format!("{}:{}", rel_path, chunk.start_line),
                file_path: rel_path.clone(),
                file_hash: current_hash.clone(),
                language: chunk.language,
                symbol: chunk.symbol,
                content: chunk.content,
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                vector,
            });
        }

        store.insert(&records).await?;
        indexed += 1;
    }

    Ok((indexed, skipped))
}

fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}
