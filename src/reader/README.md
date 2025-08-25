# Bybit Reader Module

This module provides functionality to fetch and save orderbook data from Bybit API.

## Quick Start

### Run with default parameters
```bash
# From project root
cargo run --release --bin bybit_reader

# Or directly from the reader directory
cd src/reader
cargo run --release
```

Default configuration:
- Symbol: ETHUSDT
- Interval: 1 second
- Duration: 3600 seconds (1 hour)
- Output: ./data
- Network: mainnet
- Depth: 50 levels

### Run with custom parameters using the CLI
```bash
cargo run --release --bin reader -- --symbol BTCUSDT --duration 7200
```

## Module Structure

- `bybit.rs` - Core reader implementation with Bybit API client
- `converter.rs` - Utility to convert reader format to backtest format
- `mod.rs` - Module exports

## Output Format

Files are saved as: `./data/{SYMBOL}_{YYYYMMDD_HHMMSS}_{duration}_{network}.jsonl`

Each line in the JSONL file contains:
```json
{
  "symbol": "ETHUSDT",
  "bids": [["3000.50", "1.234"], ["3000.40", "2.345"], ...],
  "asks": [["3000.60", "0.567"], ["3000.70", "1.234"], ...],
  "timestamp": 1705328422123,
  "update_id": 123456789,
  "fetch_time": 1705328422150
}
```

## API Usage

```rust
use happytest::reader::{BybitReader, ReaderConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create custom configuration
    let config = ReaderConfig {
        symbol: "BTCUSDT".to_string(),
        interval_seconds: 2,
        output_dir: "./market_data".to_string(),
        testnet: false,
        depth: 100,
        duration_seconds: 1800, // 30 minutes
    };
    
    // Create and run reader
    let reader = BybitReader::new(config)?;
    reader.run().await?;
    
    Ok(())
}
```

## Features

- **Automatic retry** on API errors
- **Graceful shutdown** with Ctrl+C
- **Progress logging** every 60 fetches
- **Configurable parameters** for all aspects
- **Testnet support** for testing
- **Buffered writes** for performance

## Error Handling

The reader handles various error scenarios:
- Network timeouts (10 second timeout per request)
- API rate limits (automatic backoff after multiple errors)
- File system errors (creates directories if needed)
- Invalid API responses (logs and continues)

## Performance

- Default 1-second interval fetches ~3600 snapshots per hour
- Each snapshot includes full orderbook depth (default 50 levels)
- Files are buffered and flushed after each write
- Typical file size: ~10-50 MB per hour depending on market activity