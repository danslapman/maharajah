use std::path::Path;
use std::sync::Arc;

use arrow_array::{
    Array, Float32Array, RecordBatch, RecordBatchIterator, StringArray, UInt32Array,
    builder::{FixedSizeListBuilder, Float32Builder, StringBuilder, UInt32Builder},
};
use arrow_schema::ArrowError;
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};

use crate::db::schema::chunks_schema;
use crate::error::{AppError, Result};

pub struct ChunkRecord {
    pub id: String,
    pub file_path: String,
    pub file_hash: String,
    pub language: String,
    pub symbol: String,
    pub content: String,
    pub start_line: u32,
    pub end_line: u32,
    pub vector: Vec<f32>,
    pub summary: Option<String>,
    pub summary_vector: Option<Vec<f32>>,
}

#[derive(serde::Serialize)]
pub struct SearchResult {
    pub id: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub symbol: String,
    pub content: String,
    pub score: f32,
    pub summary: Option<String>,
}

pub struct Store {
    table: lancedb::Table,
    embedding_dim: usize,
}

impl Store {
    pub async fn open_or_create(
        db_path: &Path,
        embedding_dim: usize,
        table_name: &str,
        reindex: bool,
    ) -> Result<Self> {
        let uri = db_path.to_str().expect("db path is not valid UTF-8");
        let conn = lancedb::connect(uri).execute().await?;

        if reindex {
            let _ = conn.drop_table(table_name, &[]).await;
        }

        let schema = chunks_schema(embedding_dim);
        let table = match conn.open_table(table_name).execute().await {
            Ok(t) => t,
            Err(lancedb::Error::TableNotFound { .. }) => {
                // First run — create the table from scratch
                conn.create_empty_table(table_name, schema).execute().await?
            }
            Err(e) => {
                // Corruption, schema mismatch, I/O error, etc.
                // Propagate — user can recover with `index --reindex`
                return Err(AppError::Database(e));
            }
        };

        Ok(Store {
            table,
            embedding_dim,
        })
    }

    pub async fn try_open(
        db_path: &Path,
        embedding_dim: usize,
        table_name: &str,
    ) -> Result<Option<Self>> {
        let uri = db_path.to_str().expect("db path is not valid UTF-8");
        let conn = lancedb::connect(uri).execute().await?;
        match conn.open_table(table_name).execute().await {
            Ok(table) => Ok(Some(Store { table, embedding_dim })),
            Err(lancedb::Error::TableNotFound { .. }) => Ok(None),
            Err(e) => Err(AppError::Database(e)),
        }
    }

    pub async fn count_rows(&self) -> Result<usize> {
        let mut total = 0usize;
        let mut stream = self.table.query().execute().await?;
        while let Some(batch) = stream.try_next().await? {
            total += batch.num_rows();
        }
        Ok(total)
    }

    pub async fn list_files(&self) -> Result<std::collections::HashSet<String>> {
        let mut files = std::collections::HashSet::new();
        let mut stream = self
            .table
            .query()
            .select(lancedb::query::Select::Columns(vec!["file_path".into()]))
            .execute()
            .await?;
        while let Some(batch) = stream.try_next().await? {
            if let Some(col) = batch.column_by_name("file_path") {
                if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
                    for i in 0..arr.len() {
                        if !arr.is_null(i) {
                            files.insert(arr.value(i).to_string());
                        }
                    }
                }
            }
        }
        Ok(files)
    }

    pub async fn count_files(&self) -> Result<usize> {
        Ok(self.list_files().await?.len())
    }

    pub async fn clear(&self) -> Result<()> {
        self.table.delete("1 = 1").await?;
        Ok(())
    }

    pub async fn get_file_hash(&self, file_path: &str) -> Result<Option<String>> {
        let escaped = file_path.replace('\'', "''");
        let mut stream = self
            .table
            .query()
            .only_if(format!("file_path = '{}'", escaped))
            .limit(1)
            .execute()
            .await?;

        while let Some(batch) = stream.try_next().await? {
            if batch.num_rows() > 0 {
                if let Some(col) = batch.column_by_name("file_hash") {
                    if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
                        if !arr.is_null(0) {
                            return Ok(Some(arr.value(0).to_string()));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    pub async fn delete_file(&self, file_path: &str) -> Result<()> {
        let escaped = file_path.replace('\'', "''");
        self.table
            .delete(&format!("file_path = '{}'", escaped))
            .await?;
        Ok(())
    }

    pub async fn insert(&self, chunks: &[ChunkRecord]) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }

        let schema = chunks_schema(self.embedding_dim);

        let mut id_builder = StringBuilder::new();
        let mut file_path_builder = StringBuilder::new();
        let mut file_hash_builder = StringBuilder::new();
        let mut language_builder = StringBuilder::new();
        let mut symbol_builder = StringBuilder::new();
        let mut content_builder = StringBuilder::new();
        let mut start_line_builder = UInt32Builder::new();
        let mut end_line_builder = UInt32Builder::new();
        let mut summary_builder = StringBuilder::new();
        let mut vector_builder =
            FixedSizeListBuilder::new(Float32Builder::new(), self.embedding_dim as i32);
        let mut summary_vector_builder =
            FixedSizeListBuilder::new(Float32Builder::new(), self.embedding_dim as i32);

        for chunk in chunks {
            id_builder.append_value(&chunk.id);
            file_path_builder.append_value(&chunk.file_path);
            file_hash_builder.append_value(&chunk.file_hash);
            language_builder.append_value(&chunk.language);
            symbol_builder.append_value(&chunk.symbol);
            content_builder.append_value(&chunk.content);
            start_line_builder.append_value(chunk.start_line);
            end_line_builder.append_value(chunk.end_line);
            summary_builder.append_option(chunk.summary.as_deref());
            for &v in &chunk.vector {
                vector_builder.values().append_value(v);
            }
            vector_builder.append(true);
            match &chunk.summary_vector {
                Some(sv) => {
                    for &v in sv {
                        summary_vector_builder.values().append_value(v);
                    }
                    summary_vector_builder.append(true);
                }
                None => {
                    // Arrow FixedSizeList invariant: child.len() must always equal
                    // list.len() * item_size, even for null entries.
                    for _ in 0..self.embedding_dim {
                        summary_vector_builder.values().append_value(0.0);
                    }
                    summary_vector_builder.append(false);
                }
            }
        }

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_builder.finish()),
                Arc::new(file_path_builder.finish()),
                Arc::new(file_hash_builder.finish()),
                Arc::new(language_builder.finish()),
                Arc::new(symbol_builder.finish()),
                Arc::new(content_builder.finish()),
                Arc::new(start_line_builder.finish()),
                Arc::new(end_line_builder.finish()),
                Arc::new(summary_builder.finish()),
                Arc::new(vector_builder.finish()),
                Arc::new(summary_vector_builder.finish()),
            ],
        )
        .map_err(|e| AppError::Other(e.into()))?;

        let reader = RecordBatchIterator::new(
            vec![Ok(batch) as std::result::Result<RecordBatch, ArrowError>],
            schema,
        );

        self.table.add(reader).execute().await?;
        Ok(())
    }

    pub async fn search(&self, vector: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        let mut stream = self
            .table
            .vector_search(vector)?
            .column("vector")
            .limit(limit)
            .execute()
            .await?;

        let mut results = Vec::new();
        while let Some(batch) = stream.try_next().await? {
            for i in 0..batch.num_rows() {
                let id = get_str_col(&batch, "id", i)?;
                let file_path = get_str_col(&batch, "file_path", i)?;
                let symbol = get_str_col(&batch, "symbol", i)?;
                let content = get_str_col(&batch, "content", i)?;
                let start_line = get_u32_col(&batch, "start_line", i)?;
                let end_line = get_u32_col(&batch, "end_line", i)?;
                let score = get_f32_col(&batch, "_distance", i).unwrap_or(0.0);
                let summary = get_nullable_str_col(&batch, "summary", i)?;

                results.push(SearchResult {
                    id,
                    file_path,
                    start_line,
                    end_line,
                    symbol,
                    content,
                    score,
                    summary,
                });
            }
        }

        Ok(results)
    }

    pub async fn search_by_summary(
        &self,
        vector: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut stream = self
            .table
            .vector_search(vector)?
            .column("summary_vector")
            .only_if("summary IS NOT NULL")
            .limit(limit)
            .execute()
            .await?;

        let mut results = Vec::new();
        while let Some(batch) = stream.try_next().await? {
            for i in 0..batch.num_rows() {
                let id = get_str_col(&batch, "id", i)?;
                let file_path = get_str_col(&batch, "file_path", i)?;
                let symbol = get_str_col(&batch, "symbol", i)?;
                let content = get_str_col(&batch, "content", i)?;
                let start_line = get_u32_col(&batch, "start_line", i)?;
                let end_line = get_u32_col(&batch, "end_line", i)?;
                let score = get_f32_col(&batch, "_distance", i).unwrap_or(0.0);
                let summary = get_nullable_str_col(&batch, "summary", i)?;

                results.push(SearchResult {
                    id,
                    file_path,
                    start_line,
                    end_line,
                    symbol,
                    content,
                    score,
                    summary,
                });
            }
        }

        Ok(results)
    }
}

fn get_str_col(batch: &RecordBatch, name: &str, row: usize) -> Result<String> {
    let col = batch
        .column_by_name(name)
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("missing column: {}", name)))?;
    let arr = col
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("column {} is not StringArray", name)))?;
    Ok(arr.value(row).to_string())
}

fn get_nullable_str_col(
    batch: &RecordBatch,
    name: &str,
    row: usize,
) -> Result<Option<String>> {
    let col = match batch.column_by_name(name) {
        Some(c) => c,
        // Column absent (e.g. old schema before migration) — treat as NULL
        None => return Ok(None),
    };
    let arr = col
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("column {} is not StringArray", name)))?;
    if arr.is_null(row) {
        Ok(None)
    } else {
        Ok(Some(arr.value(row).to_string()))
    }
}

fn get_u32_col(batch: &RecordBatch, name: &str, row: usize) -> Result<u32> {
    let col = batch
        .column_by_name(name)
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("missing column: {}", name)))?;
    let arr = col
        .as_any()
        .downcast_ref::<UInt32Array>()
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("column {} is not UInt32Array", name)))?;
    Ok(arr.value(row))
}

fn get_f32_col(batch: &RecordBatch, name: &str, row: usize) -> Result<f32> {
    let col = batch
        .column_by_name(name)
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("missing column: {}", name)))?;
    let arr = col
        .as_any()
        .downcast_ref::<Float32Array>()
        .ok_or_else(|| AppError::Other(anyhow::anyhow!("column {} is not Float32Array", name)))?;
    Ok(arr.value(row))
}
