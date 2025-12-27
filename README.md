# HappyTest

> High-performance framework for HFT data acquisition, analysis, and backtesting

![Performance Analysis](./data/BTCUSDT_300_graphcombined.png)

## What is HappyTest?

HappyTest is a Rust-based framework designed for high-frequency trading research. It provides fast, memory-efficient tools for processing orderbook data, implementing trading strategies, and analyzing performance metrics at microsecond precision.

### Core Capabilities

- **Data Processing**: Stream JSONL/Parquet orderbook snapshots with minimal memory footprint
- **Strategy Testing**: Modular architecture for market making, arbitrage, and custom algorithms  
- **Performance Analysis**: Real-time PnL tracking, position management, and risk metrics
- **Production Ready**: Sub-second processing of 12,000+ orderbook messages

## Architecture

```
src/
├── core/               # Fundamental types: Trade, OrderBook, TradeState, PnLResult
├── domain/             # Business logic layer
│   ├── indicator/      # Reusable indicators: Vwap, VolatilityDetector, MomentumDetector
│   └── model/          # Unified domain models: Position, Side
├── strategy/           # Trading strategies (GPT market maker)
├── backtest/           # Backtesting framework
│   ├── engine.rs       # BacktestEngine orchestration
│   ├── trade_dashboard.rs  # TradeDashboard analytics
│   └── executor.rs     # TradeEmitter execution simulation
├── analytics/          # Analysis and reporting
│   ├── pnl/            # P&L calculation (FIFO/Position methods)
│   └── metrics.rs      # Trading metrics: Sharpe ratio, drawdown, win rate
├── exchange/           # Exchange connectivity
│   ├── bybit/          # Bybit WebSocket reader, models
│   └── connector.rs    # ExchangeConnector trait
├── storage/            # Data storage abstraction
│   ├── source/         # Data reading: FileDataSource, ParquetDataSource
│   └── sink/           # Data writing: JsonlWriter, ParquetWriter
├── reader/             # Reader binary support, data models
├── config/             # Configuration defaults and validation
└── utils/              # (Deprecated) Legacy data loaders
```

### Key Modules

**`core`** - Trade models, orderbook structures, state management
**`domain`** - Technical indicators (VWAP, volatility, momentum) and position models
**`strategy`** - Trait-based strategy interface with GPT market maker
**`backtest`** - Engine, dashboard, and trade execution simulation
**`analytics`** - P&L calculation and trading metrics (Sharpe, drawdown)
**`exchange`** - Exchange connectivity with Bybit WebSocket support
**`storage`** - Unified data I/O for JSONL and Parquet formats

## Quick Start

```bash
# Build optimized binary
cargo build --release

# Run GPT market maker on sample data
cargo run --release --bin happytest -- --file data/BTCUSDT.parquet gpt
```

## Live Data Collection & Backtest

### Step 1: Collect live orderbook data from Bybit

```bash
# Collect BTCUSDT data for 3 minutes (180 seconds)
cargo run --release --bin reader -- \
  --symbol BTCUSDT \
  --duration 180 \
  --interval 5 \
  --parquet \
  --output ./data
```

**Reader options:**
| Option | Description | Default |
|--------|-------------|---------|
| `--symbol` | Trading pair (BTCUSDT, ETHUSDT, etc.) | required |
| `--duration` | Collection time in seconds (0 = infinite) | 60 |
| `--interval` | Flush interval in seconds | 10 |
| `--parquet` | Save as Parquet format | false |
| `--jsonl` | Save as JSONL format | false |
| `--output` | Output directory | ./data |
| `--depth` | Orderbook depth | 50 |
| `--testnet` | Use Bybit testnet | false |

### Step 2: Run backtest on collected data

```bash
cargo run --release --bin happytest -- \
  --file ./data/BTCUSDT_*.parquet \
  gpt
```

### Step 3: Understanding the results

The backtest outputs a P&L summary table:

```
=== P&L Summary by Symbol ===
+-----------+-----------+---------------+----------------+------------------+---------------+
| Symbol    | Trades    | Realized P&L  | Unrealized P&L | Remaining Shares | Total P&L     |
+===========================================================================================+
| BTCUSDT   | 418       | $62.32        | $80.87         | -1               | $143.19       |
+-----------+-----------+---------------+----------------+------------------+---------------+
```

**Key metrics explained:**

| Metric | Description |
|--------|-------------|
| **Trades** | Total number of trades executed |
| **Realized P&L** | Profit/loss from closed positions (actual gains) |
| **Unrealized P&L** | Paper profit/loss from open positions |
| **Remaining Shares** | Net position (positive = long, negative = short) |
| **Total P&L** | Realized + Unrealized P&L |

**Output files:**
- `*_graphBTCUSDT.png` - P&L chart for the symbol
- `*_graphcombined.png` - Combined P&L chart with all metrics

### Advanced backtest options

```bash
# Custom execution parameters
cargo run --release --bin happytest -- \
  --file data/BTCUSDT.parquet \
  --fill-rate 0.95 \
  --slippage-bps 1.0 \
  --rejection-rate 0.02 \
  gpt \
  --spread 0.001 \
  --take-profit-bps 30

# View all strategy options
cargo run --release --bin happytest -- gpt --help
```

| Backtest Option | Description | Default |
|-----------------|-------------|---------|
| `--fill-rate` | Order fill probability (0.0-1.0) | 0.98 |
| `--slippage-bps` | Slippage in basis points | 2.0 |
| `--rejection-rate` | Order rejection probability | 0.01 |
| `--margin-rate` | Margin requirement rate | 0.05 |

## Performance

Benchmarked on commodity hardware:
- **Throughput**: 12,000 orderbook updates/second
- **Latency**: <100μs per strategy tick
- **Memory**: <50MB for 1M orderbook snapshots

## Installation

### Requirements
- Rust 1.60+
- 4GB RAM
- x86_64 or ARM64 processor

### Cross-compilation

```bash
# Apple Silicon
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Linux x86_64
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu

# Static binary (Alpine)
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

### CPU Optimization

```bash
# Native CPU features
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Intel Skylake+
RUSTFLAGS="-C target-cpu=skylake" cargo build --release

# Apple M1/M2
RUSTFLAGS="-C target-cpu=apple-m1" cargo build --release
```

## Usage Examples

### Basic Backtest
```bash
cargo run --release -- --file data/ETHUSDT.jsonl gpt
```

### With Logging
```bash
RUST_LOG=info cargo run --release -- --file data/BTC.parquet gpt
```

### Custom Strategy Parameters
```bash
cargo run --release -- gpt --help  # View all options
```

## API Example

```rust
use happytest::{
    strategy::GPTMarketMaker,
    backtest::Backtester,
    data::JSONLReader,
};

let reader = JSONLReader::new("data/orderbook.jsonl")?;
let strategy = GPTMarketMaker::default();
let mut backtester = Backtester::new(reader, strategy);

backtester.run()?;
backtester.print_dashboard();
```

## Contributing

HappyTest is designed for extensibility:

1. **New Strategies**: Implement `Strategy` trait in `src/strategy/`
2. **Data Formats**: Add readers in `src/data/` 
3. **Metrics**: Extend `TradeDashboard` in `src/backtest/`

## License

MIT