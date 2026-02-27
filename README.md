# maharajah

A local semantic code search engine.

## Name

Named after [Maharajah and the Sepoys](https://en.wikipedia.org/wiki/Maharajah_and_the_Sepoys) — a chess variant where a single, all-powerful Maharajah (moving as queen + knight) faces a full army. The name is also a word play: maha-**RAG**-ah.

## Features

- **No external services required** — embeddings run in-process; no Ollama, no API keys
- **CodeRankEmbed embeddings** — Nomic's code retrieval model ([`nomic-ai/CodeRankEmbed`](https://huggingface.co/nomic-ai/CodeRankEmbed)), downloaded from HuggingFace Hub on first run (~550 MB, cached locally)
- AST-aware chunking for Rust, Python, JavaScript/JSX, TypeScript/TSX, Go, Java, C#, F#, Scala, Haskell, and Ruby
- **Pre-computed summaries** from doc comments and docstrings, extracted at index time — shown alongside search results
- Incremental indexing: only changed files are re-embedded; deleted files are automatically removed from the index
- Auto-refresh on `find` and `query` — index stays current without a manual `index` step
- **HTTP server mode** — expose `/find` and `/query` over HTTP with automatic background re-indexing on file changes
- Embedded vector store — no external database required
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

> **Note:** Always use a release build for practical use. Debug builds are significantly slower and make even small indexes take much longer to embed.

## Quick start

```sh
# ── From inside your project directory ────────────────────────────────────────
cd /path/to/project

# Index the project (-D is optional when you're already inside it)
mh index

# Semantic search — returns ranked code chunks with summaries
mh find "database connection pooling"

# Semantic search with fusion of content and summary results
mh query "database connection pooling"

# ── Or pass the directory explicitly (useful in scripts / CI) ─────────────────
mh -D /path/to/project index
mh -D /path/to/project find "database connection pooling"
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
| `index` | Walk the project, embed changed files, update the index; purge chunks for deleted files |
| `find <prompt>` | Search for relevant code chunks and display ranked results with summaries |
| `query <prompt>` | Like `find`, but also searches over summaries and merges the results |
| `db stats` | Show files indexed, chunk count, embedding dimension |
| `db clear --yes` | Delete all indexed data |
| `server` | Start an HTTP server exposing `/find` and `/query` endpoints |
| `config` | Print resolved configuration as JSON |

### Common flags

| Flag | Description |
|---|---|
| `-D <dir>` | Project directory to index/query (default: current directory) |
| `-c <file>` | Path to a TOML config file (default: `~/.maharajah/maharajah.toml`) |
| `-n <n>` | Number of chunks to retrieve (default: 10) |
| `--min-score <f>` | Only return results with `score >= f`; omit to return all results |
| `--format <fmt>` | Output format (`text` or `json`) |

### JSON output

`--format json` emits a JSON array, one object per result:

```json
[
  {
    "rank": 1,
    "file_path": "src/collections.rs",
    "start_line": 12,
    "end_line": 45,
    "symbol": "Stack",
    "score": 0.2103,
    "summary": "A simple stack backed by a Vec.",
    "content": "pub struct Stack<T> {\n    data: Vec<T>,\n}\n\nimpl<T> Stack<T> {\n    pub fn new() -> Self { ... }"
  },
  {
    "rank": 2,
    "file_path": "src/collections.rs",
    "start_line": 48,
    "end_line": 61,
    "symbol": "Queue",
    "score": 0.3847,
    "summary": null,
    "content": "pub struct Queue<T> {\n    data: VecDeque<T>,\n}"
  }
]
```

Fields: `rank` (1-based position), `file_path` (relative to the indexed directory), `start_line`/`end_line` (1-based), `symbol` (function or type name, empty string if unavailable), `score` (Euclidean distance for `find` — lower is more similar; RRF score for `query` — higher is better), `summary` (extracted doc comment, or `null`).

### Server mode

`mh server` starts an HTTP API server backed by the same local index.

```sh
# Start on the default address (127.0.0.1:8080)
mh -D /path/to/project server

# Bind to a different host/port
mh -D /path/to/project server --host 0.0.0.0 --port 9090
```

The model is loaded once at startup. While the server is running it watches the project directory for file changes and re-indexes modified files in the background — no manual `index` step needed.

#### `POST /find`

Searches using content vectors only (equivalent to `mh find`).

```sh
curl -X POST http://localhost:8080/find \
  -H 'Content-Type: application/json' \
  -d '{"query": "database connection pooling", "limit": 5}'
```

#### `POST /query`

Searches using both content and summary vectors, merged with Reciprocal Rank Fusion (equivalent to `mh query`).

```sh
curl -X POST http://localhost:8080/query \
  -H 'Content-Type: application/json' \
  -d '{"query": "database connection pooling"}'
```

Both endpoints accept `query` (required), `limit` (optional, default `10`), and `min_score` (optional — only results with `score >= min_score` are returned). They return a JSON array in the same shape as `--format json`, minus the `rank` field.

#### `server`-only flags

| Flag | Description |
|---|---|
| `--host <addr>` | Address to bind to (default: `127.0.0.1`) |
| `--port <port>` | Port to listen on (default: `8080`) |

### `index`-only flags

| Flag | Description |
|---|---|
| `--reindex` | Force re-embedding of all files |
| `-i <glob>` | Include only files matching this glob (repeatable) |
| `-x <glob>` | Exclude files matching this glob, in addition to the built-in defaults (repeatable) |

## Configuration

### Default excludes

mh automatically skips directories that contain build artifacts, not source code. The built-in list covers all supported languages — large trees like `target/` or `node_modules/` are never walked at all.

To add project-specific excludes on top of the defaults, use `-x` on the command line or `default_excludes` in `maharajah.toml`. To override the list entirely, replace `default_excludes` in your global `~/.maharajah/maharajah.toml`.

---

mh uses a three-layer configuration model:

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
# Maximum lines per chunk.
max_chunk_lines = 150
default_extensions = ["rs", "py", "js", "cjs", "mjs", "jsx", "ts", "tsx", "go", "java", "cs", "fs", "fsx", "scala", "sc", "hs", "rb"]
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
mh index --reindex
```

This wipes the index and rebuilds all embeddings from scratch. Required any time you change the embedding model, since vectors from different models are not comparable.
