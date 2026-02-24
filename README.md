# maharajah

A local semantic code search engine.

## Name

Named after [Maharajah and the Sepoys](https://en.wikipedia.org/wiki/Maharajah_and_the_Sepoys) — a chess variant where a single, all-powerful Maharajah (moving as queen + knight) faces a full army. The name is also a word play: maha-**RAG**-ah.

## Features

- **No external services required** — embeddings run in-process via [Candle](https://github.com/huggingface/candle); no Ollama, no API keys
- **CodeRankEmbed embeddings** — Nomic's code retrieval model ([`nomic-ai/CodeRankEmbed`](https://huggingface.co/nomic-ai/CodeRankEmbed), 137M parameters, 768-dim, up to 8192 tokens), downloaded from HuggingFace Hub on first run (~550 MB, cached locally)
- Tree-sitter AST-aware chunking for Rust, Python, JavaScript/JSX, TypeScript/TSX, Go, Java, C#, F#, Scala, Haskell, and Ruby
- **Pre-computed summaries** from doc comments and docstrings extracted by tree-sitter at index time — shown alongside search results
- Incremental indexing: SHA-256 hash checks mean only changed files are re-embedded; deleted files are automatically removed from the index
- Auto-refresh on `find` and `query` — index stays current without a manual `index` step
- Vector store powered by [LanceDB](https://lancedb.com) (embedded, no server required)
- Build-artifact directories excluded by default (`target/`, `node_modules/`, `build/`, etc.) — configurable per project

## Prerequisites

- Rust toolchain (stable)

That's it. Model weights are downloaded automatically on first use.

## Installation

```sh
# Build in release mode (strongly recommended — debug builds are ~10x slower for inference)
cargo build --release

# Or install to ~/.cargo/bin
cargo install --path .
```

> **Note:** Always use a release build for practical use. Debug builds link unoptimized Candle kernels and make even small indexes take many times longer to embed.

## Quick start

```sh
# ── From inside your project directory ────────────────────────────────────────
cd /path/to/project

# Index the project (-D is optional when you're already inside it)
maharajah index

# Semantic search — returns ranked code chunks with summaries (content vectors only)
maharajah find "database connection pooling"

# Semantic search with RRF fusion of content + summary vectors
maharajah query "database connection pooling"

# ── Or pass the directory explicitly (useful in scripts / CI) ─────────────────
maharajah -D /path/to/project index
maharajah -D /path/to/project find "database connection pooling"
```

On first run `index` will print progress as it downloads and loads the model:

```
Loading NomicEmbedder (nomic-ai/CodeRankEmbed)...
  resolving config.json
  resolving tokenizer.json
  resolving model weights
  building tokenizer
  loading model weights
  ready.
```

Subsequent runs load directly from the HuggingFace cache (pure filesystem, no network).

## Commands

| Command | Description |
|---|---|
| `index` | Walk the project, embed changed files, store in LanceDB; purge chunks for deleted files |
| `find <prompt>` | Embed prompt → vector search over code chunks → display ranked results with summaries |
| `query <prompt>` | Like `find`, but fuses content and summary vector searches with Reciprocal Rank Fusion (RRF) |
| `db stats` | Show files indexed, chunk count, embedding dimension |
| `db clear --yes` | Delete all indexed data |
| `config` | Print resolved configuration as JSON |

### Common flags

| Flag | Description |
|---|---|
| `-D <dir>` | Project directory to index/query (default: current directory) |
| `-k <n>` | Number of chunks to retrieve (default: 5) |
| `--format <fmt>` | Output format (`text` or `json`) |
| `--reindex` | Force re-embedding of all files, ignoring cached hashes |
| `-i <glob>` | Include only files matching this glob (repeatable) |
| `-x <glob>` | Exclude files matching this glob, in addition to the built-in defaults (repeatable) |

## Configuration

### Default excludes

maharajah automatically skips directories that contain build artifacts, not source code. The built-in list covers all supported languages — large trees like `target/` or `node_modules/` are never walked at all.

To add project-specific excludes on top of the defaults, use `-x` on the command line or `default_excludes` in `maharajah.toml`. To override the list entirely, replace `default_excludes` in your global `~/.maharajah/maharajah.toml`.

---

maharajah uses a three-layer configuration model:

1. **Global defaults** (built in) — sensible values that work out of the box
2. **Global config** (`~/.maharajah/maharajah.toml`) — auto-created on first run; edit to change defaults for all projects
3. **Project config** (`<project-dir>/maharajah.toml`) — optional, never auto-created; overrides the global config for a specific project
4. **Environment variables** — highest priority; prefixed `MAHARAJAH_`, nested with `__`
   (e.g. `MAHARAJAH_EMBED__MODEL_ID=nomic-ai/CodeRankEmbed`)

### Example `maharajah.toml`

```toml
[embed]
# HuggingFace model ID to use for embeddings.
# ~550 MB, downloaded from HuggingFace Hub on first use.
model_id = "nomic-ai/CodeRankEmbed"

[db]
table_name = "chunks"
embedding_dim = 768

[index]
# Maximum lines per chunk. With CodeRankEmbed's 8192-token context, larger
# chunks are viable — 150 lines covers most function definitions comfortably.
max_chunk_lines = 150
default_extensions = ["rs", "py", "ts", "tsx", "js", "jsx", "go", "java", "cs", "fs", "fsx", "scala", "hs", "rb"]
# Glob patterns excluded during indexing (merged with any -x flags passed on the CLI).
# This list replaces the built-in defaults when set here.
default_excludes = [
    "**/target/**",
    "**/node_modules/**",
    "**/__pycache__/**",
    ".venv/**",
    "venv/**",
    "env/**",
    "vendor/**",
    "dist-newstyle/**",
    ".stack-work/**",
    ".bundle/**",
    ".gradle/**",
    "**/build/**",
    ".sbt/**",
    "**/bin/Debug/**",
    "**/bin/Release/**",
    "**/obj/**",
]
```

### Schema migration

If you have an existing index that needs to be rebuilt, run:

```sh
maharajah index --reindex
```

This wipes the index and rebuilds all embeddings from scratch. Required any time you change the embedding model, since vectors from different models are not comparable.
