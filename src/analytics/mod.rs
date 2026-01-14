//! Analytics module for P&L calculation and trading metrics.
//!
//! This module provides comprehensive analytics capabilities:
//! - P&L calculation using FIFO and Position methods
//! - Trading metrics (Sharpe ratio, win rate, drawdown)
//! - Unrealized P&L tracking
//!
//! ## Usage
//! ```rust,ignore
//! use happytest::analytics::pnl::{PnlReport, FifoProcessor};
//! use happytest::analytics::metrics::{MetricsCalculator, TradingMetrics};
//! ```

pub mod metrics;
pub mod pnl;

// Re-export commonly used types from pnl
pub use pnl::{
    calculate_unrealized_pnl, FifoProcessor, Method, PnlReport, PositionProcessor, Processor,
    Record,
};

// Re-export metrics types
pub use metrics::{MetricsCalculator, TradingMetrics};
