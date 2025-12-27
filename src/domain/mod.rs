//! Domain layer containing business logic.
//!
//! This module provides:
//! - `indicator` - Reusable technical indicators (VWAP, Volatility, Momentum, OBI)
//! - `model` - Unified domain models (Position, Side)

pub mod indicator;
pub mod model;

pub use indicator::{Vwap, VolatilityDetector, MomentumDetector, OrderBookImbalance};
pub use model::{Position, Side};
