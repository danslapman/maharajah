/// Parser integration tests: verify that tree-sitter correctly splits each
/// supported language into the expected top-level symbol chunks.
///
/// Each test asserts:
/// - at least the expected number of chunks are produced (not just a fallback
///   whole-file line-split which would yield 1-2 large chunks)
/// - every expected symbol name is present in at least one chunk
/// - at least one chunk carries a non-None summary (doc comment extraction)

#[cfg(test)]
mod parser_tests {
    use crate::indexer::parser::parse_file;
    use std::path::Path;

    fn symbols(chunks: &[crate::indexer::parser::Chunk]) -> Vec<&str> {
        chunks.iter().map(|c| c.symbol.as_str()).collect()
    }

    fn has_summary(chunks: &[crate::indexer::parser::Chunk]) -> bool {
        chunks.iter().any(|c| c.summary.is_some())
    }

    // ── Rust ──────────────────────────────────────────────────────────────────

    #[test]
    fn rust_example_chunks() {
        let content = include_str!("../../example/math.rs");
        let chunks = parse_file(Path::new("math.rs"), content, 40);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 5, "expected ≥5 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"add"), "missing symbol 'add'");
        assert!(syms.contains(&"factorial"), "missing symbol 'factorial'");
        assert!(syms.contains(&"max_val"), "missing symbol 'max_val'");
        assert!(syms.contains(&"Stack"), "missing symbol 'Stack'");
        assert!(has_summary(&chunks), "no summaries extracted for rust example");
    }

    // ── Python ────────────────────────────────────────────────────────────────

    #[test]
    fn python_example_chunks() {
        let content = include_str!("../../example/utils.py");
        let chunks = parse_file(Path::new("utils.py"), content, 40);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 3, "expected ≥3 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"parse_args"), "missing symbol 'parse_args'");
        assert!(syms.contains(&"chunk_list"), "missing symbol 'chunk_list'");
        assert!(syms.contains(&"RingBuffer"), "missing symbol 'RingBuffer'");
        assert!(has_summary(&chunks), "no summaries extracted for python example");
    }

    // ── Go ────────────────────────────────────────────────────────────────────

    #[test]
    fn go_example_chunks() {
        let content = include_str!("../../example/greet.go");
        let chunks = parse_file(Path::new("greet.go"), content, 40);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 3, "expected ≥3 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"Greet"), "missing symbol 'Greet'");
        assert!(syms.contains(&"Reverse"), "missing symbol 'Reverse'");
        assert!(syms.contains(&"CountWords"), "missing symbol 'CountWords'");
        assert!(has_summary(&chunks), "no summaries extracted for go example");
    }

    // ── Java ──────────────────────────────────────────────────────────────────

    #[test]
    fn java_example_chunks() {
        let content = include_str!("../../example/shapes.java");
        let chunks = parse_file(Path::new("shapes.java"), content, 80);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 2, "expected ≥2 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"Point"), "missing symbol 'Point'");
        assert!(has_summary(&chunks), "no summaries extracted for java example");
    }

    // ── C# ────────────────────────────────────────────────────────────────────

    #[test]
    fn csharp_example_chunks() {
        let content = include_str!("../../example/collections.cs");
        let chunks = parse_file(Path::new("collections.cs"), content, 80);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 2, "expected ≥2 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"MinHeap"), "missing symbol 'MinHeap'");
        assert!(has_summary(&chunks), "no summaries extracted for csharp example");
    }

    // ── Scala ─────────────────────────────────────────────────────────────────

    #[test]
    fn scala_example_chunks() {
        let content = include_str!("../../example/algebra.scala");
        let chunks = parse_file(Path::new("algebra.scala"), content, 40);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 2, "expected ≥2 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"Rational"), "missing symbol 'Rational'");
        assert!(has_summary(&chunks), "no summaries extracted for scala example");
    }

    // ── Haskell ───────────────────────────────────────────────────────────────

    #[test]
    fn haskell_example_chunks() {
        let content = include_str!("../../example/Combinatorics.hs");
        let chunks = parse_file(Path::new("Combinatorics.hs"), content, 40);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 3, "expected ≥3 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"choose"), "missing symbol 'choose'");
        assert!(syms.contains(&"fibonacci"), "missing symbol 'fibonacci'");
        assert!(syms.contains(&"compress"), "missing symbol 'compress'");
        // Note: tree-sitter-haskell wraps sibling functions in a `declarations`
        // group; haddock comments precede that group rather than individual
        // `function` nodes, so summary extraction is not currently supported.
    }

    // ── JavaScript ────────────────────────────────────────────────────────────

    #[test]
    fn javascript_example_chunks() {
        let content = include_str!("../../example/dom.js");
        let chunks = parse_file(Path::new("dom.js"), content, 40);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 3, "expected ≥3 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"debounce"), "missing symbol 'debounce'");
        assert!(syms.contains(&"deepClone"), "missing symbol 'deepClone'");
        assert!(syms.contains(&"groupBy"), "missing symbol 'groupBy'");
        assert!(has_summary(&chunks), "no summaries extracted for javascript example");
    }

    // ── TypeScript ────────────────────────────────────────────────────────────

    #[test]
    fn typescript_example_chunks() {
        let content = include_str!("../../example/validation.ts");
        let chunks = parse_file(Path::new("validation.ts"), content, 40);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 3, "expected ≥3 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"validateEmail"), "missing symbol 'validateEmail'");
        assert!(syms.contains(&"formatDate"), "missing symbol 'formatDate'");
        assert!(syms.contains(&"User"), "missing symbol 'User'");
        assert!(has_summary(&chunks), "no summaries extracted for typescript example");
    }

    // ── Ruby ──────────────────────────────────────────────────────────────────

    #[test]
    fn ruby_example_chunks() {
        let content = include_str!("../../example/text.rb");
        let chunks = parse_file(Path::new("text.rb"), content, 40);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 4, "expected ≥4 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"camelize"), "missing symbol 'camelize'");
        assert!(syms.contains(&"word_frequency"), "missing symbol 'word_frequency'");
        assert!(syms.contains(&"LruCache"), "missing symbol 'LruCache'");
        assert!(has_summary(&chunks), "no summaries extracted for ruby example");
    }

    // ── F# ────────────────────────────────────────────────────────────────────

    #[test]
    fn fsharp_example_chunks() {
        let content = include_str!("../../example/Numerics.fs");
        let chunks = parse_file(Path::new("Numerics.fs"), content, 40);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 3, "expected ≥3 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"pow"), "missing symbol 'pow'");
        assert!(syms.contains(&"gcd"), "missing symbol 'gcd'");
        assert!(syms.contains(&"isPrime"), "missing symbol 'isPrime'");
        assert!(has_summary(&chunks), "no summaries extracted for fsharp example");
    }
}
