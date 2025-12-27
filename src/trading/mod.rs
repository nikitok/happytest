pub mod executor;

pub use executor::{TradeEmitter, BacktestTradeEmitter, BacktestConfig};

// Re-export metrics from analytics for backwards compatibility
pub use crate::analytics::{TradingMetrics, MetricsCalculator};