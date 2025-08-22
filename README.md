# HappyTest

**HappyTest** — a high-performance framework for backtesting trading strategies on historical orderbook data.

## Description

HappyTest provides tools for fast and scalable validation of algorithmic strategies (e.g., market making) on real OrderBook data. The project is written in Rust and optimized for high performance and low memory overhead.

## Key Features

- Backtesting of market making and other algorithmic trading strategies
- Streaming processing of JSONL files with orderbook data
- Customizable execution parameters: fill rate, slippage, margin level, order volume, and more
- Collection and display of PnL, positions, and performance metrics
- Extensible trait-based architecture for strategies, data sources, and trade executors
- Strong error typing with `thiserror`
- Clean CLI interface based on `clap` and logging via `log`/`env_logger`

## Quick Start

```bash
# Build in release mode
cd happytest
cargo build --release

# Run backtest on orderbook data file
cargo run --release -- --file path/to/data.jsonl

# Run with INFO level logging
RUST_LOG=info cargo run --release -- --file path/to/data.jsonl
```

## Example Run with Settings

```bash
cargo run --release -- \
  --file data/BTCUSDT_300.jsonl \
  --fill-rate 0.95 \
  --slippage-bps 1.0 \
  --rejection-rate 0.01 \
  --min-spread-pct 0.0005 \
  --margin-rate 0.05 \
  --fix-order-volume 0.005 \
  --spread-percent 0.005
```

## Project Structure

```text
happytest/
├── src/             # Source code (core, data, strategy, backtest, trading, config modules)
├── tests/           # Integration tests
├── benches/         # Performance benchmarks
├── examples/        # Usage examples
├── Cargo.toml       # Package description and dependencies
└── README.md        # Project description
```

## What HappyTest is For

HappyTest enables traders, researchers, and developers to:
- Test trading strategy effectiveness on historical data without connecting to a real market
- Quickly configure and run backtests with various trading parameters
- Collect detailed statistics on PnL and position management
- Extend existing strategies and add new ones thanks to modular design

## Dependencies

- Rust 1.60 or higher
- clap
- serde, serde_json
- chrono
- uuid
- log, env_logger
- thiserror
- comfy-table
