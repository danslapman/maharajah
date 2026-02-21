use ollama_rs::{generation::embeddings::request::GenerateEmbeddingsRequest, Ollama};

use crate::config::OllamaConfig;
use crate::error::{AppError, Result};

pub struct OllamaEmbedder {
    client: Ollama,
    config: OllamaConfig,
}

impl OllamaEmbedder {
    pub fn new(config: OllamaConfig) -> Result<Self> {
        let client = Ollama::try_new(&config.base_url)
            .map_err(|e| AppError::Embed(e.to_string()))?;
        Ok(Self { client, config })
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let req = GenerateEmbeddingsRequest::new(self.config.embed_model.clone(), text.into());
        let resp = self.client
            .generate_embeddings(req)
            .await
            .map_err(|e| AppError::Embed(e.to_string()))?;
        let vector: Vec<f32> = resp.embeddings[0].iter().map(|&x| x as f32).collect();
        Ok(vector)
    }
}
