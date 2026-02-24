use std::path::Path;
use tree_sitter::{Language, Parser};

use crate::indexer::chunker;

#[cfg(test)]
#[path = "parser_tests.rs"]
mod parser_tests;


pub struct Chunk {
    pub language: String,
    pub symbol: String,
    pub content: String,
    pub start_line: u32,
    pub end_line: u32,
    /// Raw tree-sitter node kind (e.g. "function_item") — stored for future filtering/display.
    #[allow(dead_code)]
    pub node_kind: String,
    /// Extracted docstring or preceding comment block, if available
    pub summary: Option<String>,
}

// ── per-language node kinds that represent meaningful top-level definitions ───

const RUST_KINDS: &[&str] = &[
    "function_item",
    "impl_item",
    "struct_item",
    "enum_item",
    "trait_item",
    "type_item",
    "const_item",
    "static_item",
    "mod_item",
    "macro_definition",
    "union_item",
];

const PYTHON_KINDS: &[&str] = &[
    "function_definition",
    "class_definition",
    "decorated_definition",
];

const JAVA_KINDS: &[&str] = &[
    "method_declaration",
    "class_declaration",
    "interface_declaration",
    "enum_declaration",
    "record_declaration",
    "annotation_type_declaration",
    "constructor_declaration",
];

const CSHARP_KINDS: &[&str] = &[
    "method_declaration",
    "class_declaration",
    "interface_declaration",
    "struct_declaration",
    "enum_declaration",
    "record_declaration",
    "delegate_declaration",
    "property_declaration",
    "constructor_declaration",
];

const SCALA_KINDS: &[&str] = &[
    "function_definition",
    "class_definition",
    "object_definition",
    "trait_definition",
    "enum_definition",
    "given_definition",
    "extension_definition",
    "type_definition",
];

const HASKELL_KINDS: &[&str] = &[
    "function",
    // `signature` is intentionally omitted: it always appears next to a
    // `function` node which carries the full definition and the summary.
    "data_type",
    "newtype",
    "class",
    "instance_decl",
    "type_synomym",
    "type_family",
];

const JS_KINDS: &[&str] = &[
    "function_declaration",
    "class_declaration",
    "method_definition",
    "arrow_function",
    "generator_function_declaration",
];

const TS_KINDS: &[&str] = &[
    "function_declaration",
    "class_declaration",
    "method_definition",
    "interface_declaration",
    "type_alias_declaration",
    "enum_declaration",
];

const GO_KINDS: &[&str] = &[
    "function_declaration",
    "method_declaration",
    "type_declaration",
    "const_declaration",
    "var_declaration",
];

const RUBY_KINDS: &[&str] = &["method", "class", "module", "singleton_method", "singleton_class"];

const FSHARP_KINDS: &[&str] = &[
    // value_declaration wraps function_or_value_defn; comments sit before value_declaration
    "value_declaration",
    "type_defn",
    "module_defn",
    "namespace",
    "exception_definition",
];

// ─────────────────────────────────────────────────────────────────────────────

pub fn parse_file(path: &Path, content: &str, max_chunk_lines: usize) -> Vec<Chunk> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();

    match ext.as_str() {
        "rs" => parse_with_grammar(
            content,
            tree_sitter_rust::LANGUAGE.into(),
            "rust",
            RUST_KINDS,
            max_chunk_lines,
        ),
        "py" => parse_with_grammar(
            content,
            tree_sitter_python::LANGUAGE.into(),
            "python",
            PYTHON_KINDS,
            max_chunk_lines,
        ),
        "java" => parse_with_grammar(
            content,
            tree_sitter_java::LANGUAGE.into(),
            "java",
            JAVA_KINDS,
            max_chunk_lines,
        ),
        "cs" => parse_with_grammar(
            content,
            tree_sitter_c_sharp::LANGUAGE.into(),
            "csharp",
            CSHARP_KINDS,
            max_chunk_lines,
        ),
        "scala" | "sc" => parse_with_grammar(
            content,
            tree_sitter_scala::LANGUAGE.into(),
            "scala",
            SCALA_KINDS,
            max_chunk_lines,
        ),
        "hs" => parse_with_grammar(
            content,
            tree_sitter_haskell::LANGUAGE.into(),
            "haskell",
            HASKELL_KINDS,
            max_chunk_lines,
        ),
        "js" | "cjs" | "mjs" | "jsx" => parse_with_grammar(
            content,
            tree_sitter_javascript::LANGUAGE.into(),
            "javascript",
            JS_KINDS,
            max_chunk_lines,
        ),
        "ts" => parse_with_grammar(
            content,
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            "typescript",
            TS_KINDS,
            max_chunk_lines,
        ),
        "tsx" => parse_with_grammar(
            content,
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            "tsx",
            TS_KINDS,
            max_chunk_lines,
        ),
        "go" => parse_with_grammar(
            content,
            tree_sitter_go::LANGUAGE.into(),
            "go",
            GO_KINDS,
            max_chunk_lines,
        ),
        "rb" => parse_with_grammar(
            content,
            tree_sitter_ruby::LANGUAGE.into(),
            "ruby",
            RUBY_KINDS,
            max_chunk_lines,
        ),
        "fs" | "fsx" => parse_with_grammar(
            content,
            tree_sitter_fsharp::LANGUAGE_FSHARP.into(),
            "fsharp",
            FSHARP_KINDS,
            max_chunk_lines,
        ),
        _ => vec![],
    }
}

/// Generic tree-sitter parser: walks the AST and collects nodes whose kind is in
/// `interesting_kinds`.  Falls back to line-based chunking if the tree could not
/// be parsed or no interesting nodes were found.
fn parse_with_grammar(
    content: &str,
    language: Language,
    lang_name: &str,
    interesting_kinds: &[&str],
    max_chunk_lines: usize,
) -> Vec<Chunk> {
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return chunker::split_by_lines(content, "", lang_name, 0, max_chunk_lines, "", None);
    }

    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return chunker::split_by_lines(content, "", lang_name, 0, max_chunk_lines, "", None),
    };

    let root = tree.root_node();
    let mut chunks = Vec::new();
    let prune_kinds = prune_kinds_for(lang_name);
    collect_chunks(root, content, lang_name, interesting_kinds, prune_kinds, max_chunk_lines, &mut chunks);

    if chunks.is_empty() {
        chunks = chunker::split_by_lines(content, "", lang_name, 0, max_chunk_lines, "", None);
    }

    chunks
}

/// Node kinds that should never be recursed into during chunk collection.
/// This prevents false-positive matches when a grammar reuses the same kind
/// for structurally different constructs (e.g. Haskell uses `function` for
/// both top-level definitions and function-type expressions inside signatures).
fn prune_kinds_for(lang: &str) -> &'static [&'static str] {
    match lang {
        // `signature` nodes contain function-type sub-trees whose kind is
        // `function` — the same kind used for top-level definitions.
        "haskell" => &["signature"],
        _ => &[],
    }
}

fn collect_chunks(
    node: tree_sitter::Node,
    content: &str,
    lang_name: &str,
    interesting_kinds: &[&str],
    prune_kinds: &[&str],
    max_chunk_lines: usize,
    chunks: &mut Vec<Chunk>,
) {
    if interesting_kinds.contains(&node.kind()) {
        let start_line = node.start_position().row as u32;
        let end_line = node.end_position().row as u32;
        let node_content = &content[node.byte_range()];
        let symbol = get_node_name(&node, content);
        let line_count = (end_line - start_line + 1) as usize;

        let summary = if is_summary_kind(lang_name, node.kind()) {
            extract_comment(node, content, lang_name)
        } else {
            None
        };

        if line_count > max_chunk_lines {
            let sub = chunker::split_by_lines(
                node_content,
                &symbol,
                lang_name,
                start_line,
                max_chunk_lines,
                node.kind(),
                summary.as_deref(),
            );
            chunks.extend(sub);
        } else {
            chunks.push(Chunk {
                language: lang_name.to_string(),
                symbol,
                content: node_content.to_string(),
                start_line,
                end_line,
                node_kind: node.kind().to_string(),
                summary,
            });
        }
        // Don't recurse into matched nodes
        return;
    }

    // Don't recurse into pruned node kinds — they may contain sub-nodes whose
    // kind collides with interesting_kinds but represent something different
    // (e.g. Haskell `function` type-expressions inside `signature` nodes).
    if prune_kinds.contains(&node.kind()) {
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_chunks(child, content, lang_name, interesting_kinds, prune_kinds, max_chunk_lines, chunks);
    }
}

/// Returns true for node kinds where a docstring/comment summary is meaningful.
/// Skips constants, type aliases, modules — their name typically is the summary.
fn is_summary_kind(lang: &str, kind: &str) -> bool {
    match lang {
        "rust" => matches!(
            kind,
            "function_item" | "impl_item" | "trait_item" | "struct_item" | "enum_item"
        ),
        "python" => matches!(
            kind,
            "function_definition" | "class_definition" | "decorated_definition"
        ),
        "javascript" => matches!(
            kind,
            "function_declaration"
                | "class_declaration"
                | "method_definition"
                | "arrow_function"
                | "generator_function_declaration"
        ),
        "typescript" | "tsx" => matches!(
            kind,
            "function_declaration"
                | "class_declaration"
                | "method_definition"
                | "interface_declaration"
        ),
        "java" => matches!(
            kind,
            "method_declaration"
                | "class_declaration"
                | "interface_declaration"
                | "constructor_declaration"
        ),
        "csharp" => matches!(
            kind,
            "method_declaration"
                | "class_declaration"
                | "interface_declaration"
                | "constructor_declaration"
                | "property_declaration"
        ),
        "go" => matches!(kind, "function_declaration" | "method_declaration"),
        "scala" => matches!(
            kind,
            "function_definition"
                | "class_definition"
                | "trait_definition"
                | "given_definition"
                | "extension_definition"
        ),
        "haskell" => matches!(kind, "function" | "class" | "instance_decl"),
        "ruby" => matches!(
            kind,
            "method" | "class" | "module" | "singleton_method" | "singleton_class"
        ),
        "fsharp" => matches!(kind, "value_declaration" | "type_defn"),
        _ => false,
    }
}

/// Returns the comment node kinds for a given language.
fn comment_kinds_for(lang: &str) -> &'static [&'static str] {
    match lang {
        "rust" => &["line_comment", "block_comment"],
        "python" => &["comment"],
        "java" => &["block_comment", "line_comment"],
        "csharp" => &["comment", "block_comment"],
        "javascript" | "typescript" | "tsx" => &["comment"],
        "go" => &["comment"],
        // tree-sitter-scala uses "block_comment" for /** */ scaladoc
        "scala" => &["comment", "block_comment"],
        // tree-sitter-haskell uses "haddock" for -- | doc comments
        "haskell" => &["comment", "block_comment", "haddock"],
        "ruby" => &["comment"],
        "fsharp" => &["block_comment", "line_comment"],
        _ => &[],
    }
}

/// Returns true if a comment node (identified by its raw text and tree-sitter
/// kind) is a *documentation* comment rather than an ordinary implementation
/// comment or commented-out code.
///
/// Each language has a distinct documentation comment convention:
/// - Rust/C#/F#: `///` (triple-slash)
/// - Java/JS/TS/TSX/Scala: `/** … */` (Javadoc / JSDoc / Scaladoc)
/// - Haskell: `-- |` Haddock (tree-sitter kind `"haddock"`) or `{-| … -}`
/// - Go/Ruby/Python: every `//` / `#` comment immediately preceding a
///   declaration is considered documentation (godoc / YARD / pydoc convention)
fn is_doc_comment(raw: &str, lang: &str, kind: &str) -> bool {
    let trimmed = raw.trim();
    match lang {
        "rust" => trimmed.starts_with("///") || trimmed.starts_with("/**"),
        "csharp" | "fsharp" => trimmed.starts_with("///") || trimmed.starts_with("/**"),
        "java" => trimmed.starts_with("/**"),
        "javascript" | "typescript" | "tsx" => trimmed.starts_with("/**"),
        "scala" => trimmed.starts_with("/**"),
        // Only Haddock-marked comments are documentation; plain `--` comments
        // (kind == "comment") are implementation notes and must not be used.
        "haskell" => kind == "haddock" || trimmed.starts_with("{-|"),
        // Go, Ruby, Python: the preceding comment block is the documentation
        _ => true,
    }
}

/// Extract the docstring or preceding comment block for a node.
fn extract_comment(node: tree_sitter::Node, content: &str, lang: &str) -> Option<String> {
    // Python: check for docstring as first statement in the body
    if lang == "python" {
        if let Some(doc) = extract_python_docstring(node, content) {
            if !doc.is_empty() {
                return Some(doc);
            }
        }
        // Fall through to check for # comments before the def
    }

    let comment_kinds = comment_kinds_for(lang);
    if comment_kinds.is_empty() {
        return None;
    }

    // Walk preceding named siblings collecting consecutive comment nodes.
    // For Haskell: type signatures (`signature`) sit between the haddock and
    // the function node — skip over them to reach the haddock.
    let skip_kinds: &[&str] = if lang == "haskell" { &["signature"] } else { &[] };

    let mut comments: Vec<String> = Vec::new();
    // Track whether we stopped because we ran out of siblings (true) or
    // because a non-comment, non-skippable sibling blocked us (false).
    let mut siblings_exhausted = true;
    let mut sib = node.prev_named_sibling();
    while let Some(s) = sib {
        if comment_kinds.contains(&s.kind()) {
            let raw = &content[s.byte_range()];
            if is_doc_comment(raw, lang, s.kind()) {
                comments.push(strip_comment_markers(raw, lang));
                sib = s.prev_named_sibling();
            } else {
                // A non-doc comment (implementation note, commented-out code,
                // etc.) immediately before the symbol blocks collection — it
                // separates the symbol from any real doc comment above.
                siblings_exhausted = false;
                break;
            }
        } else if skip_kinds.contains(&s.kind()) {
            sib = s.prev_named_sibling();
        } else {
            siblings_exhausted = false;
            break;
        }
    }

    // Haskell only: if no comment was found AND we exhausted all siblings
    // (i.e. were not blocked by another function node), the first function in
    // a `declarations` block inherits the haddock that precedes the block.
    // (tree-sitter-haskell places the first function's haddock as a sibling of
    // `declarations` at the module root, not inside the block itself.)
    if lang == "haskell" && comments.is_empty() && siblings_exhausted {
        if let Some(parent) = node.parent() {
            if parent.kind() == "declarations" {
                let mut parent_sib = parent.prev_named_sibling();
                while let Some(ps) = parent_sib {
                    let raw = &content[ps.byte_range()];
                    if comment_kinds.contains(&ps.kind()) && is_doc_comment(raw, lang, ps.kind()) {
                        comments.push(strip_comment_markers(raw, lang));
                        parent_sib = ps.prev_named_sibling();
                    } else {
                        break;
                    }
                }
            }
        }
    }

    comments.reverse();

    let joined = comments.join("\n").trim().to_string();
    if joined.is_empty() { None } else { Some(joined) }
}

/// For Python: look for the first `expression_statement > string` inside the
/// function/class body block.
fn extract_python_docstring(node: tree_sitter::Node, content: &str) -> Option<String> {
    let mut outer_cursor = node.walk();
    for child in node.named_children(&mut outer_cursor) {
        if child.kind() == "block" {
            let mut block_cursor = child.walk();
            for block_child in child.named_children(&mut block_cursor) {
                if block_child.kind() == "expression_statement" {
                    let mut expr_cursor = block_child.walk();
                    for expr_child in block_child.named_children(&mut expr_cursor) {
                        if expr_child.kind() == "string" {
                            let raw = &content[expr_child.byte_range()];
                            return Some(strip_python_docstring(raw));
                        }
                    }
                }
                // Only inspect the very first statement
                break;
            }
        }
    }
    None
}

/// Strip triple-quote (or single-quote) delimiters from a Python string literal,
/// then dedent continuation lines so the result has no spurious leading whitespace.
fn strip_python_docstring(s: &str) -> String {
    let s = s.trim();
    let inner = if s.starts_with("\"\"\"") && s.ends_with("\"\"\"") && s.len() >= 6 {
        &s[3..s.len() - 3]
    } else if s.starts_with("'''") && s.ends_with("'''") && s.len() >= 6 {
        &s[3..s.len() - 3]
    } else if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    };
    dedent_docstring(inner)
}

/// Dedent a docstring body: strip the common leading whitespace from all
/// non-empty continuation lines (lines after the first).
fn dedent_docstring(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= 1 {
        return s.trim().to_string();
    }
    // Find minimum indentation among non-empty continuation lines
    let min_indent = lines[1..]
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);

    let dedented: Vec<&str> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                l.trim()
            } else if l.trim().is_empty() {
                ""
            } else {
                &l[min_indent.min(l.len())..]
            }
        })
        .collect();
    dedented.join("\n").trim().to_string()
}

/// Strip comment delimiters from a raw comment node's text.
fn strip_comment_markers(raw: &str, lang: &str) -> String {
    let trimmed = raw.trim();

    if trimmed.starts_with("/*") {
        // Block comment: /* ... */ or /** ... */
        let inner = trimmed
            .strip_prefix("/**")
            .or_else(|| trimmed.strip_prefix("/*"))
            .unwrap_or(trimmed);
        let inner = inner.strip_suffix("*/").unwrap_or(inner);

        // Clean each line: strip leading * and surrounding whitespace
        inner
            .lines()
            .map(|l| {
                let s = l.trim();
                s.strip_prefix('*').unwrap_or(s).trim()
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        // Line comment
        let line_prefix: &str = match lang {
            "rust" | "csharp" | "fsharp" => {
                if trimmed.starts_with("///") { "///" } else { "//" }
            }
            "python" | "ruby" => "#",
            // Haddock uses `-- |`; regular Haskell line comments use `--`
            "haskell" => {
                if trimmed.starts_with("-- |") { "-- |" } else { "--" }
            }
            _ => "//",
        };
        let stripped = trimmed
            .lines()
            .map(|l| {
                let s = l.trim();
                s.strip_prefix(line_prefix).unwrap_or(s).trim()
            })
            .collect::<Vec<_>>()
            .join("\n");

        // C# XML doc comments use <summary>/<param>/<returns> tags — strip them
        if lang == "csharp" { strip_xml_tags(&stripped) } else { stripped }
    }
}

/// Remove XML tags (`<...>`) from a string, keeping only the text content.
/// Empty lines produced by removing tag-only lines are filtered out.
fn strip_xml_tags(s: &str) -> String {
    s.lines()
        .map(|line| {
            let mut out = String::new();
            let mut in_tag = false;
            for ch in line.chars() {
                match ch {
                    '<' => in_tag = true,
                    '>' => in_tag = false,
                    _ if !in_tag => out.push(ch),
                    _ => {}
                }
            }
            out
        })
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Try to extract a human-readable name for a node by looking for its first
/// identifier-like child.  Returns an empty string if none is found.
fn get_node_name(node: &tree_sitter::Node, content: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            // Most languages: identifier or type-identifier node kinds
            "identifier" | "type_identifier" | "simple_identifier" | "name" => {
                return content[child.byte_range()].to_string();
            }
            // Ruby: class/module names are `constant` nodes
            "constant" => {
                return content[child.byte_range()].to_string();
            }
            // Haskell: function names are `variable` nodes
            "variable" => {
                return content[child.byte_range()].to_string();
            }
            // F#: function name is the first word of function_declaration_left /
            // value_declaration_left (e.g. "pow (b: int) (e: int)" → "pow")
            "function_declaration_left" | "value_declaration_left" => {
                let text = &content[child.byte_range()];
                return text.split_whitespace().next().unwrap_or("").to_string();
            }
            // F#: value_declaration wraps function_or_value_defn — delegate one level down
            "function_or_value_defn" => {
                let name = get_node_name(&child, content);
                if !name.is_empty() {
                    return name;
                }
            }
            _ => continue,
        }
    }
    String::new()
}
