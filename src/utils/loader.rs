use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;
use log::{info, debug};
use serde::{Deserialize, Serialize};

use crate::core::{OrderBook, errors::{Result, TradeError}, traits::DataSource};

#[derive(Debug, Deserialize, Serialize)]
pub struct OrderBookMessage {
    #[serde(alias = "timestamp")]
    pub ts: i64,
    pub data: OrderBookData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OrderBookData {
    pub b: Vec<Vec<String>>, // bids: [[price, quantity], ...]
    pub a: Vec<Vec<String>>, // asks: [[price, quantity], ...]
}

// Alternative format for newer JSONL/Parquet files
#[derive(Debug, Deserialize, Serialize)]
pub struct OrderBookMessageV2 {
    pub symbol: String,
    pub bids: Vec<Vec<String>>,
    pub asks: Vec<Vec<String>>,
    pub timestamp: i64,
    pub update_id: i64,
    pub fetch_time: i64,
}

/// File-based data source for order book messages
pub struct FileDataSource {
    file_path: PathBuf,
    symbol: String,
    reader: Option<BufReader<File>>,
    buffer: Vec<String>,
    current_index: usize,
    batch_size: usize,
    total_messages: Option<usize>,
}

impl FileDataSource {
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
            reader: None,
            buffer: Vec::new(),
            current_index: 0,
            batch_size: 10000,
            total_messages: None,
        })
    }
    
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }
    
    /// Load a batch of lines from the file
    fn load_batch(&mut self) -> Result<bool> {
        if self.reader.is_none() {
            let file = File::open(&self.file_path)
                .map_err(|e| TradeError::DataLoadingError(
                    format!("Failed to open file: {}", e)
                ))?;
            self.reader = Some(BufReader::new(file));
        }
        
        self.buffer.clear();
        self.current_index = 0;
        
        let reader = self.reader.as_mut().unwrap();
        let batch_start = Instant::now();
        
        for _ in 0..self.batch_size {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if !line.trim().is_empty() {
                        self.buffer.push(line);
                    }
                }
                Err(e) => return Err(TradeError::IoError(e)),
            }
        }
        
        if !self.buffer.is_empty() {
            debug!("Loaded batch of {} messages in {:.3}s", 
                   self.buffer.len(), batch_start.elapsed().as_secs_f64());
        }
        
        Ok(!self.buffer.is_empty())
    }
    
    /// Parse a message into an OrderBook
    fn parse_message(&self, message: &OrderBookMessage) -> Result<OrderBook> {
        let mut bids = Vec::new();
        for bid in &message.data.b {
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
        
        let mut asks = Vec::new();
        for ask in &message.data.a {
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
        
        Ok(OrderBook::new(self.symbol.clone(), bids, asks, message.ts))
    }
    
    /// Parse a V2 format message into an OrderBook
    fn parse_message_v2(message: &OrderBookMessageV2) -> Result<OrderBook> {
        let mut bids = Vec::new();
        for bid in &message.bids {
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
        
        let mut asks = Vec::new();
        for ask in &message.asks {
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
        
        Ok(OrderBook::new(message.symbol.clone(), bids, asks, message.timestamp))
    }
    
    /// Pre-count total messages in the file (optional, for progress tracking)
    pub fn count_messages(&mut self) -> Result<usize> {
        if let Some(count) = self.total_messages {
            return Ok(count);
        }
        
        let start = Instant::now();
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        
        let count = reader.lines().filter(|l| l.is_ok()).count();
        self.total_messages = Some(count);
        
        info!("Counted {} messages in {:.2}s", count, start.elapsed().as_secs_f64());
        Ok(count)
    }
}

impl DataSource for FileDataSource {
    fn next_orderbook(&mut self) -> Result<Option<OrderBook>> {
        // Check if we need to load a new batch
        if self.current_index >= self.buffer.len() {
            if !self.load_batch()? {
                return Ok(None); // EOF
            }
        }
        
        // Get the next line from buffer
        if let Some(line) = self.buffer.get(self.current_index) {
            self.current_index += 1;
            
            // Try parsing as V2 format first (newer format)
            if let Ok(message_v2) = serde_json::from_str::<OrderBookMessageV2>(line) {
                let orderbook = Self::parse_message_v2(&message_v2)?;
                Ok(Some(orderbook))
            } else {
                // Fall back to V1 format (older format)
                let message: OrderBookMessage = serde_json::from_str(line)?;
                let orderbook = self.parse_message(&message)?;
                Ok(Some(orderbook))
            }
        } else {
            Ok(None)
        }
    }
    
    fn reset(&mut self) -> Result<()> {
        self.reader = None;
        self.buffer.clear();
        self.current_index = 0;
        Ok(())
    }
    
    fn total_count(&self) -> Option<usize> {
        self.total_messages
    }
}

/// Extract symbol from filename (e.g., "ETHUSDT_3600_sec_123.jsonl" -> "ETHUSDT")
pub fn extract_symbol_from_filename(filename: &str) -> String {
    filename.split('_').next().unwrap_or("UNKNOWN").to_string()
}