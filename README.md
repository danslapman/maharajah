# maharajah

A local RAG (Retrieval-Augmented Generation) engine for source code.

## Name

Named after [Maharajah and the Sepoys](https://en.wikipedia.org/wiki/Maharajah_and_the_Sepoys) — a chess variant where a single, all-powerful Maharajah (moving as queen + knight) faces a full army. The name is also a word play: maha-**RAG**-ah.

## Features

- Runs entirely locally via [Ollama](https://ollama.com) — no external API keys required
- Incremental indexing: SHA-256 hash checks mean only changed files are re-embedded
- Tree-sitter AST-aware chunking for 11 languages
- Auto-refresh on `find`/`query` — index stays current without a manual `index` step
- Vector store powered by [LanceDB](https://lancedb.com) (embedded, no server required)
- Build-artifact directories excluded by default (`target/`, `node_modules/`, `build/`, etc.) — configurable per project

## Prerequisites

- Rust toolchain (stable)
- Ollama running locally (`http://localhost:11434`)
- Required models pulled:
  ```sh
  ollama pull nomic-embed-text
  ollama pull llama3.2
  ```

## Installation

```sh
# From source
cargo install --path .
```

## Quick start

```sh
# ── From inside your project directory ────────────────────────────────────────
cd /path/to/project

# Index the project (-D is optional when you're already inside it)
maharajah index

# Ask a question
maharajah query "How does authentication work?"

# Semantic search (no LLM generation)
maharajah find "database connection pooling"

# ── Or pass the directory explicitly (useful in scripts / CI) ─────────────────
maharajah -D /path/to/project index
maharajah -D /path/to/project query "How does authentication work?"
```

## Commands

| Command | Description |
|---|---|
| `index` | Walk the project, embed changed files, store in LanceDB |
| `query <question>` | Full RAG pipeline: embed → retrieve → stream answer |
| `find <prompt>` | Semantic search only; returns ranked code chunks |
| `db stats` | Show files indexed, chunk count, embedding dimension |
| `db clear --yes` | Delete all indexed data |
| `config` | Print resolved configuration as JSON |

### Common flags

| Flag | Description |
|---|---|
| `-D <dir>` | Project directory to index/query (default: current directory) |
| `-k <n>` | Number of chunks to retrieve for context (default: 5) |
| `--model <name>` | Override the generation model for this invocation |
| `--show-context` | Print retrieved source chunks alongside the answer |
| `--format <fmt>` | Output format (`text` or `json`) |
| `--reindex` | Force re-embedding of all files, ignoring cached hashes |
| `-i <glob>` | Include only files matching this glob (repeatable) |
| `-x <glob>` | Exclude files matching this glob, in addition to the built-in defaults (repeatable) |

## Configuration

### Default excludes

maharajah automatically skips directories that contain build artifacts, not source code. The built-in list covers all supported languages:

| Pattern | Toolchain |
|---|---|
| `**/target/**` | Cargo (Rust), Maven, sbt (Scala) |
| `**/node_modules/**` | npm / yarn / pnpm |
| `**/__pycache__/**`, `.venv/**`, `venv/**`, `env/**` | Python |
| `vendor/**` | Go modules |
| `dist-newstyle/**`, `.stack-work/**` | Cabal, Stack (Haskell) |
| `.bundle/**` | Bundler (Ruby) |
| `.gradle/**`, `**/build/**`, `.sbt/**` | Gradle, sbt (Java / Scala) |
| `**/bin/Debug/**`, `**/bin/Release/**`, `**/obj/**` | MSBuild (C# / F#) |

These patterns are applied to directories, so descent is pruned early — large trees like `target/` are never walked at all.

To add project-specific excludes on top of the defaults, use `-x` on the command line or `default_excludes` in `maharajah.toml`. To override the list entirely, replace `default_excludes` in your global `~/.maharajah/maharajah.toml`.

---

maharajah uses a three-layer configuration model:

1. **Global defaults** (built in) — sensible values that work out of the box
2. **Global config** (`~/.maharajah/maharajah.toml`) — auto-created on first run; edit to change defaults for all projects
3. **Project config** (`<project-dir>/maharajah.toml`) — optional, never auto-created; overrides the global config for a specific project
4. **Environment variables** — highest priority; prefixed `MAHARAJAH_`, nested with `__`
   (e.g. `MAHARAJAH_OLLAMA__BASE_URL=http://gpu-box:11434`)

### Example `maharajah.toml`

```toml
[ollama]
base_url = "http://localhost:11434"
embed_model = "nomic-embed-text"
generate_model = "llama3.2"
timeout_secs = 120

[db]
table_name = "chunks"
embedding_dim = 768

[index]
max_chunk_lines = 40
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

## Supported languages

| Extension | Parser |
|---|---|
| `rs` | tree-sitter-rust |
| `py` | tree-sitter-python |
| `js`, `jsx` | tree-sitter-javascript |
| `ts`, `tsx` | tree-sitter-typescript |
| `go` | tree-sitter-go |
| `java` | tree-sitter-java |
| `cs` | tree-sitter-c-sharp |
| `fs`, `fsx` | tree-sitter-fsharp |
| `scala` | tree-sitter-scala |
| `hs` | tree-sitter-haskell |
| `rb` | tree-sitter-ruby |

## How it works

**Indexing:** maharajah walks the project directory, reads each file, and computes a SHA-256 hash. Files whose hash matches a stored value are skipped. Changed or new files are parsed with tree-sitter into AST-aware chunks (functions, classes, methods); files with no matching parser are skipped. Each chunk is sent to Ollama's embedding endpoint and the resulting vector is stored in a LanceDB table alongside metadata (file path, hash, language, symbol name, line range).

**Querying:** The question is embedded with the same model, and the top-*k* nearest chunks are retrieved from LanceDB via vector search. The chunks are assembled into a prompt and streamed through Ollama's generation model, which produces a grounded answer. `find` stops after the retrieval step and returns the ranked chunks directly without LLM generation.
