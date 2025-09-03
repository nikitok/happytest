use clap::Parser;
use env_logger;
use happytest::reader::{BybitReader, ReaderConfig};
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
#[command(
    name = "bybit-reader",
    about = "Fetch orderbook data from Bybit API",
    version,
    author
)]
struct Args {
    /// Symbol to fetch data for (e.g., "BTCUSDT", "ETHUSDT")
    #[arg(short, long)]
    symbol: String,
    
    /// Interval in seconds between data fetches
    #[arg(short, long, default_value_t = 10)]
    interval: u64,
    
    /// Duration to run in seconds (0 for infinite)
    #[arg(short, long, default_value_t = 60)]
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
    
    /// Save as JSONL format
    #[arg(long, default_value_t = false)]
    jsonl: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    
    let args = Args::parse();
    
    // Validate symbol is not empty
    if args.symbol.trim().is_empty() {
        eprintln!("Error: Symbol cannot be empty. Please provide a valid trading pair (e.g., BTCUSDT, ETHUSDT)");
        std::process::exit(1);
    }
    
    println!("=== Bybit Orderbook Reader ===");
    println!("Symbol: {}", args.symbol);
    println!("Interval: {} seconds", args.interval);
    println!("Duration: {} seconds", if args.duration > 0 { args.duration.to_string() } else { "infinite".to_string() });
    println!("Output: {}", args.output);
    println!("Network: {}", if args.testnet { "testnet" } else { "mainnet" });
    println!("Depth: {}", args.depth);
    println!("Parquet: {}", if args.parquet { "enabled" } else { "disabled" });
    println!("JSONL: {}", if args.jsonl { "enabled" } else { "disabled" });
    println!("==============================\n");
    
    let config = ReaderConfig {
        symbol: args.symbol,
        interval_seconds: args.interval,
        output_dir: args.output,
        testnet: args.testnet,
        depth: args.depth,
        duration_seconds: args.duration,
        save_parquet: args.parquet,
        save_jsonl: args.jsonl,
    };
    
    let reader = BybitReader::new(config)?;
    
    // Create a cancellation token for graceful shutdown
    let cancel_token = CancellationToken::new();
    let cancel_clone = cancel_token.clone();
    
    // Handle Ctrl+C gracefully
    let reader_handle = tokio::spawn(async move {
        if let Err(e) = reader.run_with_cancellation(cancel_clone).await {
            eprintln!("Reader error: {}", e);
        }
    });
    
    // Set up Ctrl+C handler
    let ctrl_c = tokio::signal::ctrl_c();
    
    // Wait for the reader to complete or Ctrl+C
    tokio::select! {
        result = reader_handle => {
            match result {
                Ok(_) => println!("\nReader completed"),
                Err(e) => eprintln!("Reader task error: {}", e),
            }
        }
        _ = ctrl_c => {
            println!("\nReceived interrupt signal, shutting down...");
            // Signal the reader to stop
            cancel_token.cancel();
            // Give the reader some time to clean up
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
    
    Ok(())
}