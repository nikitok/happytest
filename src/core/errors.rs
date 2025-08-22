use thiserror::Error;

#[derive(Error, Debug)]
pub enum TradeError {
    #[error("Trade not found: {0}")]
    TradeNotFound(String),
    
    #[error("Invalid order book: {0}")]
    InvalidOrderBook(String),
    
    #[error("Position limit exceeded for symbol {symbol}: current {current}, limit {limit}")]
    PositionLimitExceeded {
        symbol: String,
        current: f64,
        limit: f64,
    },
    
    #[error("Insufficient margin: required {required}, available {available}")]
    InsufficientMargin {
        required: f64,
        available: f64,
    },
    
    #[error("Invalid trade parameters: {0}")]
    InvalidTradeParameters(String),
    
    #[error("Order execution failed: {0}")]
    OrderExecutionFailed(String),
    
    #[error("Strategy error: {0}")]
    StrategyError(String),
    
    #[error("Data loading error: {0}")]
    DataLoadingError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TradeError>;