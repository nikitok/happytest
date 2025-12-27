@AGENTS.md

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

High-frequency trading backtest engine written in Rust. Supports market-making strategies with orderbook data from Bybit exchange. Two binaries: `happytest` (backtest engine) and `reader` (live data collection).

## Build/Test Commands

```bash
cargo build --release          # Build optimized binaries
cargo test                     # Run all tests
cargo test pnl::tests          # Run specific test module
cargo check                    # Fast syntax/type check
cargo clippy                   # Linting
cargo fmt                      # Format code
RUST_LOG=info cargo run --release -- --file data.parquet gpt  # Run with logging
```

### Backtest Examples

```bash
# Single file with GPT strategy
cargo run --release -- --file data/BTCUSDT.parquet gpt --spread 0.001

# Regex pattern matching multiple files (aggregated as continuous range)
cargo run --release -- --file "BTCUSDT_202509.*\.parquet" -d ./data gpt

# Parallel processing (separate backtests per file)
cargo run --release -- --file "*.parquet" -d ./data --parallel --workers 4 gpt

# Simple market maker with custom parameters
cargo run --release -- --file data.jsonl simple --spread 0.002 --volume 0.05
```

### Data Collection (reader binary)

```bash
# Collect orderbook data from Bybit
cargo run --release --bin reader -- --symbol BTCUSDT --interval 1 --duration 3600 --parquet
```

## Architecture

### Data Flow

```
OrderBook Data → DataSource trait → BacktestEngine → Strategy.propose_trade()
    → TradeEmitter.execute_trade() → TradeState → TradeDashboard/PnlReport
```

### Key Abstractions

**`DataSource` trait** (`src/core/traits.rs`): Provides orderbook updates. Implementations:
- `FileDataSource` - JSONL files
- `ParquetDataSource` - Parquet files (faster)
- `MultiFileDataSource` - Multiple files as continuous stream

**`Strategy` trait** (`src/strategy/base.rs`): Trading logic interface. Must implement:
- `propose_trade(&OrderBook) -> Option<Trade>` - Generate trade signals
- `update_position(&Trade, filled: bool)` - Track position state

**`TradeEmitter` trait** (`src/trading/executor.rs`): Trade execution simulation with fill rates, slippage, and rejections.

**Technical Indicators** (`src/domain/indicator/`): Reusable components for strategies:
- `Vwap` - Volume Weighted Average Price with sliding window
- `VolatilityDetector` - Rolling volatility with cooldown logic
- `MomentumDetector` - Price momentum with cooldown logic
- `OrderBookImbalance` - Bid/ask volume imbalance calculation

**Domain Models** (`src/domain/model/`):
- `Position` - Unified position representation with P&L calculations
- `Side` - Type-safe enum for Buy/Sell with conversions

### Module Organization

- **`core/`** - Fundamental types: `Trade`, `OrderBook`, `TradeState`, `PnLResult`
- **`domain/`** - Business logic layer:
  - `indicator/` - Reusable indicators: `Vwap`, `VolatilityDetector`, `MomentumDetector`, `OrderBookImbalance`
  - `model/` - Unified domain models: `Position`, `Side`
- **`strategy/`** - Trading strategies (GPT market maker, Simple market maker)
- **`backtest/`** - `BacktestEngine` orchestration, `TradeDashboard` analytics
- **`pnl/`** - P&L calculation with FIFO/Position methods, commission handling
- **`trading/`** - Execution simulation, position tracking, metrics
- **`reader/`** - Bybit WebSocket data collection, Parquet/JSONL writers
- **`utils/`** - Data loaders, file source implementations
- **`config/`** - Configuration defaults and validation

### Adding a New Strategy

1. Create `src/strategy/my_strategy.rs` implementing `Strategy` trait
2. Add CLI args struct in `src/strategy/args.rs` with `StrategyArgs` trait
3. Add variant to `StrategyCommand` enum in `src/main.rs`
4. Export in `src/strategy/mod.rs`

### PnL Calculation

Two methods via `Processor` trait:
- **FIFO** (`FifoProcessor`): First-in-first-out matching of buys/sells
- **Position** (`PositionProcessor`): Net position weighted average

Commission rate defaults to 0.03% (configurable via `PnlReport::with_commission()`).

## Key Data Structures

```rust
// Core orderbook (src/core/models.rs)
struct OrderBook {
    symbol: String,
    bids: Vec<(f64, f64)>,  // (price, quantity)
    asks: Vec<(f64, f64)>,
    current_time: i64,
}

// Trade with lifecycle status
struct Trade {
    id: String,           // UUID
    symbol: String,
    side: String,         // "buy" or "sell"
    price: f64,
    quantity: f64,
    status: String,       // "pending", "filled", "rejected"
    time: i64,
}

// Backtest simulation config
struct BacktestConfig {
    fill_rate: f64,       // 0.0-1.0
    slippage_bps: f64,    // Basis points
    rejection_rate: f64,
    margin_rate: f64,
}

// Unified position model (src/domain/model/position.rs)
enum Side { Buy, Sell }

struct Position {
    symbol: String,
    quantity: f64,
    entry_price: f64,
    side: Side,
    entry_time: i64,
}
```

## Code Patterns

- Use `Result<T, TradeError>` (via `crate::core::Result`) for fallible operations
- Prefer `&str` over `String` in hot paths
- Use `VecDeque` for sliding windows (VWAP, volatility tracking)
- Data sources implement `Send` for potential parallelization
- Strategies implement `Send + Sync` for thread safety
