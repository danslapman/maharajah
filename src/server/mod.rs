pub mod embedder_actor;
pub mod handlers;
pub mod watcher;

use std::path::PathBuf;

use actix_web::{App, HttpServer, web};
use tokio::sync::mpsc;

use crate::cli::ServerArgs;
use crate::config::AppConfig;
use embedder_actor::EmbedRequest;

#[derive(Clone)]
pub struct AppState {
    pub embed_tx: mpsc::Sender<EmbedRequest>,
    pub db_path: PathBuf,
    pub config: AppConfig,
}

pub async fn run_server(
    args: ServerArgs,
    config: AppConfig,
    db_path: PathBuf,
    target_dir: PathBuf,
) -> anyhow::Result<()> {
    let bind_addr = format!("{}:{}", args.host, args.port);
    tracing::info!("Starting server on {bind_addr}");

    tracing::info!("Loading embedder model...");
    let embed_tx = embedder_actor::spawn_embedder_actor();

    let _watcher = watcher::spawn_watcher(target_dir, db_path.clone(), config.clone())?;

    let state = AppState { embed_tx, db_path, config };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .route("/find", web::post().to(handlers::find_handler))
            .route("/query", web::post().to(handlers::query_handler))
    })
    .bind(&bind_addr)?
    .run()
    .await?;

    Ok(())
}
