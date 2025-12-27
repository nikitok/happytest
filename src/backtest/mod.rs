pub mod trade_dashboard;
pub mod engine;
pub mod executor;

pub use trade_dashboard::TradeDashboard;
pub use engine::BacktestEngine;
pub use executor::{TradeEmitter, BacktestTradeEmitter, BacktestConfig};