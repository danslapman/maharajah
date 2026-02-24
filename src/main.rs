mod cli;
mod config;
mod db;
mod embed;
mod error;
mod indexer;
mod rag;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, DbAction};
use db::store::Store;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    fmt().with_env_filter(EnvFilter::new(filter)).init();

    // 1. Resolve target directory
    let target_dir = cli
        .target_dir
        .unwrap_or_else(|| std::env::current_dir().expect("cannot read current directory"));

    // 2. Resolve global config path (overridable via --config)
    let global_cfg_path = cli.config.unwrap_or_else(config::global_config_path);

    // 3. Auto-create global config on first launch
    config::ensure_global_config(&global_cfg_path)?;

    // 4. Probe for project config in target dir (optional, never auto-created)
    let project_cfg_path = target_dir.join("maharajah.toml");
    let project_cfg = project_cfg_path.exists().then_some(project_cfg_path.as_path());

    // 5. Load layered config
    let cfg = config::load(&global_cfg_path, project_cfg)?;

    // 6. Compute DB path from target dir
    let db_path = config::db_path(&target_dir);

    match cli.command {
        Commands::Index(args) => {
            indexer::run(&cfg, &db_path, &target_dir, args).await?;
        }
        Commands::Find(args) => {
            rag::retriever::find_cmd(&cfg, &db_path, &target_dir, args).await?;
        }
        Commands::Db(args) => {
            match args.action {
                DbAction::Stats => {
                    match Store::try_open(&db_path, cfg.db.embedding_dim, &cfg.db.table_name)
                        .await?
                    {
                        None => println!("No index found. Run `index` first."),
                        Some(store) => {
                            let chunks = store.count_rows().await?;
                            let files = store.count_files().await?;
                            println!("Files indexed : {files}");
                            println!("Total chunks  : {chunks}");
                            println!("Embedding dim : {}", cfg.db.embedding_dim);
                        }
                    }
                }
                DbAction::Clear { yes } => {
                    if !yes {
                        println!("Pass --yes to confirm clearing all indexed data.");
                    } else {
                        match Store::try_open(&db_path, cfg.db.embedding_dim, &cfg.db.table_name)
                            .await?
                        {
                            None => println!("No index found. Nothing to clear."),
                            Some(store) => {
                                store.clear().await?;
                                println!("Index cleared.");
                            }
                        }
                    }
                }
            }
        }
        Commands::Config => {
            println!("{}", serde_json::to_string_pretty(&cfg)?);
        }
    }

    Ok(())
}
