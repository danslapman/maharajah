use anyhow::Result;
use tokio::sync::{mpsc, oneshot};

use crate::embed::nomic::NomicEmbedder;

pub type EmbedRequest = (String, oneshot::Sender<Result<Vec<f32>>>);

/// Spawn a dedicated OS thread that owns the `NomicEmbedder`.
/// Returns a sender that callers use to embed queries.
/// Each request is `(query_string, oneshot::Sender<Result<Vec<f32>>>)`.
pub fn spawn_embedder_actor() -> mpsc::Sender<EmbedRequest> {
    let (tx, mut rx) = mpsc::channel::<EmbedRequest>(32);

    std::thread::spawn(move || {
        let embedder = match NomicEmbedder::load() {
            Ok(e) => e,
            Err(err) => {
                tracing::error!("Embedder failed to load: {err}");
                return;
            }
        };

        // Drive the receiver on a single-threaded runtime so we can use async recv
        // without moving the embedder across threads.
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("embedder actor runtime");

        rt.block_on(async move {
            while let Some((query, reply_tx)) = rx.recv().await {
                let result = embedder.embed_query(&query);
                let _ = reply_tx.send(result);
            }
        });
    });

    tx
}
