use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::api::sync::{Api, ApiRepo};
use tokenizers::{
    decoders::byte_level::ByteLevel as ByteLevelDecoder,
    models::bpe::BPE,
    pre_tokenizers::byte_level::ByteLevel,
    AddedToken, Tokenizer,
};

// ─── Model selection ──────────────────────────────────────────────────────────

enum UniXcoderVariant {
    Base,
    Nine,
}

impl UniXcoderVariant {
    fn from_str(s: &str) -> Self {
        match s {
            "nine" | "unixcoder-base-nine" => Self::Nine,
            _ => Self::Base,
        }
    }

    fn repo_id(&self) -> &'static str {
        match self {
            Self::Base => "microsoft/unixcoder-base",
            Self::Nine => "microsoft/unixcoder-base-nine",
        }
    }
}

// ─── Model wrapper (encoder-only) ─────────────────────────────────────────────

struct UniXcoderModel {
    bert: BertModel,
}

impl UniXcoderModel {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        let bert = BertModel::load(vb, config)?;
        Ok(Self { bert })
    }
}

// ─── HuggingFace download helpers ────────────────────────────────────────────

/// Resolve a model file to a local path.
///
/// `ApiRepo::get()` already checks the HF cache first (pure filesystem, no
/// network) and only calls `download()` when the file is absent.  The ureq
/// fallback handles files that the API cannot fetch for any reason.
fn hf_get(repo: &ApiRepo, repo_id: &str, filename: &str) -> Result<std::path::PathBuf> {
    if let Ok(path) = repo.get(filename) {
        return Ok(path);
    }

    // Fallback: direct HTTP GET with a local cache entry.
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

// ─── Tokenizer construction ───────────────────────────────────────────────────

fn build_tokenizer(
    vocab_path: &std::path::Path,
    merges_path: &std::path::Path,
) -> Result<Tokenizer> {
    let bpe = BPE::from_file(
        vocab_path.to_str().unwrap(),
        merges_path.to_str().unwrap(),
    )
    .unk_token("<unk>".to_string())
    .build()
    .map_err(|e| anyhow::anyhow!("BPE build error: {e}"))?;

    let mut tok = Tokenizer::new(bpe);
    tok.with_pre_tokenizer(Some(ByteLevel::default()));
    tok.with_decoder(Some(ByteLevelDecoder::default()));

    tok.add_special_tokens(&[
        AddedToken::from("<s>", true),
        AddedToken::from("<pad>", true),
        AddedToken::from("</s>", true),
        AddedToken::from("<unk>", true),
    ]);

    for token in [
        "<encoder-decoder>",
        "<encoder-only>",
        "<decoder-only>",
        "<mask0>",
        "<mask1>",
        "<mask2>",
    ] {
        if tok.token_to_id(token).is_some() {
            tok.add_special_tokens(&[AddedToken::from(token, true)]);
        }
    }

    Ok(tok)
}

// ─── Tokenization ─────────────────────────────────────────────────────────────

fn tokenize_encoder(tok: &Tokenizer, text: &str, max_len: usize) -> Vec<u32> {
    let mut ids: Vec<u32> = tok
        .encode(text, false)
        .expect("tokenize failed")
        .get_ids()
        .to_vec();

    match tok.token_to_id("<encoder-only>") {
        Some(enc_only_id) => {
            ids.truncate(max_len - 4);
            let mut full = vec![0u32, enc_only_id, 2u32];
            full.extend_from_slice(&ids);
            full.push(2u32);
            full
        }
        None => {
            ids.truncate(max_len - 2);
            let mut full = vec![0u32];
            full.extend_from_slice(&ids);
            full.push(2u32);
            full
        }
    }
}

// ─── Embedding utilities ──────────────────────────────────────────────────────

fn normalize_l2(v: &Tensor) -> Result<Tensor> {
    Ok(v.broadcast_div(&v.sqr()?.sum_all()?.sqrt()?)?)
}

// ─── Public embedder ──────────────────────────────────────────────────────────

pub struct UniXcoderEmbedder {
    model: UniXcoderModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl UniXcoderEmbedder {
    /// Load the model synchronously from the HuggingFace Hub cache.
    ///
    /// The first call downloads ~125 MB; subsequent calls use the local cache
    /// and complete in under a second (mmap'd weights + tokenizer build).
    ///
    /// Call this from `tokio::task::spawn_blocking` — never directly from an
    /// async context, because hf-hub's sync API may perform blocking I/O.
    pub fn load(variant: &str) -> Result<Self> {
        let var = UniXcoderVariant::from_str(variant);
        let model_id = var.repo_id();

        eprintln!("[maharajah] Loading UniXcoder embedder ({model_id})...");
        let device = Device::Cpu;
        let repo = Api::new()?.model(model_id.to_string());

        eprintln!("[maharajah]   resolving config.json");
        let config_path = hf_get(&repo, model_id, "config.json").context("config.json")?;

        eprintln!("[maharajah]   resolving vocab.json");
        let vocab_path = hf_get(&repo, model_id, "vocab.json").context("vocab.json")?;

        eprintln!("[maharajah]   resolving merges.txt");
        let merges_path = hf_get(&repo, model_id, "merges.txt").context("merges.txt")?;

        eprintln!("[maharajah]   resolving model weights");
        let (weights_path, use_safetensors) =
            match hf_get(&repo, model_id, "model.safetensors") {
                Ok(p) => (p, true),
                Err(_) => (
                    hf_get(&repo, model_id, "pytorch_model.bin")
                        .context("pytorch_model.bin")?,
                    false,
                ),
            };

        eprintln!("[maharajah]   building tokenizer");
        let tokenizer = build_tokenizer(&vocab_path, &merges_path)?;

        eprintln!("[maharajah]   loading model weights");
        let config: Config =
            serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;
        let vb = if use_safetensors {
            unsafe {
                VarBuilder::from_mmaped_safetensors(&[&weights_path], DTYPE, &device)?
            }
        } else {
            VarBuilder::from_pth(&weights_path, DTYPE, &device)?
        };
        let model = UniXcoderModel::load(vb, &config)?;

        eprintln!("[maharajah]   ready.");
        Ok(Self { model, tokenizer, device })
    }

    /// Embed `text` using encoder-only mean pooling, returning an L2-normalised
    /// 768-dim vector.  Synchronous; call from `spawn_blocking`.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let ids = tokenize_encoder(&self.tokenizer, text, 512);
        let seq_len = ids.len();
        let input_ids = Tensor::from_vec(
            ids.iter().map(|&x| x as i64).collect(),
            (1, seq_len),
            &self.device,
        )?;
        let token_type_ids = Tensor::zeros((1, seq_len), DType::I64, &self.device)?;
        let hidden = self
            .model
            .bert
            .forward(&input_ids, &token_type_ids, None)?
            .squeeze(0)?
            .mean(0)?;
        let normalized = normalize_l2(&hidden)?;
        Ok(normalized.to_vec1::<f32>()?)
    }
}
