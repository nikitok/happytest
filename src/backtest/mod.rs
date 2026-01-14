pub mod engine;
pub mod executor;
pub mod trade_dashboard;

pub use engine::BacktestEngine;
pub use executor::{BacktestConfig, BacktestTradeEmitter, TradeEmitter};
pub use trade_dashboard::TradeDashboard;
