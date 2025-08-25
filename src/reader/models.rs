use serde::{Deserialize, Serialize};

/// Bybit orderbook response structure
#[derive(Debug, Deserialize)]
pub struct BybitResponse {
    #[serde(rename = "retCode")]
    pub ret_code: i32,
    #[serde(rename = "retMsg")]
    pub ret_msg: String,
    pub result: OrderbookResult,
    #[allow(dead_code)]
    pub time: i64,
}

#[derive(Debug, Deserialize)]
pub struct OrderbookResult {
    pub s: String,           // symbol
    pub b: Vec<[String; 2]>, // bids [price, size]
    pub a: Vec<[String; 2]>, // asks [price, size]
    pub ts: i64,             // timestamp
    pub u: i64,              // update id
}

/// Orderbook data structure to save
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderbookData {
    pub symbol: String,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
    pub timestamp: i64,
    pub update_id: i64,
    pub fetch_time: i64,
}