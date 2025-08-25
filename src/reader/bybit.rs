use anyhow::{Context, Result};
use chrono::Local;
use log::{debug, error, info, warn};
use reqwest;
use std::fs::create_dir_all;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

// Import models and storage
use super::models::{BybitResponse, OrderbookData};
use super::storage::{StorageWriter, WriterConfig, JsonlWriter, ParquetWriter};

/// Configuration for the Bybit reader
#[derive(Debug, Clone)]
pub struct ReaderConfig {
    /// Symbol to fetch data for (e.g., "BTCUSDT", "ETHUSDT")
    pub symbol: String,
    /// Interval in seconds between data fetches
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
            interval_seconds: 1,
            output_dir: "./data".to_string(),
            testnet: false,
            depth: 50,
            duration_seconds: 3600, // 1 hour by default
            save_parquet: true,     // Enable Parquet by default
        }
    }
}

/// Bybit data reader
pub struct BybitReader {
    config: ReaderConfig,
    client: reqwest::Client,
    writers: Arc<Mutex<Vec<Box<dyn StorageWriter>>>>,
    start_time: SystemTime,
}

impl BybitReader {
    /// Create a new Bybit reader with the given configuration
    pub fn new(config: ReaderConfig) -> Result<Self> {
        // Create output directory if it doesn't exist
        create_dir_all(&config.output_dir).context("Failed to create output directory")?;

        // Create HTTP client
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            config,
            client,
            writers: Arc::new(Mutex::new(Vec::new())),
            start_time: SystemTime::now(),
        })
    }

    /// Get the base URL for the API
    fn get_base_url(&self) -> &'static str {
        if self.config.testnet {
            "https://api-testnet.bybit.com"
        } else {
            "https://api.bybit.com"
        }
    }

    /// Generate base filename for output files
    fn generate_base_filename(&self) -> String {
        let now = Local::now();
        let date_str = now.format("%Y%m%d_%H%M%S").to_string();
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
            buffer_size: 100,
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

    /// Fetch orderbook data from Bybit API
    async fn fetch_orderbook(&self) -> Result<OrderbookData> {
        let url = format!("{}/v5/market/orderbook", self.get_base_url());

        let response = self
            .client
            .get(&url)
            .query(&[
                ("category", "linear"),
                ("symbol", &self.config.symbol),
                ("limit", &self.config.depth.to_string()),
            ])
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("API request failed: {} - {}", status, text));
        }

        let bybit_response: BybitResponse =
            response.json().await.context("Failed to parse response")?;

        if bybit_response.ret_code != 0 {
            return Err(anyhow::anyhow!(
                "API error: {} - {}",
                bybit_response.ret_code,
                bybit_response.ret_msg
            ));
        }

        let fetch_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Ok(OrderbookData {
            symbol: bybit_response.result.s,
            bids: bybit_response.result.b,
            asks: bybit_response.result.a,
            timestamp: bybit_response.result.ts,
            update_id: bybit_response.result.u,
            fetch_time,
        })
    }

    /// Write data to all storage writers
    fn write_data(&self, data: &OrderbookData) -> Result<()> {
        let mut writers_guard = self.writers.lock().unwrap();

        for writer in writers_guard.iter_mut() {
            if let Err(e) = writer.write(data) {
                error!("Failed to write data to {}: {}", writer.file_extension(), e);
            }
        }

        // Flush writers periodically
        for writer in writers_guard.iter_mut() {
            if let Err(e) = writer.flush() {
                error!("Failed to flush {}: {}", writer.file_extension(), e);
            }
        }

        debug!(
            "Wrote orderbook data: {} bids, {} asks",
            data.bids.len(),
            data.asks.len()
        );

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

    /// Run the reader
    pub async fn run(&self) -> Result<()> {
        info!("Starting Bybit reader for symbol: {}", self.config.symbol);
        info!("Interval: {} seconds", self.config.interval_seconds);
        info!(
            "Duration: {} seconds",
            if self.config.duration_seconds > 0 {
                self.config.duration_seconds.to_string()
            } else {
                "infinite".to_string()
            }
        );
        info!("Parquet output: {}", if self.config.save_parquet { "enabled" } else { "disabled" });

        // Initialize storage writers
        {
            let mut writers_guard = self.writers.lock().unwrap();
            *writers_guard = self.init_writers()?;
        }

        let mut fetch_count = 0u64;
        let mut error_count = 0u64;

        loop {
            // Check if we should stop
            if self.config.duration_seconds > 0 {
                let elapsed = self.start_time.elapsed().unwrap().as_secs();
                if elapsed >= self.config.duration_seconds {
                    info!("Duration reached, stopping reader");
                    break;
                }
            }

            // Fetch orderbook data
            match self.fetch_orderbook().await {
                Ok(data) => {
                    fetch_count += 1;

                    // Write to storage
                    if let Err(e) = self.write_data(&data) {
                        error!("Failed to write data: {}", e);
                        error_count += 1;
                    }

                    if fetch_count % 60 == 0 {
                        info!(
                            "Fetched {} orderbook snapshots, {} errors",
                            fetch_count, error_count
                        );
                    }
                }
                Err(e) => {
                    error!("Failed to fetch orderbook: {}", e);
                    error_count += 1;

                    // If too many consecutive errors, wait a bit longer
                    if error_count % 10 == 0 {
                        warn!("Multiple errors occurred, waiting 5 seconds...");
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            }

            // Wait for the next interval
            sleep(Duration::from_secs(self.config.interval_seconds)).await;
        }

        // Close all writers
        if let Err(e) = self.close_writers() {
            error!("Failed to close writers: {}", e);
        }

        info!(
            "Reader finished. Total fetches: {}, errors: {}",
            fetch_count, error_count
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
        assert_eq!(config.interval_seconds, 1);
        assert_eq!(config.output_dir, "./data");
        assert!(!config.testnet);
        assert_eq!(config.depth, 50);
        assert_eq!(config.duration_seconds, 3600);
        assert!(config.save_parquet);
    }
}

