//! Base trait and config for storage writers.

use crate::reader::models::OrderbookData;
use anyhow::Result;

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

/// Trait for writing orderbook data to storage
pub trait StorageWriter: Send {
    /// Initialize the writer with configuration
    fn init(&mut self, config: WriterConfig) -> Result<()>;

    /// Write a single orderbook data point
    fn write(&mut self, data: &OrderbookData) -> Result<()>;

    /// Write a batch of orderbook data points
    fn write_batch(&mut self, batch: &[OrderbookData]) -> Result<()>;

    /// Flush any buffered data to storage
    fn flush(&mut self) -> Result<()>;

    /// Close the writer and finalize the file
    fn close(&mut self) -> Result<()>;

    /// Get the file extension for this writer type
    fn file_extension(&self) -> &'static str;
}
