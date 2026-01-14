use std::path::PathBuf;
use log::info;

use crate::core::{DataSource, OrderBook, Result, TradeError};
use super::{FileDataSource, ParquetDataSource};

/// A data source that chains multiple files together to process them as one continuous stream
pub struct MultiFileDataSource {
    file_paths: Vec<PathBuf>,
    current_index: usize,
    current_source: Option<Box<dyn DataSource>>,
    total_messages: usize,
    processed_messages: usize,
}

impl MultiFileDataSource {
    pub fn new(file_paths: Vec<PathBuf>) -> Result<Self> {
        if file_paths.is_empty() {
            return Err(TradeError::DataLoadingError("No files provided".to_string()));
        }
        
        let mut source = Self {
            file_paths,
            current_index: 0,
            current_source: None,
            total_messages: 0,
            processed_messages: 0,
        };
        
        // Count total messages across all files
        source.count_total_messages()?;
        
        // Initialize first file
        source.open_next_file()?;
        
        Ok(source)
    }
    
    fn count_total_messages(&mut self) -> Result<()> {
        info!("Counting messages across {} files...", self.file_paths.len());
        
        for path in &self.file_paths {
            let source: Box<dyn DataSource> = if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                let mut s = ParquetDataSource::new(path)?;
                s.count_messages()?;
                Box::new(s)
            } else {
                let mut s = FileDataSource::new(path)?;
                s.count_messages()?;
                Box::new(s)
            };
            
            if let Some(count) = source.total_count() {
                self.total_messages += count;
                info!("File {:?}: {} messages", path.file_name().unwrap_or_default(), count);
            }
        }
        
        info!("Total messages across all files: {}", self.total_messages);
        Ok(())
    }
    
    fn open_next_file(&mut self) -> Result<bool> {
        if self.current_index >= self.file_paths.len() {
            return Ok(false);
        }
        
        let path = &self.file_paths[self.current_index];
        info!("Opening file {}/{}: {:?}", 
              self.current_index + 1, 
              self.file_paths.len(), 
              path.file_name().unwrap_or_default());
        
        let source: Box<dyn DataSource> = if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
            Box::new(ParquetDataSource::new(path)?)
        } else {
            Box::new(FileDataSource::new(path)?.with_batch_size(10000))
        };
        
        self.current_source = Some(source);
        self.current_index += 1;
        
        Ok(true)
    }
}

impl DataSource for MultiFileDataSource {
    fn next_orderbook(&mut self) -> Result<Option<OrderBook>> {
        loop {
            if let Some(ref mut source) = self.current_source {
                // Try to get next orderbook from current source
                if let Some(orderbook) = source.next_orderbook()? {
                    self.processed_messages += 1;
                    
                    // Log progress every 10%
                    let progress = (self.processed_messages * 100) / self.total_messages.max(1);
                    if progress > 0 && self.processed_messages % (self.total_messages / 10).max(1) == 0 {
                        info!("Overall progress: {}% ({}/{} messages)", 
                              progress, self.processed_messages, self.total_messages);
                    }
                    
                    return Ok(Some(orderbook));
                }
            }
            
            // Current source exhausted, try next file
            if !self.open_next_file()? {
                // No more files
                return Ok(None);
            }
        }
    }
    
    fn reset(&mut self) -> Result<()> {
        self.current_index = 0;
        self.processed_messages = 0;
        self.current_source = None;
        self.open_next_file()?;
        Ok(())
    }
    
    fn total_count(&self) -> Option<usize> {
        Some(self.total_messages)
    }
}