use anyhow::{Context, Result};
use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::nomic_bert::{Config, NomicBertModel};
use hf_hub::api::sync::{Api, ApiRepo};
use tokenizers::Tokenizer;

const QUERY_PREFIX: &str = "Represent this query for searching relevant code: ";
const MAX_LEN: usize = 8192;
const MODEL_ID: &str = "nomic-ai/CodeRankEmbed";

// ─── HuggingFace download helper ─────────────────────────────────────────────

fn hf_get(repo: &ApiRepo, repo_id: &str, filename: &str) -> Result<std::path::PathBuf> {
    if let Ok(path) = repo.get(filename) {
        return Ok(path);
    }

    let url = format!("https://huggingface.co/{repo_id}/resolve/main/{filename}");
    let cache_dir = hf_hub::Cache::default().path().join("http-fallback");
    let dest = cache_dir.join(format!(
        "{}-{}",
        repo_id.replace('/', "-"),
        filename.replace('/', "_")
    ));

    if !dest.exists() {
        std::fs::create_dir_all(&cache_dir)?;
        let response = ureq::get(&url)
            .call()
            .with_context(|| format!("HTTP GET {url}"))?;
        let mut file = std::fs::File::create(&dest)?;
        std::io::copy(&mut response.into_reader(), &mut file)?;
    }
    Ok(dest)
}

// ─── Tokenization ─────────────────────────────────────────────────────────────

fn tokenize(tok: &Tokenizer, text: &str) -> (Vec<i64>, Vec<i64>) {
    let encoding = tok.encode(text, true).expect("tokenize failed");
    let ids: Vec<i64> = encoding
        .get_ids()
        .iter()
        .take(MAX_LEN)
        .map(|&x| x as i64)
        .collect();
    let mask: Vec<i64> = encoding
        .get_attention_mask()
        .iter()
        .take(MAX_LEN)
        .map(|&x| x as i64)
        .collect();
    (ids, mask)
}

// ─── Embedding utility ────────────────────────────────────────────────────────

fn cls_pool_and_normalize(hidden: &Tensor) -> Result<Vec<f32>> {
    // hidden shape: (batch=1, seq_len, n_embd) — take CLS token at position 0
    let cls = hidden.i((.., 0usize, ..))?;
    // l2-normalize
    let norm = cls.broadcast_div(&cls.sqr()?.sum_all()?.sqrt()?)?;
    Ok(norm.squeeze(0)?.to_vec1::<f32>()?)
}

// ─── Public embedder ──────────────────────────────────────────────────────────

pub struct NomicEmbedder {
    model: NomicBertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl NomicEmbedder {
    /// Load CodeRankEmbed from the HuggingFace Hub cache.
    /// Synchronous — call from `tokio::task::spawn_blocking`.
    pub fn load() -> Result<Self> {
        tracing::info!("Loading NomicEmbedder ({MODEL_ID})...");
        let device = Device::Cpu;
        let repo = Api::new()?.model(MODEL_ID.to_string());

        tracing::info!("  resolving config.json");
        let config_path = hf_get(&repo, MODEL_ID, "config.json").context("config.json")?;

        tracing::info!("  resolving tokenizer.json");
        let tokenizer_path =
            hf_get(&repo, MODEL_ID, "tokenizer.json").context("tokenizer.json")?;

        tracing::info!("  resolving model weights");
        let weights_path = match hf_get(&repo, MODEL_ID, "model.safetensors") {
            Ok(p) => p,
            Err(_) => hf_get(&repo, MODEL_ID, "pytorch_model.bin").context("pytorch_model.bin")?,
        };

        tracing::info!("  building tokenizer");
        let tokenizer =
            Tokenizer::from_file(&tokenizer_path).map_err(|e| anyhow::anyhow!("{e}"))?;

        tracing::info!("  loading model weights");
        let config: Config =
            serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[&weights_path], DType::F32, &device)?
        };
        let model = NomicBertModel::load(vb, &config)?;

        tracing::info!("  ready.");
        Ok(Self { model, tokenizer, device })
    }

    /// Embed a code snippet. No prefix is prepended.
    pub fn embed_code(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_raw(text)
    }

    /// Embed a natural-language query. Prepends the required task instruction.
    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        let prefixed = format!("{QUERY_PREFIX}{query}");
        self.embed_raw(&prefixed)
    }

    fn embed_raw(&self, text: &str) -> Result<Vec<f32>> {
        let (ids, mask) = tokenize(&self.tokenizer, text);
        let seq_len = ids.len();

        let input_ids = Tensor::from_vec(ids, (1, seq_len), &self.device)?;
        let attention_mask = Tensor::from_vec(mask, (1, seq_len), &self.device)?;
        let token_type_ids = Tensor::zeros((1, seq_len), DType::I64, &self.device)?;

        let hidden = self
            .model
            .forward(&input_ids, Some(&token_type_ids), Some(&attention_mask))?;

        cls_pool_and_normalize(&hidden)
    }
}
