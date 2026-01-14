//! Shared data models for orderbook data.

use serde::{Deserialize, Serialize};

/// Orderbook data structure for storage and processing.
///
/// This is the canonical representation of orderbook data used across
/// the system for storage, backtest, and analytics.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderbookData {
    pub symbol: String,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
    pub timestamp: i64,
    pub update_id: i64,
    pub fetch_time: i64,
}
