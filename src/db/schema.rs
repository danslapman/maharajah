use arrow_schema::{DataType, Field, Fields, Schema};
use std::sync::Arc;

/// Arrow schema for indexed code chunks stored in LanceDB.
///
/// Columns:
/// - id         : unique chunk identifier (file_path:start_line)
/// - file_path  : source file path
/// - file_hash  : SHA-256 hex of file content (for incremental updates)
/// - language   : detected language (rust, python, ...)
/// - symbol     : tree-sitter node name (fn/class name, or empty string)
/// - content    : raw source text of the chunk
/// - start_line : 0-based start line in file
/// - end_line   : 0-based end line in file
/// - summary    : extracted docstring/comment summary (nullable)
/// - vector     : embedding vector (FixedSizeList<Float32>)
pub fn chunks_schema(embedding_dim: usize) -> Arc<Schema> {
    Arc::new(Schema::new(Fields::from(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("file_path", DataType::Utf8, false),
        Field::new("file_hash", DataType::Utf8, false),
        Field::new("language", DataType::Utf8, false),
        Field::new("symbol", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("start_line", DataType::UInt32, false),
        Field::new("end_line", DataType::UInt32, false),
        Field::new("summary", DataType::Utf8, true),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                embedding_dim as i32,
            ),
            false,
        ),
    ])))
}
