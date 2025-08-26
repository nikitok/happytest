use anyhow::{Context, Result};
use chrono::Local;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use std::fs::create_dir_all;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// Import models and storage
use super::models::{OrderbookData, WsRequest, WsResponse};
use super::storage::{JsonlWriter, ParquetWriter, StorageWriter, WriterConfig};

/// Configuration for the Bybit reader
#[derive(Debug, Clone)]
pub struct ReaderConfig {
    /// Symbol to fetch data for (e.g., "BTCUSDT", "ETHUSDT")
    pub symbol: String,
    /// Interval in seconds between flushes
    pub interval_seconds: u64,
    /// Output directory for data files
    pub output_dir: String,
    /// Use testnet API
    pub testnet: bool,
    /// Orderbook depth to fetch
    pub depth: u32,
    /// Duration to run in seconds (0 for infinite)
    pub duration_seconds: u64,
    /// Save as Parquet in addition to JSONL
    pub save_parquet: bool,
}

impl Default for ReaderConfig {
    fn default() -> Self {
        Self {
            symbol: "ETHUSDT".to_string(),
            output_dir: "./data".to_string(),
            testnet: false,
            depth: 50,
            duration_seconds: 3600, // 1 hour by default
            save_parquet: true,     // Enable Parquet by default
            interval_seconds: 10, // Flush every 10 seconds by default
        }
    }
}

/// Bybit data reader using WebSocket
pub struct BybitReader {
    config: ReaderConfig,
    writers: Arc<Mutex<Vec<Box<dyn StorageWriter>>>>,
    start_time: SystemTime,
    data_buffer: Arc<Mutex<Vec<OrderbookData>>>,
}

impl BybitReader {
    /// Create a new Bybit reader with the given configuration
    pub fn new(config: ReaderConfig) -> Result<Self> {
        // Create output directory if it doesn't exist
        create_dir_all(&config.output_dir).context("Failed to create output directory")?;

        Ok(Self {
            config,
            writers: Arc::new(Mutex::new(Vec::new())),
            start_time: SystemTime::now(),
            data_buffer: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Get the WebSocket URL
    fn get_ws_url(&self) -> &'static str {
        if self.config.testnet {
            "wss://stream-testnet.bybit.com/v5/public/linear"
        } else {
            "wss://stream.bybit.com/v5/public/linear"
        }
    }

    /// Generate base filename for output files
    fn generate_base_filename(&self) -> String {
        let now = Local::now();
        let date_str = now.format("%Y%m%d_%H:%M").to_string();
        let duration_str = if self.config.duration_seconds > 0 {
            format!("{}s", self.config.duration_seconds)
        } else {
            "continuous".to_string()
        };

        format!(
            "{}/{}_{}_{}_{}",
            self.config.output_dir,
            self.config.symbol,
            date_str,
            duration_str,
            if self.config.testnet {
                "testnet"
            } else {
                "mainnet"
            }
        )
    }

    /// Initialize storage writers
    fn init_writers(&self) -> Result<Vec<Box<dyn StorageWriter>>> {
        let base_filename = self.generate_base_filename();
        let writer_config = WriterConfig {
            base_filename: base_filename.clone(),
            ..Default::default()
        };

        let mut writers: Vec<Box<dyn StorageWriter>> = Vec::new();

        // Always add JSONL writer
        let mut jsonl_writer = Box::new(JsonlWriter::new());
        jsonl_writer.init(writer_config.clone())?;
        writers.push(jsonl_writer);

        // Add Parquet writer if enabled
        if self.config.save_parquet {
            let mut parquet_writer = Box::new(ParquetWriter::new());
            parquet_writer.init(writer_config)?;
            writers.push(parquet_writer);
        }

        Ok(writers)
    }

    /// Write data to all storage writers
    fn write_data(&self, data: &OrderbookData) -> Result<()> {
        // Add data to buffer instead of writing immediately
        let mut buffer_guard = self.data_buffer.lock().unwrap();
        buffer_guard.push(data.clone());
        
        // debug!(
        //     "Buffered orderbook data: {} bids, {} asks (buffer size: {})",
        //     data.bids.len(),
        //     data.asks.len(),
        //     buffer_guard.len()
        // );

        Ok(())
    }
    
    /// Flush buffered data to all storage writers
    fn flush_data(&self) -> Result<()> {
        let mut buffer_guard = self.data_buffer.lock().unwrap();
        
        if !buffer_guard.is_empty() {
            let mut writers_guard = self.writers.lock().unwrap();
            
            for writer in writers_guard.iter_mut() {
                if let Err(e) = writer.write_batch(&buffer_guard) {
                    error!("Failed to write batch to {}: {}", writer.file_extension(), e);
                }
            }
            
            let batch_size = buffer_guard.len();
            buffer_guard.clear();
            
            debug!("Flushed batch of {} records to storage", batch_size);
        }
        
        Ok(())
    }

    /// Close all storage writers
    fn close_writers(&self) -> Result<()> {
        let mut writers_guard = self.writers.lock().unwrap();

        for writer in writers_guard.iter_mut() {
            if let Err(e) = writer.close() {
                error!("Failed to close {}: {}", writer.file_extension(), e);
            }
        }

        Ok(())
    }

    /// Run the WebSocket reader
    pub async fn run(&self) -> Result<()> {
        info!("Starting Bybit WebSocket reader for symbol: {}", self.config.symbol);
        info!("Flush interval: {} seconds", self.config.interval_seconds);
        info!(
            "Duration: {} seconds",
            if self.config.duration_seconds > 0 {
                self.config.duration_seconds.to_string()
            } else {
                "infinite".to_string()
            }
        );
        info!(
            "Parquet output: {}",
            if self.config.save_parquet {
                "enabled"
            } else {
                "disabled"
            }
        );

        // Initialize storage writers
        {
            let mut writers_guard = self.writers.lock().unwrap();
            *writers_guard = self.init_writers()?;
        }

        // Connect to WebSocket
        let ws_url = self.get_ws_url();
        info!("Connecting to WebSocket: {}", ws_url);

        let (ws_stream, _response) = connect_async(ws_url)
            .await
            .context("Failed to connect to WebSocket")?;

        info!("WebSocket connected successfully");

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to orderbook
        let subscribe_msg = WsRequest::subscribe(vec![self.config.symbol.clone()], self.config.depth);
        let subscribe_text = serde_json::to_string(&subscribe_msg)?;
        ws_sender
            .send(Message::Text(subscribe_text))
            .await
            .context("Failed to send subscribe message")?;

        info!("Subscribed to orderbook for {}", self.config.symbol);

        let mut message_count = 0u64;
        let mut error_count = 0u64;
        let mut last_ping = Instant::now();
        let mut flush_interval = interval(Duration::from_secs(self.config.interval_seconds));
        flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            // Check if we should stop
            if self.config.duration_seconds > 0 {
                let elapsed = self.start_time.elapsed().unwrap().as_secs();
                if elapsed >= self.config.duration_seconds {
                    info!("Duration reached, stopping reader");
                    break;
                }
            }

            // Send ping every 20 seconds
            if last_ping.elapsed() >= Duration::from_secs(20) {
                let ping_msg = WsRequest::ping();
                let ping_text = serde_json::to_string(&ping_msg)?;
                if let Err(e) = ws_sender.send(Message::Text(ping_text)).await {
                    warn!("Failed to send ping: {}", e);
                }
                last_ping = Instant::now();
            }

            tokio::select! {
                // Handle WebSocket messages
                Some(msg) = ws_receiver.next() => {
                    match msg {
                        Ok(Message::Text(text)) => {
                            match serde_json::from_str::<WsResponse>(&text) {
                                Ok(response) => {
                                    // Handle subscription confirmation
                                    if let Some(op) = &response.op {
                                        if op == "subscribe" {
                                            if response.success == Some(true) {
                                                info!("Subscription confirmed");
                                            } else {
                                                warn!("Subscription failed: {:?}", response.ret_msg);
                                            }
                                        } else if op == "pong" {
                                            debug!("Received pong");
                                        }
                                    }

                                    // Handle orderbook data
                                    if let Some(data) = response.data {
                                        // Only process if this is an orderbook update (has topic)
                                        if response.topic.is_some() {
                                            message_count += 1;

                                            let fetch_time = SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_millis() as i64;

                                            let orderbook_data = OrderbookData {
                                                symbol: data.s,
                                                bids: data.b,
                                                asks: data.a,
                                                timestamp: response.ts.unwrap_or(fetch_time),
                                                update_id: data.u,
                                                fetch_time,
                                            };

                                            // Write to storage
                                            if let Err(e) = self.write_data(&orderbook_data) {
                                                error!("Failed to write data: {}", e);
                                                error_count += 1;
                                            }

                                            if message_count % 100 == 0 {
                                                info!(
                                                    "Processed {} orderbook messages, {} errors",
                                                    message_count, error_count
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("Failed to parse message: {} - Text: {}", e, text);
                                }
                            }
                        }
                        Ok(Message::Close(_)) => {
                            info!("WebSocket closed by server");
                            break;
                        }
                        Ok(Message::Ping(data)) => {
                            debug!("Received ping, sending pong");
                            if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                                warn!("Failed to send pong: {}", e);
                            }
                        }
                        Ok(_) => {
                            // Ignore other message types
                        }
                        Err(e) => {
                            error!("WebSocket error: {}", e);
                            error_count += 1;
                            
                            // If too many errors, try to reconnect
                            if error_count % 10 == 0 {
                                error!("Too many errors, stopping");
                                break;
                            }
                        }
                    }
                }
                
                // Periodic flush based on interval_seconds
                _ = flush_interval.tick() => {
                    // First flush buffered data
                    if let Err(e) = self.flush_data() {
                        error!("Failed to flush data: {}", e);
                    }
                    
                    // Then flush writers
                    let mut writers_guard = self.writers.lock().unwrap();
                    for writer in writers_guard.iter_mut() {
                        if let Err(e) = writer.flush() {
                            error!("Failed to flush {}: {}", writer.file_extension(), e);
                        }
                    }
                    debug!("Flushed writers after {} seconds", self.config.interval_seconds);
                }
            }
        }

        // Close WebSocket connection
        if let Err(e) = ws_sender.close().await {
            warn!("Failed to close WebSocket: {}", e);
        }

        // Flush any remaining buffered data
        if let Err(e) = self.flush_data() {
            error!("Failed to flush remaining data: {}", e);
        }

        // Close all writers
        if let Err(e) = self.close_writers() {
            error!("Failed to close writers: {}", e);
        }

        info!(
            "Reader finished. Total messages: {}, errors: {}",
            message_count, error_count
        );

        Ok(())
    }

    /// Run the reader with cancellation support
    pub async fn run_with_cancellation(
        &self,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Result<()> {
        info!("Starting Bybit WebSocket reader for symbol: {}", self.config.symbol);
        info!("Flush interval: {} seconds", self.config.interval_seconds);
        info!(
            "Duration: {} seconds",
            if self.config.duration_seconds > 0 {
                self.config.duration_seconds.to_string()
            } else {
                "infinite".to_string()
            }
        );
        info!(
            "Parquet output: {}",
            if self.config.save_parquet {
                "enabled"
            } else {
                "disabled"
            }
        );

        // Initialize storage writers
        {
            let mut writers_guard = self.writers.lock().unwrap();
            *writers_guard = self.init_writers()?;
        }

        // Connect to WebSocket
        let ws_url = self.get_ws_url();
        info!("Connecting to WebSocket: {}", ws_url);

        let (ws_stream, _response) = connect_async(ws_url)
            .await
            .context("Failed to connect to WebSocket")?;

        info!("WebSocket connected successfully");

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to orderbook
        let subscribe_msg = WsRequest::subscribe(vec![self.config.symbol.clone()], self.config.depth);
        let subscribe_text = serde_json::to_string(&subscribe_msg)?;
        ws_sender
            .send(Message::Text(subscribe_text))
            .await
            .context("Failed to send subscribe message")?;

        info!("Subscribed to orderbook for {}", self.config.symbol);

        let mut message_count = 0u64;
        let mut error_count = 0u64;
        let mut last_ping = Instant::now();
        let mut flush_interval = interval(Duration::from_secs(self.config.interval_seconds));
        flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            // Check if we should stop due to cancellation
            if cancel_token.is_cancelled() {
                info!("Cancellation requested, stopping reader");
                break;
            }

            // Check if we should stop due to duration
            if self.config.duration_seconds > 0 {
                let elapsed = self.start_time.elapsed().unwrap().as_secs();
                if elapsed >= self.config.duration_seconds {
                    info!("Duration reached, stopping reader");
                    break;
                }
            }

            // Send ping every 20 seconds
            if last_ping.elapsed() >= Duration::from_secs(20) {
                let ping_msg = WsRequest::ping();
                let ping_text = serde_json::to_string(&ping_msg)?;
                if let Err(e) = ws_sender.send(Message::Text(ping_text)).await {
                    warn!("Failed to send ping: {}", e);
                }
                last_ping = Instant::now();
            }

            tokio::select! {
                // Handle WebSocket messages
                Some(msg) = ws_receiver.next() => {
                    match msg {
                        Ok(Message::Text(text)) => {
                            match serde_json::from_str::<WsResponse>(&text) {
                                Ok(response) => {
                                    // Handle subscription confirmation
                                    if let Some(op) = &response.op {
                                        if op == "subscribe" {
                                            if response.success == Some(true) {
                                                info!("Subscription confirmed");
                                            } else {
                                                warn!("Subscription failed: {:?}", response.ret_msg);
                                            }
                                        } else if op == "pong" {
                                            debug!("Received pong");
                                        }
                                    }

                                    // Handle orderbook data
                                    if let Some(data) = response.data {
                                        // Only process if this is an orderbook update (has topic)
                                        if response.topic.is_some() {
                                            message_count += 1;

                                            let fetch_time = SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_millis() as i64;

                                            let orderbook_data = OrderbookData {
                                                symbol: data.s,
                                                bids: data.b,
                                                asks: data.a,
                                                timestamp: response.ts.unwrap_or(fetch_time),
                                                update_id: data.u,
                                                fetch_time,
                                            };

                                            // Write to storage
                                            if let Err(e) = self.write_data(&orderbook_data) {
                                                error!("Failed to write data: {}", e);
                                                error_count += 1;
                                            }

                                            if message_count % 100 == 0 {
                                                info!(
                                                    "Processed {} orderbook messages, {} errors",
                                                    message_count, error_count
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("Failed to parse message: {} - Text: {}", e, text);
                                }
                            }
                        }
                        Ok(Message::Close(_)) => {
                            info!("WebSocket closed by server");
                            break;
                        }
                        Ok(Message::Ping(data)) => {
                            debug!("Received ping, sending pong");
                            if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                                warn!("Failed to send pong: {}", e);
                            }
                        }
                        Ok(_) => {
                            // Ignore other message types
                        }
                        Err(e) => {
                            error!("WebSocket error: {}", e);
                            error_count += 1;
                            
                            // If too many errors, try to reconnect
                            if error_count % 10 == 0 {
                                error!("Too many errors, stopping");
                                break;
                            }
                        }
                    }
                }
                
                // Periodic flush based on interval_seconds
                _ = flush_interval.tick() => {
                    // First flush buffered data
                    if let Err(e) = self.flush_data() {
                        error!("Failed to flush data: {}", e);
                    }
                    
                    // Then flush writers
                    let mut writers_guard = self.writers.lock().unwrap();
                    for writer in writers_guard.iter_mut() {
                        if let Err(e) = writer.flush() {
                            error!("Failed to flush {}: {}", writer.file_extension(), e);
                        }
                    }
                    debug!("Flushed writers after {} seconds", self.config.interval_seconds);
                }
                
                // Check for cancellation
                _ = cancel_token.cancelled() => {
                    info!("Cancellation requested during operation");
                    break;
                }
            }
        }

        // Close WebSocket connection
        if let Err(e) = ws_sender.close().await {
            warn!("Failed to close WebSocket: {}", e);
        }

        // Flush any remaining buffered data
        if let Err(e) = self.flush_data() {
            error!("Failed to flush remaining data: {}", e);
        }

        // Close all writers
        if let Err(e) = self.close_writers() {
            error!("Failed to close writers: {}", e);
        }

        info!(
            "Reader finished. Total messages: {}, errors: {}",
            message_count, error_count
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ReaderConfig::default();
        assert_eq!(config.symbol, "ETHUSDT");
        assert_eq!(config.interval_seconds, 10);
        assert_eq!(config.output_dir, "./data");
        assert!(!config.testnet);
        assert_eq!(config.depth, 50);
        assert_eq!(config.duration_seconds, 3600);
        assert!(config.save_parquet);
    }
}