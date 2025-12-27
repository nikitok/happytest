//! Technical indicators for trading strategies.
//!
//! This module provides reusable indicators that can be composed
//! into trading strategies. Each indicator is independent and testable.

pub mod vwap;
pub mod volatility;
pub mod momentum;
pub mod obi;

pub use vwap::Vwap;
pub use volatility::VolatilityDetector;
pub use momentum::MomentumDetector;
pub use obi::OrderBookImbalance;
