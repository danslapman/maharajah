use std::path::Path;
use tree_sitter::{Language, Parser};

use crate::indexer::chunker;

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
    "signature",
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
    "function_or_value_defn",
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
    collect_chunks(root, content, lang_name, interesting_kinds, max_chunk_lines, &mut chunks);

    if chunks.is_empty() {
        chunks = chunker::split_by_lines(content, "", lang_name, 0, max_chunk_lines, "", None);
    }

    chunks
}

fn collect_chunks(
    node: tree_sitter::Node,
    content: &str,
    lang_name: &str,
    interesting_kinds: &[&str],
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

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_chunks(child, content, lang_name, interesting_kinds, max_chunk_lines, chunks);
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
        "fsharp" => matches!(kind, "function_or_value_defn" | "type_defn"),
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
        "scala" => &["comment"],
        "haskell" => &["comment", "block_comment"],
        "ruby" => &["comment"],
        "fsharp" => &["block_comment", "line_comment"],
        _ => &[],
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

    // Walk preceding named siblings collecting consecutive comment nodes
    let mut comments: Vec<String> = Vec::new();
    let mut sib = node.prev_named_sibling();
    while let Some(s) = sib {
        if comment_kinds.contains(&s.kind()) {
            comments.push(strip_comment_markers(&content[s.byte_range()], lang));
            sib = s.prev_named_sibling();
        } else {
            break;
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

/// Strip triple-quote (or single-quote) delimiters from a Python string literal.
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
    inner.trim().to_string()
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
            "haskell" => "--",
            _ => "//",
        };
        trimmed
            .lines()
            .map(|l| {
                let s = l.trim();
                s.strip_prefix(line_prefix).unwrap_or(s).trim()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Try to extract a human-readable name for a node by looking for its first
/// identifier-like child.  Returns an empty string if none is found.
fn get_node_name(node: &tree_sitter::Node, content: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "type_identifier" | "simple_identifier" | "name" => {
                return content[child.byte_range()].to_string();
            }
            _ => continue,
        }
    }
    String::new()
}
