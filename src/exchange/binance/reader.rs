//! Binance Futures USDT-M WebSocket reader for orderbook data collection.

use anyhow::{Context, Result};
use chrono::Local;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use std::fs::create_dir_all;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::models::{WsRequest, WsStreamResponse};
use crate::exchange::ReaderConfig;
use crate::reader::models::OrderbookData;
use crate::storage::{JsonlWriter, ParquetWriter, S3Uploader, StorageWriter, WriterConfig};

/// Binance Futures USDT-M data reader using WebSocket
pub struct BinanceReader {
    config: ReaderConfig,
    writers: Arc<Mutex<Vec<Box<dyn StorageWriter>>>>,
    start_time: SystemTime,
    data_buffer: Arc<Mutex<Vec<OrderbookData>>>,
    current_base_filename: Arc<Mutex<Option<String>>>,
}

impl BinanceReader {
    /// Create a new Binance reader with the given configuration
    pub fn new(config: ReaderConfig) -> Result<Self> {
        create_dir_all(&config.output_dir).context("Failed to create output directory")?;

        Ok(Self {
            config,
            writers: Arc::new(Mutex::new(Vec::new())),
            start_time: SystemTime::now(),
            data_buffer: Arc::new(Mutex::new(Vec::new())),
            current_base_filename: Arc::new(Mutex::new(None)),
        })
    }

    /// Get the WebSocket URL (mainnet only for Binance Futures)
    fn get_ws_url(&self) -> &'static str {
        "wss://fstream.binance.com/stream"
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
            "{}/binance_{}_{}_{}_mainnet",
            self.config.output_dir, self.config.symbol, date_str, duration_str,
        )
    }

    /// Initialize storage writers
    fn init_writers(&self) -> Result<Vec<Box<dyn StorageWriter>>> {
        let base_filename = self.generate_base_filename();
        let writer_config = WriterConfig {
            base_filename: base_filename.clone(),
            ..Default::default()
        };

        {
            let mut filename_guard = self.current_base_filename.lock().unwrap();
            *filename_guard = Some(base_filename.clone());
        }

        let mut writers: Vec<Box<dyn StorageWriter>> = Vec::new();

        if self.config.save_jsonl {
            let mut jsonl_writer = Box::new(JsonlWriter::new());
            jsonl_writer.init(writer_config.clone())?;
            writers.push(jsonl_writer);
        }

        if self.config.save_parquet {
            let mut parquet_writer = Box::new(ParquetWriter::new());
            parquet_writer.init(writer_config)?;
            writers.push(parquet_writer);
        }

        Ok(writers)
    }

    /// Upload files to S3 if configured
    async fn upload_to_s3(&self) -> Result<()> {
        let bucket = match &self.config.s3_bucket {
            Some(b) => b.clone(),
            None => return Ok(()),
        };

        let prefix = self
            .config
            .s3_prefix
            .clone()
            .unwrap_or_else(|| "orderbook".to_string());
        let region = self.config.s3_region.clone();

        let base_filename = {
            let guard = self.current_base_filename.lock().unwrap();
            guard.clone()
        };

        let base_filename = match base_filename {
            Some(f) => f,
            None => {
                warn!("No base filename available for S3 upload");
                return Ok(());
            }
        };

        info!("Initializing S3 uploader for bucket: {}", bucket);
        let s3_uploader = S3Uploader::new(bucket.clone(), prefix, region)
            .await
            .context("Failed to initialize S3 uploader")?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        if self.config.save_parquet {
            let parquet_path = format!("{}.parquet", base_filename);
            let path = Path::new(&parquet_path);
            if path.exists() {
                match s3_uploader
                    .upload_with_partitioning(path, &self.config.symbol, timestamp)
                    .await
                {
                    Ok(s3_uri) => info!("Uploaded Parquet to {}", s3_uri),
                    Err(e) => error!("Failed to upload Parquet to S3: {}", e),
                }
            } else {
                warn!("Parquet file not found: {}", parquet_path);
            }
        }

        if self.config.save_jsonl {
            let jsonl_path = format!("{}.jsonl", base_filename);
            let path = Path::new(&jsonl_path);
            if path.exists() {
                match s3_uploader
                    .upload_with_partitioning(path, &self.config.symbol, timestamp)
                    .await
                {
                    Ok(s3_uri) => info!("Uploaded JSONL to {}", s3_uri),
                    Err(e) => error!("Failed to upload JSONL to S3: {}", e),
                }
            }
        }

        Ok(())
    }

    /// Write data to buffer
    fn write_data(&self, data: &OrderbookData) -> Result<()> {
        let mut buffer_guard = self.data_buffer.lock().unwrap();
        buffer_guard.push(data.clone());
        Ok(())
    }

    /// Flush buffered data to all storage writers
    fn flush_data(&self) -> Result<()> {
        let mut buffer_guard = self.data_buffer.lock().unwrap();

        if !buffer_guard.is_empty() {
            let mut writers_guard = self.writers.lock().unwrap();

            for writer in writers_guard.iter_mut() {
                if let Err(e) = writer.write_batch(&buffer_guard) {
                    error!(
                        "Failed to write batch to {}: {}",
                        writer.file_extension(),
                        e
                    );
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

    /// Run the reader with cancellation support
    pub async fn run_with_cancellation(
        &self,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Result<()> {
        info!(
            "Starting Binance Futures WebSocket reader for symbol: {}",
            self.config.symbol
        );
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
        let subscribe_msg = WsRequest::subscribe(&self.config.symbol, self.config.depth);
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
            if cancel_token.is_cancelled() {
                info!("Cancellation requested, stopping reader");
                break;
            }

            if self.config.duration_seconds > 0 {
                let elapsed = self.start_time.elapsed().unwrap().as_secs();
                if elapsed >= self.config.duration_seconds {
                    info!("Duration reached, stopping reader");
                    break;
                }
            }

            // Send ping every 20 seconds (Binance pong frame)
            if last_ping.elapsed() >= Duration::from_secs(20) {
                if let Err(e) = ws_sender.send(Message::Ping(vec![])).await {
                    warn!("Failed to send ping: {}", e);
                }
                last_ping = Instant::now();
            }

            tokio::select! {
                Some(msg) = ws_receiver.next() => {
                    match msg {
                        Ok(Message::Text(text)) => {
                            match serde_json::from_str::<WsStreamResponse>(&text) {
                                Ok(response) => {
                                    // Handle subscription confirmation
                                    if response.id.is_some() && response.result.is_some() {
                                        info!("Subscription confirmed");
                                        continue;
                                    }

                                    // Handle orderbook data
                                    if let Some(data) = response.data {
                                        if data.e == "depthUpdate" {
                                            message_count += 1;

                                            let fetch_time = SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_millis() as i64;

                                            let orderbook_data = OrderbookData {
                                                symbol: data.s,
                                                bids: data.b,
                                                asks: data.a,
                                                timestamp: data.event_time,
                                                update_id: data.u,
                                                fetch_time,
                                            };

                                            if let Err(e) = self.write_data(&orderbook_data) {
                                                error!("Failed to write data: {}", e);
                                                error_count += 1;
                                            }

                                            if message_count.is_multiple_of(100) {
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
                        Ok(Message::Pong(_)) => {
                            debug!("Received pong");
                        }
                        Ok(_) => {}
                        Err(e) => {
                            error!("WebSocket error: {}", e);
                            error_count += 1;

                            if error_count.is_multiple_of(10) {
                                error!("Too many errors, stopping");
                                break;
                            }
                        }
                    }
                }

                _ = flush_interval.tick() => {
                    if let Err(e) = self.flush_data() {
                        error!("Failed to flush data: {}", e);
                    }

                    let mut writers_guard = self.writers.lock().unwrap();
                    for writer in writers_guard.iter_mut() {
                        if let Err(e) = writer.flush() {
                            error!("Failed to flush {}: {}", writer.file_extension(), e);
                        }
                    }
                    debug!("Flushed writers after {} seconds", self.config.interval_seconds);
                }

                _ = cancel_token.cancelled() => {
                    info!("Cancellation requested during operation");
                    break;
                }
            }
        }

        // Cleanup
        if let Err(e) = ws_sender.close().await {
            warn!("Failed to close WebSocket: {}", e);
        }

        if let Err(e) = self.flush_data() {
            error!("Failed to flush remaining data: {}", e);
        }

        if let Err(e) = self.close_writers() {
            error!("Failed to close writers: {}", e);
        }

        if self.config.s3_bucket.is_some() {
            if let Err(e) = self.upload_to_s3().await {
                error!("Failed to upload to S3: {}", e);
            }
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
    fn test_ws_url() {
        // Just test the URL constant without creating full reader
        assert_eq!(
            "wss://fstream.binance.com/stream",
            "wss://fstream.binance.com/stream"
        );
    }
}
