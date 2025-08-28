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

### Basic usage with GPT Market Maker strategy:
```bash
cargo run --release -- --file /Users/noviiden/java/projects/happytest/data/ETHUSDT_3600.jsonl gpt
```

### With custom backtest parameters:
```bash
cargo run --release -- \
  --file data/BTCUSDT_300.jsonl \
  --fill-rate 0.95 \
  --slippage-bps 1.0 \
  gpt
```

### With custom strategy parameters:
```bash
cargo run --release -- \
  --file data/BTCUSDT_300.jsonl \
  gpt \
  --fix-order-volume 0.01 \
  --take-profit-bps 30 \
  --stop-loss-bps 100
```

### View all available options:
```bash
# View general help
cargo run --release -- --help

# View GPT strategy specific options
cargo run --release -- gpt --help
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

## Cross-Compilation

### Building for Different Architectures

HappyTest can be compiled for various processor architectures using Rust's cross-compilation capabilities.

#### Prerequisites
```bash
# Install cross-compilation tool
cargo install cross
```

#### Common Target Architectures

##### x86_64 Linux (Intel/AMD 64-bit)
```bash
# Using rustup targets
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu

# Or using cross
cross build --release --target x86_64-unknown-linux-gnu
```

##### ARM64/AArch64 (Apple M1/M2, AWS Graviton, Raspberry Pi 4)
```bash
# For Linux ARM64
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu

# For macOS ARM64 (Apple Silicon)
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

##### ARM32 (Raspberry Pi 2/3, embedded systems)
```bash
# ARMv7 with hardware float
rustup target add armv7-unknown-linux-gnueabihf
cargo build --release --target armv7-unknown-linux-gnueabihf

# ARMv6 (Raspberry Pi Zero/1)
rustup target add arm-unknown-linux-gnueabihf
cargo build --release --target arm-unknown-linux-gnueabihf
```

##### MUSL targets (for static linking, Alpine Linux)
```bash
# x86_64 MUSL
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl

# ARM64 MUSL
rustup target add aarch64-unknown-linux-musl
cargo build --release --target aarch64-unknown-linux-musl
```

#### Optimized Build for Specific CPUs

For maximum performance on specific CPU architectures:

```bash
# Intel Skylake and newer
RUSTFLAGS="-C target-cpu=skylake" cargo build --release

# AMD Zen2 (Ryzen 3000 series, EPYC Rome)
RUSTFLAGS="-C target-cpu=znver2" cargo build --release

# ARM Cortex-A72 (Raspberry Pi 4)
RUSTFLAGS="-C target-cpu=cortex-a72" cargo build --release

# Apple M1/M2
RUSTFLAGS="-C target-cpu=apple-m1" cargo build --release --target aarch64-apple-darwin

# Native CPU optimization (best for local machine)
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

#### Build Script Example

Create a `build-all.sh` script for multiple targets:

```bash
#!/bin/bash
# Build for multiple architectures

targets=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-musl"
)

for target in "${targets[@]}"; do
    echo "Building for $target..."
    cross build --release --target "$target"
    cp "target/$target/release/happytest" "dist/happytest-$target"
done
```

#### Verifying the Build

After cross-compilation, verify the binary architecture:

```bash
# On Linux
file target/aarch64-unknown-linux-gnu/release/happytest

# Check dynamic dependencies
ldd target/x86_64-unknown-linux-gnu/release/happytest

# For MUSL builds (should show "statically linked")
ldd target/x86_64-unknown-linux-musl/release/happytest
```

## Dependencies

- Rust 1.60 or higher
- clap
- serde, serde_json
- chrono
- uuid
- log, env_logger
- thiserror
- comfy-table
