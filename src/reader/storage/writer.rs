use anyhow::Result;
use crate::reader::models::OrderbookData;

/// Configuration for storage writers
#[derive(Debug, Clone)]
pub struct WriterConfig {
    pub base_filename: String,
    pub buffer_size: usize,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            base_filename: String::new(),
            buffer_size: 1000,
        }
    }
}

/// Trait for storage writers that can save orderbook data
pub trait StorageWriter: Send {
    /// Initialize the writer with the given configuration
    fn init(&mut self, config: WriterConfig) -> Result<()>;
    
    /// Write a single orderbook data record
    fn write(&mut self, data: &OrderbookData) -> Result<()>;
    
    /// Flush any buffered data
    fn flush(&mut self) -> Result<()>;
    
    /// Close the writer and finalize the file
    fn close(&mut self) -> Result<()>;
    
    /// Get the file extension for this writer type
    fn file_extension(&self) -> &'static str;
}