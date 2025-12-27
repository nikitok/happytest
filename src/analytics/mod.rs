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

pub mod pnl;
pub mod metrics;

// Re-export commonly used types from pnl
pub use pnl::{
    Method, Record, PnlReport, Processor,
    FifoProcessor, PositionProcessor,
    calculate_unrealized_pnl,
};

// Re-export metrics types
pub use metrics::{TradingMetrics, MetricsCalculator};
