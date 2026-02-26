use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

use crate::cli::{FindArgs, OutputFormat};
use crate::config::AppConfig;
use crate::db::store::{SearchResult, Store};
use crate::embed::nomic::NomicEmbedder;
use crate::error::{AppError, Result};
use crate::indexer;

#[derive(Serialize)]
struct JsonResult {
    rank: usize,
    file_path: String,
    start_line: u32,
    end_line: u32,
    symbol: String,
    score: f32,
    content: String,
    summary: Option<String>,
}

pub async fn find_cmd(
    config: &AppConfig,
    db_path: &Path,
    target_dir: &Path,
    args: FindArgs,
) -> Result<()> {
    // Auto-refresh changed files before searching
    let (refreshed, _) = indexer::refresh(config, db_path, target_dir).await?;
    if refreshed > 0 {
        println!("[auto-refresh: {refreshed} file(s) updated]");
    }

    // Load embedder and embed the query in one spawn_blocking call
    let prompt = args.prompt.clone();
    let vector = tokio::task::spawn_blocking(move || {
        NomicEmbedder::load()?.embed_query(&prompt)
    })
    .await
    .map_err(|e| AppError::Other(e.into()))?
    .map_err(|e| AppError::Embed(e.to_string()))?;

    let store = Store::open_or_create(
        db_path,
        config.db.embedding_dim,
        &config.db.table_name,
        false,
    )
    .await?;

    let results = store.search(&vector, args.limit).await?;

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    match args.format {
        OutputFormat::Text => {
            for (i, r) in results.iter().enumerate() {
                let symbol_display = if r.symbol.is_empty() {
                    String::new()
                } else {
                    format!("  {}", r.symbol)
                };
                println!(
                    "[{}] dist:{:.4}  {}:{}-{}{}",
                    i + 1,
                    r.score,
                    r.file_path,
                    r.start_line,
                    r.end_line,
                    symbol_display
                );
                if let Some(ref s) = r.summary {
                    println!("  summary: {}", s);
                }
                let preview: String = r
                    .content
                    .lines()
                    .take(3)
                    .map(|l| format!("  {}", l))
                    .collect::<Vec<_>>()
                    .join("\n");
                println!("{}", preview);
                println!();
            }
        }
        OutputFormat::Json => {
            let json_results: Vec<JsonResult> = results
                .into_iter()
                .enumerate()
                .map(|(i, r)| JsonResult {
                    rank: i + 1,
                    file_path: r.file_path,
                    start_line: r.start_line,
                    end_line: r.end_line,
                    symbol: r.symbol,
                    score: r.score,
                    content: r.content,
                    summary: r.summary,
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json_results)
                    .map_err(|e| crate::error::AppError::Other(e.into()))?
            );
        }
    }

    Ok(())
}

pub(crate) fn rrf_merge(
    content_results: Vec<SearchResult>,
    summary_results: Vec<SearchResult>,
    limit: usize,
) -> Vec<SearchResult> {
    const K: f32 = 60.0;
    let mut scores: HashMap<String, (SearchResult, f32)> = HashMap::new();

    for (rank, r) in content_results.into_iter().enumerate() {
        let rrf = 1.0 / (K + (rank + 1) as f32);
        scores.entry(r.id.clone()).and_modify(|(_, s)| *s += rrf).or_insert((r, rrf));
    }
    for (rank, r) in summary_results.into_iter().enumerate() {
        let rrf = 1.0 / (K + (rank + 1) as f32);
        scores.entry(r.id.clone()).and_modify(|(_, s)| *s += rrf).or_insert((r, rrf));
    }

    let mut merged: Vec<(SearchResult, f32)> = scores.into_values().collect();
    merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    merged.truncate(limit);
    merged.into_iter().map(|(mut r, rrf_score)| { r.score = rrf_score; r }).collect()
}

pub async fn query_cmd(
    config: &AppConfig,
    db_path: &std::path::Path,
    target_dir: &std::path::Path,
    args: FindArgs,
) -> Result<()> {
    // Auto-refresh changed files before searching
    let (refreshed, _) = indexer::refresh(config, db_path, target_dir).await?;
    if refreshed > 0 {
        println!("[auto-refresh: {refreshed} file(s) updated]");
    }

    // Load embedder and embed the query in one spawn_blocking call
    let prompt = args.prompt.clone();
    let vector = tokio::task::spawn_blocking(move || {
        NomicEmbedder::load()?.embed_query(&prompt)
    })
    .await
    .map_err(|e| AppError::Other(e.into()))?
    .map_err(|e| AppError::Embed(e.to_string()))?;

    let store = Store::open_or_create(
        db_path,
        config.db.embedding_dim,
        &config.db.table_name,
        false,
    )
    .await?;

    let content_results = store.search(&vector, args.limit).await?;
    let summary_results = store.search_by_summary(&vector, args.limit).await?;
    let results = rrf_merge(content_results, summary_results, args.limit);

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    match args.format {
        OutputFormat::Text => {
            for (i, r) in results.iter().enumerate() {
                let symbol_display = if r.symbol.is_empty() {
                    String::new()
                } else {
                    format!("  {}", r.symbol)
                };
                println!(
                    "[{}] rrf:{:.4}  {}:{}-{}{}",
                    i + 1,
                    r.score,
                    r.file_path,
                    r.start_line,
                    r.end_line,
                    symbol_display
                );
                if let Some(ref s) = r.summary {
                    println!("  summary: {}", s);
                }
                let preview: String = r
                    .content
                    .lines()
                    .take(3)
                    .map(|l| format!("  {}", l))
                    .collect::<Vec<_>>()
                    .join("\n");
                println!("{}", preview);
                println!();
            }
        }
        OutputFormat::Json => {
            let json_results: Vec<JsonResult> = results
                .into_iter()
                .enumerate()
                .map(|(i, r)| JsonResult {
                    rank: i + 1,
                    file_path: r.file_path,
                    start_line: r.start_line,
                    end_line: r.end_line,
                    symbol: r.symbol,
                    score: r.score,
                    content: r.content,
                    summary: r.summary,
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json_results)
                    .map_err(|e| crate::error::AppError::Other(e.into()))?
            );
        }
    }

    Ok(())
}
