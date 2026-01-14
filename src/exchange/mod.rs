//! Exchange connectivity abstraction.
//!
//! This module provides a trait-based abstraction for connecting to
//! different cryptocurrency exchanges. Currently supports:
//! - Bybit (WebSocket orderbook streaming)
//! - Binance Futures USDT-M (WebSocket orderbook streaming)
//!
//! To add a new exchange:
//! 1. Create `src/exchange/new_exchange/` directory with `mod.rs`, `reader.rs`, `models.rs`
//! 2. Implement `ExchangeConnector` trait
//! 3. Add variant to `ExchangeType` enum
//! 4. Export in this mod.rs

pub mod binance;
pub mod bybit;
pub mod connector;

pub use connector::{
    ExchangeConfig, ExchangeConnector, ExchangeError, ExchangeResult, ExchangeType,
};

// Re-export Bybit types at exchange level
pub use bybit::{BybitReader, ReaderConfig};

// Re-export Binance types at exchange level
pub use binance::BinanceReader;
