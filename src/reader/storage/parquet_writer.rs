use super::writer::{StorageWriter, WriterConfig};
use crate::reader::models::OrderbookData;
use anyhow::{Context, Result};
use serde_json;
use std::fs::File;

// Parquet imports
use arrow::array::{ArrayRef, Int64Array, StringBuilder};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::sync::Arc;

/// Parquet writer implementation with batching
pub struct ParquetWriter {
    writer: Option<ArrowWriter<File>>,
    buffer: Vec<OrderbookData>,
    config: WriterConfig,
}

impl ParquetWriter {
    pub fn new() -> Self {
        Self {
            writer: None,
            buffer: Vec::new(),
            config: WriterConfig::default(),
        }
    }
    
    /// Create the schema for Parquet file
    fn create_schema() -> Schema {
        Schema::new(vec![
            Field::new("symbol", DataType::Utf8, false),
            Field::new("bids", DataType::Utf8, false), // JSON string of bids
            Field::new("asks", DataType::Utf8, false), // JSON string of asks
            Field::new("timestamp", DataType::Int64, false),
            Field::new("update_id", DataType::Int64, false),
            Field::new("fetch_time", DataType::Int64, false),
        ])
    }
    
    /// Convert buffered data to Arrow arrays
    fn convert_to_arrow_batch(records: &[OrderbookData]) -> Result<RecordBatch> {
        let mut symbol_builder = StringBuilder::new();
        let mut bids_builder = StringBuilder::new();
        let mut asks_builder = StringBuilder::new();
        let mut timestamp_builder = Int64Array::builder(records.len());
        let mut update_id_builder = Int64Array::builder(records.len());
        let mut fetch_time_builder = Int64Array::builder(records.len());

        for record in records {
            symbol_builder.append_value(&record.symbol);
            
            // Serialize bids and asks as JSON strings
            let bids_json = serde_json::to_string(&record.bids)
                .context("Failed to serialize bids")?;
            let asks_json = serde_json::to_string(&record.asks)
                .context("Failed to serialize asks")?;
            
            bids_builder.append_value(&bids_json);
            asks_builder.append_value(&asks_json);
            
            timestamp_builder.append_value(record.timestamp);
            update_id_builder.append_value(record.update_id);
            fetch_time_builder.append_value(record.fetch_time);
        }

        let arrays: Vec<ArrayRef> = vec![
            Arc::new(symbol_builder.finish()),
            Arc::new(bids_builder.finish()),
            Arc::new(asks_builder.finish()),
            Arc::new(timestamp_builder.finish()),
            Arc::new(update_id_builder.finish()),
            Arc::new(fetch_time_builder.finish()),
        ];

        let schema = Arc::new(Self::create_schema());
        RecordBatch::try_new(schema, arrays).context("Failed to create record batch")
    }
    
    /// Write buffered data to Parquet file
    fn flush_buffer(&mut self) -> Result<()> {
        if !self.buffer.is_empty() {
            if let Some(writer) = &mut self.writer {
                let batch = Self::convert_to_arrow_batch(&self.buffer)?;
                writer.write(&batch).context("Failed to write Parquet batch")?;
                log::debug!("Wrote batch of {} records to Parquet file", self.buffer.len());
                self.buffer.clear();
            }
        }
        Ok(())
    }
}

impl StorageWriter for ParquetWriter {
    fn init(&mut self, config: WriterConfig) -> Result<()> {
        self.config = config;
        
        let filename = format!("{}.parquet", self.config.base_filename);
        log::info!("Creating Parquet output file: {}", filename);
        
        let file = File::create(&filename)
            .context("Failed to create Parquet output file")?;
        
        let schema = Arc::new(Self::create_schema());
        let props = WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build();
        
        self.writer = Some(
            ArrowWriter::try_new(file, schema, Some(props))
                .context("Failed to create Parquet writer")?
        );
        
        Ok(())
    }
    
    fn write(&mut self, data: &OrderbookData) -> Result<()> {
        self.buffer.push(data.clone());
        
        // Write batch when buffer is full
        if self.buffer.len() >= self.config.buffer_size {
            self.flush_buffer()?;
        }
        
        Ok(())
    }
    
    fn write_batch(&mut self, batch: &[OrderbookData]) -> Result<()> {
        // Add batch to buffer
        self.buffer.extend_from_slice(batch);
        
        // Write if buffer is full or force write if batch is large
        if self.buffer.len() >= self.config.buffer_size || batch.len() >= self.config.buffer_size {
            self.flush_buffer()?;
        }
        
        Ok(())
    }
    
    fn flush(&mut self) -> Result<()> {
        self.flush_buffer()
    }
    
    fn close(&mut self) -> Result<()> {
        // Write any remaining buffered data
        self.flush_buffer()?;
        
        // Close the Parquet writer
        if let Some(writer) = self.writer.take() {
            writer.close().context("Failed to close Parquet writer")?;
            log::info!("Parquet file saved: {}.parquet", self.config.base_filename);
        }
        
        Ok(())
    }
    
    fn file_extension(&self) -> &'static str {
        "parquet"
    }
}