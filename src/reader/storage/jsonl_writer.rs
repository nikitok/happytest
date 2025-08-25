use super::writer::{StorageWriter, WriterConfig};
use crate::reader::models::OrderbookData;
use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};

/// JSONL (newline-delimited JSON) writer implementation
pub struct JsonlWriter {
    writer: Option<BufWriter<File>>,
    config: WriterConfig,
}

impl JsonlWriter {
    pub fn new() -> Self {
        Self {
            writer: None,
            config: WriterConfig::default(),
        }
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
        if let Some(writer) = &mut self.writer {
            let json_line = serde_json::to_string(data)
                .context("Failed to serialize data to JSON")?;
            
            writeln!(writer, "{}", json_line)
                .context("Failed to write to JSONL file")?;
        }
        Ok(())
    }
    
    fn flush(&mut self) -> Result<()> {
        if let Some(writer) = &mut self.writer {
            writer.flush().context("Failed to flush JSONL writer")?;
        }
        Ok(())
    }
    
    fn close(&mut self) -> Result<()> {
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