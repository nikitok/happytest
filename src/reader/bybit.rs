use anyhow::{Context, Result};
use chrono::Local;
use log::{debug, error, info, warn};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

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
        }
    }
}

/// Bybit orderbook response structure
#[derive(Debug, Deserialize)]
struct BybitResponse {
    #[serde(rename = "retCode")]
    ret_code: i32,
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: OrderbookResult,
    #[allow(dead_code)]
    time: i64,
}

#[derive(Debug, Deserialize)]
struct OrderbookResult {
    s: String,           // symbol
    b: Vec<[String; 2]>, // bids [price, size]
    a: Vec<[String; 2]>, // asks [price, size]
    ts: i64,             // timestamp
    u: i64,              // update id
}

/// Orderbook data structure to save
#[derive(Debug, Serialize)]
struct OrderbookData {
    symbol: String,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
    timestamp: i64,
    update_id: i64,
    fetch_time: i64,
}

/// Bybit data reader
pub struct BybitReader {
    config: ReaderConfig,
    client: reqwest::Client,
    writer: Arc<Mutex<Option<BufWriter<File>>>>,
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
            writer: Arc::new(Mutex::new(None)),
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

    /// Initialize the output file
    fn init_output_file(&self) -> Result<BufWriter<File>> {
        let now = Local::now();
        let date_str = now.format("%Y%m%d_%H%M%S").to_string();
        let duration_str = if self.config.duration_seconds > 0 {
            format!("{}s", self.config.duration_seconds)
        } else {
            "continuous".to_string()
        };

        let filename = format!(
            "{}/{}_{}_{}_{}.jsonl",
            self.config.output_dir,
            self.config.symbol,
            date_str,
            duration_str,
            if self.config.testnet {
                "testnet"
            } else {
                "mainnet"
            }
        );

        info!("Creating output file: {}", filename);

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&filename)
            .context("Failed to create output file")?;

        Ok(BufWriter::new(file))
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

    /// Write orderbook data to file
    fn write_data(&self, data: &OrderbookData) -> Result<()> {
        let mut writer_guard = self.writer.lock().unwrap();

        if let Some(writer) = writer_guard.as_mut() {
            let json_line =
                serde_json::to_string(data).context("Failed to serialize orderbook data")?;

            writeln!(writer, "{}", json_line).context("Failed to write to file")?;

            writer.flush().context("Failed to flush writer")?;

            debug!(
                "Wrote orderbook data: {} bids, {} asks",
                data.bids.len(),
                data.asks.len()
            );
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

        // Initialize output file
        {
            let mut writer_guard = self.writer.lock().unwrap();
            *writer_guard = Some(self.init_output_file()?);
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

                    // Write to file
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
    }
}

/// Main function to run the reader with default parameters
/// 
/// This allows running the reader directly as:
/// ```bash
/// cd src/reader && cargo run --bin bybit_reader
/// ```
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    // Use default configuration
    let config = ReaderConfig {
        duration_seconds: 60,
        ..ReaderConfig::default()
    };

    fn print_reader_config(config: &ReaderConfig) {
        println!("=== Bybit Orderbook Reader (Config) ===");
        println!("Symbol: {}", config.symbol);
        println!("Interval: {} second{}", config.interval_seconds, if config.interval_seconds == 1 { "" } else { "s" });
        println!("Duration: {} seconds", config.duration_seconds);
        println!("Output: {}", config.output_dir);
        println!("Network: {}", if config.testnet { "testnet" } else { "mainnet" });
        println!("Depth: {}", config.depth);
        println!("==============================================\n");
    }

    print_reader_config(&config);


    // Create and run reader
    let reader = BybitReader::new(config)?;
    
    // Handle Ctrl+C gracefully
    let reader_handle = tokio::spawn(async move {
        if let Err(e) = reader.run().await {
            eprintln!("Reader error: {}", e);
        }
    });
    
    // Wait for the reader to complete or Ctrl+C
    tokio::select! {
        _ = reader_handle => {
            println!("\nReader completed");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\nReceived interrupt signal, shutting down...");
        }
    }
    
    Ok(())
}
