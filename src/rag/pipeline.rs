use std::path::Path;

use futures::StreamExt;
use ollama_rs::{Ollama, generation::completion::request::GenerationRequest};

use crate::cli::QueryArgs;
use crate::config::AppConfig;
use crate::db::store::Store;
use crate::embed::ollama::OllamaEmbedder;
use crate::error::{AppError, Result};
use crate::indexer;

pub async fn run(config: &AppConfig, db_path: &Path, target_dir: &Path, args: QueryArgs) -> Result<()> {
    // Auto-refresh changed files before querying
    let (refreshed, _) = indexer::refresh(config, db_path, target_dir).await?;
    if refreshed > 0 {
        println!("[auto-refresh: {refreshed} file(s) updated]");
    }

    // 1. Embed the question
    let embedder = OllamaEmbedder::new(config.ollama.clone())?;
    let vector = embedder.embed(&args.question).await?;

    // 2. Retrieve relevant chunks
    let store = Store::open_or_create(
        db_path, config.db.embedding_dim, &config.db.table_name, false,
    ).await?;
    let results = store.search(&vector, args.top_k).await?;

    if results.is_empty() {
        println!("No relevant code found for this question.");
        return Ok(());
    }

    // 3. Optionally print context
    if args.show_context {
        println!("## Retrieved Context\n");
        for (i, r) in results.iter().enumerate() {
            let sym = if r.symbol.is_empty() { String::new() } else { format!(" — {}", r.symbol) };
            println!("### [{}] {}:{}-{}{}", i + 1, r.file_path, r.start_line, r.end_line, sym);
            println!("```\n{}\n```\n", r.content);
        }
    }

    // 4. Build prompt
    let context = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let sym = if r.symbol.is_empty() { String::new() } else { format!(" — {}", r.symbol) };
            format!("### [{}] {}:{}-{}{}\n```\n{}\n```", i + 1, r.file_path, r.start_line, r.end_line, sym, r.content)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let prompt = format!(
        "You are a code expert assistant. Answer the question about the codebase \
         using the provided code snippets as context. If the snippets do not contain \
         enough information, say so.\n\n\
         ## Code Context\n\n{context}\n\n\
         ## Question\n\n{}\n\n\
         ## Answer\n\n",
        args.question
    );

    // 5. Generate answer
    let model = args.model.unwrap_or_else(|| config.ollama.generate_model.clone());
    let ollama = Ollama::try_new(&config.ollama.base_url)
        .map_err(|e| AppError::Llm(e.to_string()))?;
    let req = GenerationRequest::new(model, prompt);

    use std::io::Write;
    let mut stream = ollama
        .generate_stream(req)
        .await
        .map_err(|e| AppError::Llm(e.to_string()))?;
    while let Some(result) = stream.next().await {
        let responses = result.map_err(|e| AppError::Llm(e.to_string()))?;
        for r in responses {
            print!("{}", r.response);
            std::io::stdout().flush().ok();
        }
    }
    println!();

    Ok(())
}
