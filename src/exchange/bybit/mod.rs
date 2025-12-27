//! Bybit exchange integration.
//!
//! Provides WebSocket-based orderbook data streaming from Bybit exchange.
//!
//! ## Usage
//! ```rust,ignore
//! use happytest::exchange::bybit::{BybitReader, ReaderConfig};
//!
//! let config = ReaderConfig::default();
//! let reader = BybitReader::new(config)?;
//! reader.run().await?;
//! ```

pub mod models;
pub mod reader;

pub use models::{BybitResponse, OrderbookResult, WsRequest, WsResponse, WsOrderbookData};
pub use reader::{BybitReader, ReaderConfig};
