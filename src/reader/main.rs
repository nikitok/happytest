use clap::Parser;
use env_logger;
use happytest::reader::{BybitReader, ReaderConfig};

#[derive(Parser, Debug)]
#[command(
    name = "bybit-reader",
    about = "Fetch orderbook data from Bybit API",
    version,
    author
)]
struct Args {
    /// Symbol to fetch data for (e.g., "BTCUSDT", "ETHUSDT")
    #[arg(short, long, default_value = "ETHUSDT")]
    symbol: String,
    
    /// Interval in seconds between data fetches
    #[arg(short, long, default_value_t = 1)]
    interval: u64,
    
    /// Duration to run in seconds (0 for infinite)
    #[arg(short, long, default_value_t = 3600)]
    duration: u64,
    
    /// Output directory for data files
    #[arg(short, long, default_value = "./data")]
    output: String,
    
    /// Use testnet API
    #[arg(long)]
    testnet: bool,
    
    /// Orderbook depth to fetch
    #[arg(long, default_value_t = 50)]
    depth: u32,
    
    /// Save as Parquet in addition to JSONL
    #[arg(long, default_value_t = false)]
    parquet: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let args = Args::parse();
    
    println!("=== Bybit Orderbook Reader ===");
    println!("Symbol: {}", args.symbol);
    println!("Interval: {} seconds", args.interval);
    println!("Duration: {} seconds", if args.duration > 0 { args.duration.to_string() } else { "infinite".to_string() });
    println!("Output: {}", args.output);
    println!("Network: {}", if args.testnet { "testnet" } else { "mainnet" });
    println!("Depth: {}", args.depth);
    println!("Parquet: {}", if args.parquet { "enabled" } else { "disabled" });
    println!("==============================\n");
    
    let config = ReaderConfig {
        symbol: args.symbol,
        interval_seconds: args.interval,
        output_dir: args.output,
        testnet: args.testnet,
        depth: args.depth,
        duration_seconds: args.duration,
        save_parquet: args.parquet,
    };
    
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