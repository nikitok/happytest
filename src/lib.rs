pub mod analytics;
pub mod backtest;
pub mod config;
pub mod core;
pub mod domain;
pub mod exchange;
pub mod reader;
pub mod storage;
pub mod strategy;
pub mod utils; // Deprecated: use storage instead

// Re-export commonly used types
pub use backtest::{
    BacktestConfig, BacktestEngine, BacktestTradeEmitter, TradeDashboard, TradeEmitter,
};
pub use config::{validate_config, AppConfig};
pub use core::{
    CapitalMetrics, ClosedTrade, DataSource, ExecutionStats, OrderBook, PnLResult, Result, Trade,
    TradeError, TradeExecutor, TradeState,
};
pub use domain::{MomentumDetector, OrderBookImbalance, Position, Side, VolatilityDetector, Vwap};
pub use exchange::{ExchangeConfig, ExchangeConnector, ExchangeError, ExchangeType};
pub use strategy::{GptMarketMaker, GptMarketMakerConfig, Strategy};
pub use utils::{FileDataSource, OrderBookMessage, ParquetDataSource};
