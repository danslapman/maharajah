use crate::indexer::parser::Chunk;

/// Split `content` into overlapping windows of at most `max_lines` lines.
/// `start_offset` is the line number of the first line of `content` within the original file.
pub fn split_by_lines(
    content: &str,
    symbol: &str,
    language: &str,
    start_offset: u32,
    max_lines: usize,
) -> Vec<Chunk> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut offset = 0usize;

    while offset < lines.len() {
        let end = (offset + max_lines).min(lines.len());
        let chunk_lines = &lines[offset..end];
        let chunk_content = chunk_lines.join("\n");
        let start_line = start_offset + offset as u32;
        let end_line = start_offset + end as u32 - 1;

        chunks.push(Chunk {
            language: language.to_string(),
            symbol: symbol.to_string(),
            content: chunk_content,
            start_line,
            end_line,
        });

        offset += max_lines;
    }

    chunks
}
