use std::fs::File;
use std::path::{Path, PathBuf};
use log::{info, debug};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use arrow::array::{Int64Array, StringArray};
use arrow::record_batch::RecordBatch;
use serde_json;

use crate::core::{OrderBook, errors::{Result, TradeError}, traits::DataSource};

/// Parquet-based data source for order book messages
pub struct ParquetDataSource {
    file_path: PathBuf,
    symbol: String,
    current_batch: Option<RecordBatch>,
    current_row: usize,
    batch_reader: Option<parquet::arrow::arrow_reader::ParquetRecordBatchReader>,
    total_messages: Option<usize>,
}

impl ParquetDataSource {
    pub fn new(file_path: impl AsRef<Path>) -> Result<Self> {
        let path = file_path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(TradeError::DataLoadingError(
                format!("File not found: {:?}", path)
            ));
        }
        
        // Extract symbol from filename
        let symbol = path.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.split('_').next())
            .unwrap_or("UNKNOWN")
            .to_string();
        
        Ok(Self {
            file_path: path,
            symbol,
            current_batch: None,
            current_row: 0,
            batch_reader: None,
            total_messages: None,
        })
    }
    
    /// Initialize the Parquet reader
    fn init_reader(&mut self) -> Result<()> {
        if self.batch_reader.is_some() {
            return Ok(());
        }
        
        let file = File::open(&self.file_path)
            .map_err(|e| TradeError::DataLoadingError(
                format!("Failed to open parquet file: {}", e)
            ))?;
        
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| TradeError::DataLoadingError(
                format!("Failed to create parquet reader: {}", e)
            ))?;
        
        // Get total row count from metadata
        let metadata = builder.metadata();
        let total_rows: usize = metadata.file_metadata().num_rows() as usize;
        self.total_messages = Some(total_rows);
        info!("Parquet file contains {} rows", total_rows);
        
        // Create the batch reader with a reasonable batch size
        let batch_reader = builder
            .with_batch_size(10000)
            .build()
            .map_err(|e| TradeError::DataLoadingError(
                format!("Failed to build parquet reader: {}", e)
            ))?;
        
        self.batch_reader = Some(batch_reader);
        Ok(())
    }
    
    /// Load the next batch of records
    fn load_next_batch(&mut self) -> Result<bool> {
        if self.batch_reader.is_none() {
            self.init_reader()?;
        }
        
        let reader = self.batch_reader.as_mut().unwrap();
        
        match reader.next() {
            Some(Ok(batch)) => {
                debug!("Loaded batch with {} rows", batch.num_rows());
                self.current_batch = Some(batch);
                self.current_row = 0;
                Ok(true)
            }
            Some(Err(e)) => Err(TradeError::DataLoadingError(
                format!("Failed to read parquet batch: {}", e)
            )),
            None => Ok(false), // No more batches
        }
    }
    
    /// Parse a row from the current batch into an OrderBook
    fn parse_row(&self, batch: &RecordBatch, row_idx: usize) -> Result<OrderBook> {
        // The Parquet file has columns: timestamp (Int64), bids (String/JSON), asks (String/JSON)
        
        let ts_column = batch
            .column_by_name("timestamp")
            .ok_or_else(|| TradeError::DataLoadingError("Missing 'timestamp' column in parquet".to_string()))?
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| TradeError::DataLoadingError("'timestamp' column is not Int64".to_string()))?;
        
        let bids_column = batch
            .column_by_name("bids")
            .ok_or_else(|| TradeError::DataLoadingError("Missing 'bids' column in parquet".to_string()))?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| TradeError::DataLoadingError("'bids' column is not String".to_string()))?;
            
        let asks_column = batch
            .column_by_name("asks")
            .ok_or_else(|| TradeError::DataLoadingError("Missing 'asks' column in parquet".to_string()))?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| TradeError::DataLoadingError("'asks' column is not String".to_string()))?;
        
        let ts = ts_column.value(row_idx);
        let bids_json = bids_column.value(row_idx);
        let asks_json = asks_column.value(row_idx);
        
        // Parse the JSON arrays directly
        let bid_array: Vec<Vec<String>> = serde_json::from_str(bids_json)
            .map_err(|e| TradeError::DataLoadingError(
                format!("Failed to parse bids JSON: {}", e)
            ))?;
            
        let ask_array: Vec<Vec<String>> = serde_json::from_str(asks_json)
            .map_err(|e| TradeError::DataLoadingError(
                format!("Failed to parse asks JSON: {}", e)
            ))?;
        
        // Parse bids
        let mut bids = Vec::new();
        for bid in &bid_array {
            if bid.len() >= 2 {
                let price = bid[0].parse::<f64>()
                    .map_err(|_| TradeError::InvalidOrderBook(
                        format!("Invalid bid price: {}", bid[0])
                    ))?;
                let quantity = bid[1].parse::<f64>()
                    .map_err(|_| TradeError::InvalidOrderBook(
                        format!("Invalid bid quantity: {}", bid[1])
                    ))?;
                bids.push((price, quantity));
            }
        }
        
        // Parse asks
        let mut asks = Vec::new();
        for ask in &ask_array {
            if ask.len() >= 2 {
                let price = ask[0].parse::<f64>()
                    .map_err(|_| TradeError::InvalidOrderBook(
                        format!("Invalid ask price: {}", ask[0])
                    ))?;
                let quantity = ask[1].parse::<f64>()
                    .map_err(|_| TradeError::InvalidOrderBook(
                        format!("Invalid ask quantity: {}", ask[1])
                    ))?;
                asks.push((price, quantity));
            }
        }
        
        Ok(OrderBook::new(self.symbol.clone(), bids, asks, ts))
    }
    
    /// Count total messages (rows) in the file
    pub fn count_messages(&mut self) -> Result<usize> {
        if let Some(count) = self.total_messages {
            return Ok(count);
        }
        
        // Initialize reader to get metadata
        self.init_reader()?;
        
        Ok(self.total_messages.unwrap_or(0))
    }
}

impl DataSource for ParquetDataSource {
    fn next_orderbook(&mut self) -> Result<Option<OrderBook>> {
        // Check if we need to load the first batch or a new batch
        if self.current_batch.is_none() || 
           self.current_row >= self.current_batch.as_ref().unwrap().num_rows() {
            if !self.load_next_batch()? {
                return Ok(None); // No more data
            }
        }
        
        // Get the current batch
        if let Some(batch) = &self.current_batch {
            let orderbook = self.parse_row(batch, self.current_row)?;
            self.current_row += 1;
            Ok(Some(orderbook))
        } else {
            Ok(None)
        }
    }
    
    fn reset(&mut self) -> Result<()> {
        self.batch_reader = None;
        self.current_batch = None;
        self.current_row = 0;
        Ok(())
    }
    
    fn total_count(&self) -> Option<usize> {
        self.total_messages
    }
}