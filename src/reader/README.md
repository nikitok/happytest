# Bybit Reader Module

This module provides functionality to fetch and save orderbook data from Bybit API in both JSONL and Apache Parquet formats.

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
- Parquet: enabled (saves both JSONL and Parquet)

### Run with custom parameters using the CLI
```bash
cargo run --release --bin reader -- --symbol BTCUSDT --duration 100
```

## Module Structure

- `bybit.rs` - Core reader implementation with Bybit API client
- `converter.rs` - Utility to convert reader format to backtest format
- `mod.rs` - Module exports

## Output Format

Files are saved with the same base name in two formats:
- JSONL: `./data/{SYMBOL}_{YYYYMMDD_HHMMSS}_{duration}_{network}.jsonl`
- Parquet: `./data/{SYMBOL}_{YYYYMMDD_HHMMSS}_{duration}_{network}.parquet`

### JSONL Format
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

### Parquet Format
The Parquet file contains the same data in columnar format with:
- Snappy compression for efficient storage
- Schema: symbol (UTF8), bids (JSON string), asks (JSON string), timestamp (Int64), update_id (Int64), fetch_time (Int64)
- Optimized for analytical queries and long-term storage
- ~50-70% smaller than JSONL format

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
        save_parquet: true, // Enable Parquet output
    };
    
    // Create and run reader
    let reader = BybitReader::new(config)?;
    reader.run().await?;
    
    Ok(())
}
```

## Features

- **Dual format output**: Saves both JSONL and Apache Parquet simultaneously
- **Automatic retry** on API errors
- **Graceful shutdown** with Ctrl+C
- **Progress logging** every 60 fetches
- **Configurable parameters** for all aspects
- **Testnet support** for testing
- **Buffered writes** for performance
- **Efficient Parquet batching**: Writes every 100 records for optimal performance
- **Snappy compression** for Parquet files

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
- Typical file sizes per hour:
  - JSONL: ~20-100 MB depending on market activity
  - Parquet: ~10-50 MB (50-70% compression ratio)
- Parquet writes are batched (100 records) for optimal performance

## Command Line Options

```bash
cargo run --release --bin reader -- [OPTIONS]
```

- `-s, --symbol <SYMBOL>`: Trading symbol (default: ETHUSDT)
- `-i, --interval <SECONDS>`: Fetch interval in seconds (default: 1)
- `-d, --duration <SECONDS>`: Duration in seconds, 0 for infinite (default: 3600)
- `-o, --output <DIR>`: Output directory (default: ./data)
- `--testnet`: Use testnet API instead of mainnet
- `--depth <DEPTH>`: Orderbook depth (default: 50)
- `--parquet <BOOL>`: Save as Parquet in addition to JSONL (default: true)

### Examples

```bash
# Disable Parquet output (JSONL only)
cargo run --release --bin reader -- --parquet false

# Fetch BTCUSDT for 2 hours with 2-second intervals
cargo run --release --bin reader -- --symbol BTCUSDT --duration 7200 --interval 2

# Use testnet with deeper orderbook
cargo run --release --bin reader -- --testnet --depth 100
```