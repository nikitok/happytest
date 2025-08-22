pub mod core;
pub mod strategy;
pub mod backtest;
pub mod data;
pub mod trading;
pub mod config;
pub mod pnl;

// Re-export commonly used types
pub use core::{
    Trade, OrderBook, PnLResult, ClosedTrade, CapitalMetrics,
    TradeState, TradeError, Result, DataSource, TradeExecutor, ExecutionStats
};
pub use strategy::{Strategy, GptMarketMaker, GptMarketMakerConfig};
pub use backtest::{TradeDashboard, BacktestEngine};
pub use data::{FileDataSource, OrderBookMessage};
pub use trading::{TradeEmitter, BacktestTradeEmitter, BacktestConfig};
pub use config::{AppConfig, validate_config};

