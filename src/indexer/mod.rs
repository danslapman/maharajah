pub mod chunker;
pub mod parser;
pub mod walker;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::cli::IndexArgs;
use crate::config::AppConfig;
use crate::db::store::{ChunkRecord, Store};
use crate::embed::unixcoder::UniXcoderEmbedder;
use crate::error::{AppError, Result};

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

    let variant = config.unixcoder.variant.clone();
    let embedder = Arc::new(
        tokio::task::spawn_blocking(move || UniXcoderEmbedder::load(&variant))
            .await
            .map_err(|e| AppError::Other(e.into()))?
            .map_err(|e| AppError::Embed(e.to_string()))?,
    );

    let mut exclude = args.exclude.clone();
    exclude.extend_from_slice(&config.index.default_excludes);
    let files = walker::collect_files(
        target_dir,
        &args.include,
        &exclude,
        &config.index.default_extensions,
    );

    let total = files.len();
    let (indexed, skipped) = index_files(
        &store,
        embedder,
        target_dir,
        &files,
        args.reindex,
        config.index.max_chunk_lines,
    )
    .await?;

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

    let variant = config.unixcoder.variant.clone();
    let embedder = Arc::new(
        tokio::task::spawn_blocking(move || UniXcoderEmbedder::load(&variant))
            .await
            .map_err(|e| AppError::Other(e.into()))?
            .map_err(|e| AppError::Embed(e.to_string()))?,
    );

    let files = walker::collect_files(
        target_dir,
        &[],
        &config.index.default_excludes,
        &config.index.default_extensions,
    );

    index_files(&store, embedder, target_dir, &files, false, config.index.max_chunk_lines).await
}

async fn index_files(
    store: &Store,
    embedder: Arc<UniXcoderEmbedder>,
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
        if chunks.is_empty() {
            skipped += 1;
            continue;
        }

        // Embed all chunks for this file in one spawn_blocking call
        let emb = Arc::clone(&embedder);
        let contents: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let summaries: Vec<Option<String>> = chunks.iter().map(|c| c.summary.clone()).collect();

        let (vectors, summary_vectors): (Vec<Option<Vec<f32>>>, Vec<Option<Vec<f32>>>) =
            tokio::task::spawn_blocking(move || {
                let vecs: Vec<Option<Vec<f32>>> =
                    contents.iter().map(|c| emb.embed(c).ok()).collect();
                let svecs: Vec<Option<Vec<f32>>> = summaries
                    .iter()
                    .map(|s| s.as_deref().and_then(|text| emb.embed(text).ok()))
                    .collect();
                (vecs, svecs)
            })
            .await
            .map_err(|e| AppError::Other(e.into()))?;

        let mut records = Vec::with_capacity(chunks.len());
        for ((chunk, vector_opt), summary_vector) in
            chunks.into_iter().zip(vectors).zip(summary_vectors)
        {
            let vector = match vector_opt {
                Some(v) => v,
                None => {
                    eprintln!("Warning: embed failed for {}", rel_path);
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
                summary: chunk.summary,
                summary_vector,
            });
        }

        store.insert(&records).await?;
        tracing::info!("indexed: {rel_path} ({} chunks)", records.len());
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
