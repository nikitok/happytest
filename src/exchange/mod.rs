//! Exchange connectivity abstraction.
//!
//! This module provides a trait-based abstraction for connecting to
//! different cryptocurrency exchanges. Currently supports:
//! - Bybit (via existing reader module)
//!
//! To add a new exchange:
//! 1. Create `src/exchange/new_exchange.rs` implementing `ExchangeConnector`
//! 2. Add variant to `ExchangeType` enum
//! 3. Export in this mod.rs

pub mod connector;

pub use connector::{
    ExchangeConnector, ExchangeConfig, ExchangeError, ExchangeResult, ExchangeType,
};

// Re-export Bybit from reader module for backwards compatibility
pub use crate::reader::{BybitReader, ReaderConfig};
