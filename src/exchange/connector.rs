//! Exchange connector trait for multi-exchange support.
//!
//! This trait abstracts the exchange connectivity, allowing easy extension
//! to new exchanges without modifying core logic.

use crate::reader::models::OrderbookData;
use async_trait::async_trait;

/// Result type for exchange operations.
pub type ExchangeResult<T> = Result<T, ExchangeError>;

/// Errors that can occur during exchange operations.
#[derive(Debug, thiserror::Error)]
pub enum ExchangeError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Disconnected")]
    Disconnected,

    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for ExchangeError {
    fn from(err: anyhow::Error) -> Self {
        ExchangeError::Other(err.to_string())
    }
}

/// Configuration for exchange connector.
#[derive(Debug, Clone)]
pub struct ExchangeConfig {
    /// Trading symbol (e.g., "BTCUSDT")
    pub symbol: String,
    /// Use testnet/sandbox mode
    pub testnet: bool,
    /// Order book depth
    pub depth: u32,
}

impl Default for ExchangeConfig {
    fn default() -> Self {
        Self {
            symbol: "BTCUSDT".to_string(),
            testnet: false,
            depth: 50,
        }
    }
}

/// Trait for exchange connectivity.
///
/// Implement this trait to add support for a new exchange.
/// The connector handles WebSocket connection, subscription, and data streaming.
#[async_trait]
pub trait ExchangeConnector: Send + Sync {
    /// Get the exchange name.
    fn name(&self) -> &'static str;

    /// Connect to the exchange WebSocket.
    async fn connect(&mut self) -> ExchangeResult<()>;

    /// Subscribe to orderbook updates for a symbol.
    async fn subscribe(&mut self, symbol: &str) -> ExchangeResult<()>;

    /// Get the next orderbook update.
    /// Returns None if the connection is closed.
    async fn next_orderbook(&mut self) -> ExchangeResult<Option<OrderbookData>>;

    /// Check if connected.
    fn is_connected(&self) -> bool;

    /// Disconnect from the exchange.
    async fn disconnect(&mut self) -> ExchangeResult<()>;
}

/// Exchange type enum for factory pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeType {
    Bybit,
    Binance,
    // Future exchanges:
    // OKX,
    // Deribit,
}

impl ExchangeType {
    /// Get the display name of the exchange.
    pub fn name(&self) -> &'static str {
        match self {
            ExchangeType::Bybit => "Bybit",
            ExchangeType::Binance => "Binance",
        }
    }
}

impl std::fmt::Display for ExchangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_config_default() {
        let config = ExchangeConfig::default();
        assert_eq!(config.symbol, "BTCUSDT");
        assert!(!config.testnet);
        assert_eq!(config.depth, 50);
    }

    #[test]
    fn test_exchange_type_name() {
        assert_eq!(ExchangeType::Bybit.name(), "Bybit");
        assert_eq!(ExchangeType::Bybit.to_string(), "Bybit");
    }

    #[test]
    fn test_exchange_error_display() {
        let err = ExchangeError::ConnectionFailed("test".to_string());
        assert!(err.to_string().contains("Connection failed"));

        let err = ExchangeError::Timeout;
        assert_eq!(err.to_string(), "Timeout");
    }
}
