use std::path::Path;
use tree_sitter::{Language, Parser};

use crate::indexer::chunker;

pub struct Chunk {
    pub language: String,
    pub symbol: String,
    pub content: String,
    pub start_line: u32,
    pub end_line: u32,
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
        return chunker::split_by_lines(content, "", lang_name, 0, max_chunk_lines);
    }

    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return chunker::split_by_lines(content, "", lang_name, 0, max_chunk_lines),
    };

    let root = tree.root_node();
    let mut chunks = Vec::new();
    collect_chunks(root, content, lang_name, interesting_kinds, max_chunk_lines, &mut chunks);

    if chunks.is_empty() {
        chunks = chunker::split_by_lines(content, "", lang_name, 0, max_chunk_lines);
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

        if line_count > max_chunk_lines {
            let sub = chunker::split_by_lines(
                node_content,
                &symbol,
                lang_name,
                start_line,
                max_chunk_lines,
            );
            chunks.extend(sub);
        } else {
            chunks.push(Chunk {
                language: lang_name.to_string(),
                symbol,
                content: node_content.to_string(),
                start_line,
                end_line,
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

/// Try to extract a human-readable name for a node by looking for its first
/// identifier-like child.  Returns an empty string if none is found.
fn get_node_name(node: &tree_sitter::Node, content: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "type_identifier" | "simple_identifier" | "name" => {
                return content[child.byte_range()].to_string();
            }
            _ => {}
        }
    }
    String::new()
}
