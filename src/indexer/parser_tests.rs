/// Parser integration tests: verify that tree-sitter correctly splits each
/// supported language into the expected top-level symbol chunks.
///
/// Each test asserts:
/// - at least the expected number of chunks are produced (not just a fallback
///   whole-file line-split which would yield 1-2 large chunks)
/// - every expected symbol name is present in at least one chunk
/// - specific symbols carry the correct summary text (no definition bleed-through)

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

    fn summary_for<'a>(chunks: &'a [crate::indexer::parser::Chunk], symbol: &str) -> Option<&'a str> {
        chunks.iter().find(|c| c.symbol == symbol).and_then(|c| c.summary.as_deref())
    }

    /// Assert that a summary:
    /// 1. is present (not None),
    /// 2. contains an expected keyword phrase,
    /// 3. does NOT contain definition-syntax text that would indicate the code
    ///    body leaked into the summary.
    fn assert_summary_ok(
        summary: Option<&str>,
        symbol: &str,
        must_contain: &str,
        must_not_contain: &[&str],
    ) {
        let s = summary.unwrap_or_else(|| panic!("no summary for symbol '{symbol}'"));
        assert!(
            s.contains(must_contain),
            "summary for '{symbol}' should contain {must_contain:?}, got: {s:?}"
        );
        for fragment in must_not_contain {
            assert!(
                !s.contains(fragment),
                "summary for '{symbol}' must not contain {fragment:?} (definition bleed-through), got: {s:?}"
            );
        }
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

        assert_summary_ok(
            summary_for(&chunks, "add"),
            "add",
            "Adds two integers together",
            &["fn add", "pub fn", "-> i32"],
        );
        assert_summary_ok(
            summary_for(&chunks, "factorial"),
            "factorial",
            "factorial",
            &["fn factorial", "pub fn", "-> u64"],
        );
        assert_summary_ok(
            summary_for(&chunks, "Stack"),
            "Stack",
            "stack backed by a Vec",
            &["pub struct", "struct Stack"],
        );
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

        assert_summary_ok(
            summary_for(&chunks, "parse_args"),
            "parse_args",
            "Parse command-line arguments",
            &["def parse_args", "    def"],
        );
        // Multi-line docstring must be dedented — continuation lines must not
        // have leading spaces from the source indentation.
        let pa_summary = summary_for(&chunks, "parse_args").unwrap();
        for line in pa_summary.lines().skip(1).filter(|l| !l.is_empty()) {
            assert!(
                !line.starts_with("    "),
                "docstring continuation line still has source indentation: {line:?}"
            );
        }

        assert_summary_ok(
            summary_for(&chunks, "RingBuffer"),
            "RingBuffer",
            "circular buffer",
            &["class RingBuffer", "def __init__"],
        );
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

        assert_summary_ok(
            summary_for(&chunks, "Greet"),
            "Greet",
            "greeting string",
            &["func Greet", "func(", "string {"],
        );
        assert_summary_ok(
            summary_for(&chunks, "Reverse"),
            "Reverse",
            "UTF-8",
            &["func Reverse", "string {"],
        );
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

        assert_summary_ok(
            summary_for(&chunks, "Point"),
            "Point",
            "2D point",
            &["public class", "class Point", "double x"],
        );
        assert_summary_ok(
            summary_for(&chunks, "circleArea"),
            "circleArea",
            "area of a circle",
            &["public static", "double radius", "Math.PI"],
        );
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

        // XML doc tags must be stripped — summary must not contain <summary> etc.
        assert_summary_ok(
            summary_for(&chunks, "MinHeap"),
            "MinHeap",
            "min-heap",
            &["<summary>", "</summary>", "public class", "List<T>"],
        );
        assert_summary_ok(
            summary_for(&chunks, "StringUtils"),
            "StringUtils",
            "string manipulation",
            &["<summary>", "</summary>", "public static class"],
        );
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

        assert_summary_ok(
            summary_for(&chunks, "Rational"),
            "Rational",
            "rational number",
            // Block-comment delimiters must be stripped
            &["/**", "*/", "case class", "numer: Int"],
        );
        assert_summary_ok(
            summary_for(&chunks, "isPerfectSquare"),
            "isPerfectSquare",
            "perfect square",
            &["/**", "*/", "def isPerfectSquare", "Boolean"],
        );
        // Note: tree-sitter-scala parses the body of `isPerfectSquare` to include
        // the following block comment (indentation-based scope), so `flatten`
        // currently does not receive a summary. This is a known tree-sitter-scala
        // limitation; we merely assert it is still parsed as a chunk.
        assert!(syms.contains(&"flatten"), "missing symbol 'flatten'");
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
        assert!(has_summary(&chunks), "no summaries extracted for haskell example");

        // tree-sitter-haskell uses `-- |` Haddock syntax; the `-- |` prefix
        // and surrounding whitespace must be stripped, leaving clean text.
        assert_summary_ok(
            summary_for(&chunks, "choose"),
            "choose",
            "binomial coefficient",
            // Haddock marker and definition must not appear in the summary
            &["-- |", "-- ", "choose ::", "Integer ->"],
        );
        assert_summary_ok(
            summary_for(&chunks, "fibonacci"),
            "fibonacci",
            "Fibonacci number",
            &["-- |", "-- ", "fibonacci ::", "Integer ->"],
        );
        // Only the first pattern-match function inherits the haddock; subsequent
        // pattern-match functions for the same name have no comment.
        assert_summary_ok(
            summary_for(&chunks, "compress"),
            "compress",
            "consecutive duplicate",
            &["-- |", "-- ", "compress ::", "Eq a =>"],
        );
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

        assert_summary_ok(
            summary_for(&chunks, "debounce"),
            "debounce",
            "Debounces a function",
            &["/**", "*/", "function debounce", "let timer"],
        );
        assert_summary_ok(
            summary_for(&chunks, "deepClone"),
            "deepClone",
            "Deep-clones",
            &["/**", "*/", "function deepClone", "JSON.parse"],
        );
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

        assert_summary_ok(
            summary_for(&chunks, "validateEmail"),
            "validateEmail",
            "well-formed email",
            &["/**", "*/", "function validateEmail", ": string"],
        );
        assert_summary_ok(
            summary_for(&chunks, "User"),
            "User",
            "user with an id",
            &["/**", "*/", "interface User", "id: number"],
        );
    }

    // ── TSX ───────────────────────────────────────────────────────────────────

    #[test]
    fn tsx_example_chunks() {
        let content = include_str!("../../example/components.tsx");
        let chunks = parse_file(Path::new("components.tsx"), content, 40);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 3, "expected ≥3 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"Counter"), "missing symbol 'Counter'");
        assert!(syms.contains(&"formatCurrency"), "missing symbol 'formatCurrency'");
        assert!(syms.contains(&"ListItem"), "missing symbol 'ListItem'");
        assert!(has_summary(&chunks), "no summaries extracted for tsx example");

        assert_summary_ok(
            summary_for(&chunks, "Counter"),
            "Counter",
            "counter component",
            &["/**", "*/", "function Counter", "useState"],
        );
        assert_summary_ok(
            summary_for(&chunks, "formatCurrency"),
            "formatCurrency",
            "currency string",
            &["/**", "*/", "function formatCurrency", ": string"],
        );
        assert_summary_ok(
            summary_for(&chunks, "ListItem"),
            "ListItem",
            "labeled item",
            &["/**", "*/", "interface ListItem", "id: number"],
        );
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

        assert_summary_ok(
            summary_for(&chunks, "camelize"),
            "camelize",
            "snake_case to CamelCase",
            &["def camelize", "str.split"],
        );
        assert_summary_ok(
            summary_for(&chunks, "LruCache"),
            "LruCache",
            "LRU cache",
            &["class LruCache", "def initialize"],
        );
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

        assert_summary_ok(
            summary_for(&chunks, "pow"),
            "pow",
            "integer power",
            &["///", "let rec pow", "(b: int)", ": int ="],
        );
        assert_summary_ok(
            summary_for(&chunks, "gcd"),
            "gcd",
            "greatest common divisor",
            &["///", "let rec gcd", "(a: int)"],
        );
        assert_summary_ok(
            summary_for(&chunks, "isPrime"),
            "isPrime",
            "prime",
            &["///", "let isPrime", "(n: int)"],
        );
    }

    // ── Kotlin ────────────────────────────────────────────────────────────────

    #[test]
    fn kotlin_example_chunks() {
        let content = include_str!("../../example/geometry.kt");
        let chunks = parse_file(Path::new("geometry.kt"), content, 40);
        let syms = symbols(&chunks);
        assert!(chunks.len() >= 4, "expected ≥4 chunks, got {}: {:?}", chunks.len(), syms);
        assert!(syms.contains(&"Vector2"), "missing symbol 'Vector2'");
        assert!(syms.contains(&"distance"), "missing symbol 'distance'");
        assert!(syms.contains(&"BoundingBox"), "missing symbol 'BoundingBox'");
        assert!(syms.contains(&"Angles"), "missing symbol 'Angles'");
        assert!(has_summary(&chunks), "no summaries extracted for kotlin example");

        assert_summary_ok(
            summary_for(&chunks, "Vector2"),
            "Vector2",
            "2D vector",
            &["/**", "*/", "data class", "val x"],
        );
        assert_summary_ok(
            summary_for(&chunks, "distance"),
            "distance",
            "Euclidean distance",
            &["/**", "*/", "fun distance", "Double {"],
        );
        assert_summary_ok(
            summary_for(&chunks, "BoundingBox"),
            "BoundingBox",
            "bounding box",
            &["/**", "*/", "class BoundingBox", "val minX"],
        );
        assert_summary_ok(
            summary_for(&chunks, "Angles"),
            "Angles",
            "angle conversion",
            &["/**", "*/", "object Angles"],
        );
    }

    // ── Non-doc comments must not become summaries ────────────────────────────
    //
    // Regression: a plain `--` comment containing code-like text was being
    // used as the summary for the following Haskell function because
    // `comment_kinds_for("haskell")` previously included the `"comment"` kind
    // alongside `"haddock"`. The same risk exists for `//` comments in Rust,
    // Java, JS, TS, Scala, C#, and F# where only a specific marker (`///`,
    // `/**`) denotes documentation.

    /// Assert that every chunk for `symbol` has `summary == None`.
    /// Also verifies the symbol was actually parsed (prevents vacuous passes
    /// when tree-sitter produces no output for invalid input).
    fn assert_no_summary_for(
        chunks: &[crate::indexer::parser::Chunk],
        symbol: &str,
        context: &str,
    ) {
        assert!(
            chunks.iter().any(|c| c.symbol == symbol),
            "{context}: symbol {symbol:?} not found in chunks — check the test fixture"
        );
        for c in chunks.iter().filter(|c| c.symbol == symbol) {
            assert!(
                c.summary.is_none(),
                "{context}: non-doc comment must not become a summary for '{symbol}', got {:?}",
                c.summary
            );
        }
    }

    #[test]
    fn non_doc_comment_does_not_become_summary() {
        // ── Haskell: plain `--` (not `-- |`) ─────────────────────────────────
        // Directly reproduces the reported bug: a commented-out guard line was
        // showing up as the summary of the following function.
        // Note: tree-sitter-haskell uses `function` for definitions with
        // arguments (patterns), and `bind` for zero-argument value bindings;
        // we need a function with an argument to hit the affected code path.
        {
            let src = concat!(
                "module T where\n",
                "-- foo apiUser _ _ | not apiUser.allowed = throwError err402\n",
                "foo :: Int -> Int\n",
                "foo x = x + 1\n",
            );
            let chunks = parse_file(Path::new("t.hs"), src, 80);
            assert_no_summary_for(&chunks, "foo", "haskell plain -- comment");
        }

        // ── Rust: plain `//` (not `///`) ──────────────────────────────────────
        {
            let src = concat!(
                "// plain implementation note, not a doc comment\n",
                "fn add(a: i32, b: i32) -> i32 { a + b }\n",
            );
            let chunks = parse_file(Path::new("t.rs"), src, 80);
            assert_no_summary_for(&chunks, "add", "rust plain // comment");
        }

        // ── Java: `//` line comment (not `/** */` Javadoc) ───────────────────
        {
            let src = concat!(
                "// plain implementation note, not javadoc\n",
                "public static int add(int a, int b) { return a + b; }\n",
            );
            let chunks = parse_file(Path::new("t.java"), src, 80);
            assert_no_summary_for(&chunks, "add", "java plain // comment");
        }

        // ── JavaScript: `//` (not `/** */` JSDoc) ────────────────────────────
        {
            let src = concat!(
                "// plain implementation note, not jsdoc\n",
                "function add(a, b) { return a + b; }\n",
            );
            let chunks = parse_file(Path::new("t.js"), src, 80);
            assert_no_summary_for(&chunks, "add", "javascript plain // comment");
        }

        // ── TypeScript: `//` (not `/** */` JSDoc) ────────────────────────────
        {
            let src = concat!(
                "// plain implementation note, not jsdoc\n",
                "function add(a: number, b: number): number { return a + b; }\n",
            );
            let chunks = parse_file(Path::new("t.ts"), src, 80);
            assert_no_summary_for(&chunks, "add", "typescript plain // comment");
        }

        // ── Scala: `//` (not `/** */` Scaladoc) ──────────────────────────────
        {
            let src = concat!(
                "// plain implementation note, not scaladoc\n",
                "def add(a: Int, b: Int): Int = a + b\n",
            );
            let chunks = parse_file(Path::new("t.scala"), src, 80);
            assert_no_summary_for(&chunks, "add", "scala plain // comment");
        }

        // ── C#: `//` (not `///` XML doc) ─────────────────────────────────────
        // Note: collect_chunks doesn't recurse into class_declaration, so we
        // test a top-level class (the outermost interesting node) rather than
        // a method inside one.
        {
            let src = concat!(
                "// plain implementation note, not xml doc\n",
                "public class Widget { }\n",
            );
            let chunks = parse_file(Path::new("t.cs"), src, 80);
            assert_no_summary_for(&chunks, "Widget", "csharp plain // comment");
        }

        // ── F#: `//` (not `///` XML doc) ─────────────────────────────────────
        {
            let src = concat!(
                "module T\n",
                "// plain implementation note, not xml doc\n",
                "let add (a: int) (b: int) : int = a + b\n",
            );
            let chunks = parse_file(Path::new("t.fs"), src, 80);
            assert_no_summary_for(&chunks, "add", "fsharp plain // comment");
        }

        // ── Kotlin: `//` (not `/** */` KDoc) ─────────────────────────────────
        {
            let src = concat!(
                "// plain implementation note, not kdoc\n",
                "fun add(a: Int, b: Int): Int = a + b\n",
            );
            let chunks = parse_file(Path::new("t.kt"), src, 80);
            assert_no_summary_for(&chunks, "add", "kotlin plain // comment");
        }
    }
}
