use std::path::Path;

use serde::Serialize;

use crate::cli::{FindArgs, OutputFormat};
use crate::config::AppConfig;
use crate::db::store::Store;
use crate::embed::unixcoder::UniXcoderEmbedder;
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
    let variant = config.unixcoder.variant.clone();
    let prompt = args.prompt.clone();
    let vector = tokio::task::spawn_blocking(move || {
        UniXcoderEmbedder::load(&variant)?.embed(&prompt)
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
