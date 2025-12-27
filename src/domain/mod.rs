//! Domain layer containing business logic.
//!
//! This module provides:
//! - `indicator` - Reusable technical indicators (VWAP, Volatility, Momentum, OBI)
//!
//! Future additions:
//! - `model` - Unified domain models (Position, Trade, OrderBook)

pub mod indicator;

pub use indicator::{Vwap, VolatilityDetector, MomentumDetector, OrderBookImbalance};
