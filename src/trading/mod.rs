pub mod executor;
pub mod metrics;

pub use executor::{TradeEmitter, BacktestTradeEmitter, BacktestConfig};
pub use metrics::{TradingMetrics, MetricsCalculator};