//! Technical indicators for trading strategies.
//!
//! This module provides reusable indicators that can be composed
//! into trading strategies. Each indicator is independent and testable.

pub mod momentum;
pub mod obi;
pub mod volatility;
pub mod vwap;

pub use momentum::MomentumDetector;
pub use obi::OrderBookImbalance;
pub use volatility::VolatilityDetector;
pub use vwap::Vwap;
