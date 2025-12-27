pub mod core;
pub mod domain;
pub mod exchange;
pub mod storage;
pub mod strategy;
pub mod backtest;
pub mod utils;  // Deprecated: use storage instead
pub mod trading;
pub mod config;
pub mod pnl;
pub mod reader;

// Re-export commonly used types
pub use core::{
    Trade, OrderBook, PnLResult, ClosedTrade, CapitalMetrics,
    TradeState, TradeError, Result, DataSource, TradeExecutor, ExecutionStats
};
pub use domain::{Vwap, VolatilityDetector, MomentumDetector, OrderBookImbalance, Position, Side};
pub use exchange::{ExchangeConnector, ExchangeConfig, ExchangeError, ExchangeType};
pub use strategy::{Strategy, GptMarketMaker, GptMarketMakerConfig};
pub use backtest::{TradeDashboard, BacktestEngine};
pub use utils::{FileDataSource, ParquetDataSource, OrderBookMessage};
pub use trading::{TradeEmitter, BacktestTradeEmitter, BacktestConfig};
pub use config::{AppConfig, validate_config};

