pub mod executor;
pub mod position;
pub mod metrics;

pub use executor::{TradeEmitter, BacktestTradeEmitter, BacktestConfig};
pub use position::{Position, PositionTracker};
pub use metrics::{TradingMetrics, MetricsCalculator};