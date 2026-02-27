use actix_web::{HttpResponse, Responder, web};
use tokio::sync::oneshot;

use crate::db::store::Store;
use crate::rag::retriever::rrf_merge;
use crate::server::AppState;

#[derive(serde::Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub min_score: Option<f32>,
}

fn default_limit() -> usize {
    10
}

/// Embed `query` via the actor and open the store, returning both or an error response.
async fn prepare(
    state: &AppState,
    query: &str,
) -> Result<(Vec<f32>, Store), HttpResponse> {
    let (reply_tx, reply_rx) = oneshot::channel();
    if state.embed_tx.send((query.to_owned(), reply_tx)).await.is_err() {
        return Err(HttpResponse::InternalServerError().body("Embedder not available"));
    }
    let vector = match reply_rx.await {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => return Err(HttpResponse::InternalServerError().body(e.to_string())),
        Err(_) => return Err(HttpResponse::InternalServerError().body("Embedder channel closed")),
    };

    let store = Store::open_or_create(
        &state.db_path,
        state.config.db.embedding_dim,
        &state.config.db.table_name,
        false,
    )
    .await
    .map_err(|e| HttpResponse::InternalServerError().body(e.to_string()))?;

    Ok((vector, store))
}

pub async fn find_handler(
    state: web::Data<AppState>,
    body: web::Json<SearchRequest>,
) -> impl Responder {
    let (vector, store) = match prepare(&state, &body.query).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    match store.search(&vector, body.limit).await {
        Ok(results) => {
            let filtered: Vec<_> = results.into_iter()
                .filter(|r| body.min_score.map_or(true, |t| r.score >= t))
                .collect();
            HttpResponse::Ok().json(filtered)
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

pub async fn query_handler(
    state: web::Data<AppState>,
    body: web::Json<SearchRequest>,
) -> impl Responder {
    let (vector, store) = match prepare(&state, &body.query).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    let limit = body.limit;
    let (content_res, summary_res) =
        tokio::join!(store.search(&vector, limit), store.search_by_summary(&vector, limit));

    match (content_res, summary_res) {
        (Ok(content), Ok(summary)) => {
            let merged = rrf_merge(content, summary, limit);
            let filtered: Vec<_> = merged.into_iter()
                .filter(|r| body.min_score.map_or(true, |t| r.score >= t))
                .collect();
            HttpResponse::Ok().json(filtered)
        }
        (Err(e), _) | (_, Err(e)) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
