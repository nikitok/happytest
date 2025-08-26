use super::writer::{StorageWriter, WriterConfig};
use crate::reader::models::OrderbookData;
use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};

/// JSONL (newline-delimited JSON) writer implementation with batching
pub struct JsonlWriter {
    writer: Option<BufWriter<File>>,
    buffer: Vec<OrderbookData>,
    config: WriterConfig,
}

impl JsonlWriter {
    pub fn new() -> Self {
        Self {
            writer: None,
            buffer: Vec::new(),
            config: WriterConfig::default(),
        }
    }
    
    /// Write buffered data to JSONL file
    fn flush_buffer(&mut self) -> Result<()> {
        if !self.buffer.is_empty() {
            if let Some(writer) = &mut self.writer {
                for data in &self.buffer {
                    let json_line = serde_json::to_string(data)
                        .context("Failed to serialize data to JSON")?;
                    writeln!(writer, "{}", json_line)
                        .context("Failed to write to JSONL file")?;
                }
                writer.flush().context("Failed to flush JSONL writer")?;
                log::debug!("Wrote batch of {} records to JSONL file", self.buffer.len());
                self.buffer.clear();
            }
        }
        Ok(())
    }
}

impl StorageWriter for JsonlWriter {
    fn init(&mut self, config: WriterConfig) -> Result<()> {
        self.config = config;
        
        let filename = format!("{}.jsonl", self.config.base_filename);
        log::info!("Creating JSONL output file: {}", filename);
        
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&filename)
            .context("Failed to create JSONL output file")?;
        
        self.writer = Some(BufWriter::new(file));
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
        
        // Close the JSONL writer
        if let Some(mut writer) = self.writer.take() {
            writer.flush().context("Failed to flush JSONL writer on close")?;
            log::info!("JSONL file saved: {}.jsonl", self.config.base_filename);
        }
        Ok(())
    }
    
    fn file_extension(&self) -> &'static str {
        "jsonl"
    }
}