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

/// WebSocket message structures for Bybit
#[derive(Debug, Serialize, Deserialize)]
pub struct WsRequest {
    pub op: String,
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct WsResponse {
    pub topic: Option<String>,
    pub success: Option<bool>,
    pub ret_msg: Option<String>,
    pub conn_id: Option<String>,
    pub op: Option<String>,
    #[serde(rename = "type")]
    pub msg_type: Option<String>,  // "snapshot" or "delta" for orderbook updates
    pub ts: Option<i64>,
    pub data: Option<WsOrderbookData>,
    pub cts: Option<i64>,  // client timestamp
}

#[derive(Debug, Deserialize)]
pub struct WsOrderbookData {
    pub s: String,           // symbol
    pub b: Vec<[String; 2]>, // bids
    pub a: Vec<[String; 2]>, // asks
    pub u: i64,              // update_id
    pub seq: Option<i64>,    // sequence number
}

impl WsRequest {
    pub fn subscribe(symbols: Vec<String>, depth: u32) -> Self {
        let args: Vec<String> = symbols
            .into_iter()
            .map(|symbol| format!("orderbook.{}.{}", depth, symbol))
            .collect();
        
        Self {
            op: "subscribe".to_string(),
            args,
        }
    }
    
    pub fn ping() -> Self {
        Self {
            op: "ping".to_string(),
            args: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_orderbook_delta_message() {
        let json = r#"{
            "topic":"orderbook.50.ETHUSDT",
            "type":"delta",
            "ts":1756134462072,
            "data":{
                "s":"ETHUSDT",
                "b":[["4646.26","6.46"],["4646.01","0.47"]],
                "a":[["4646.96","34.95"],["4647.07","2.20"],["4647.81","0"]],
                "u":48114057,
                "seq":302697339869
            },
            "cts":1756134462070
        }"#;

        let response: WsResponse = serde_json::from_str(json).unwrap();
        
        assert_eq!(response.topic, Some("orderbook.50.ETHUSDT".to_string()));
        assert_eq!(response.msg_type, Some("delta".to_string()));
        assert_eq!(response.ts, Some(1756134462072));
        assert_eq!(response.cts, Some(1756134462070));
        
        let data = response.data.unwrap();
        assert_eq!(data.s, "ETHUSDT");
        assert_eq!(data.b.len(), 2);
        assert_eq!(data.a.len(), 3);
        assert_eq!(data.u, 48114057);
        assert_eq!(data.seq, Some(302697339869));
    }

    #[test]
    fn test_parse_orderbook_snapshot_message() {
        let json = r#"{
            "topic":"orderbook.50.BTCUSDT",
            "type":"snapshot",
            "ts":1756134462000,
            "data":{
                "s":"BTCUSDT",
                "b":[["68000.00","1.5"]],
                "a":[["68001.00","2.0"]],
                "u":12345678,
                "seq":null
            }
        }"#;

        let response: WsResponse = serde_json::from_str(json).unwrap();
        
        assert_eq!(response.topic, Some("orderbook.50.BTCUSDT".to_string()));
        assert_eq!(response.msg_type, Some("snapshot".to_string()));
        assert_eq!(response.ts, Some(1756134462000));
        
        let data = response.data.unwrap();
        assert_eq!(data.s, "BTCUSDT");
        assert_eq!(data.b.len(), 1);
        assert_eq!(data.a.len(), 1);
        assert_eq!(data.u, 12345678);
        assert_eq!(data.seq, None);
    }

    #[test]
    fn test_parse_subscribe_response() {
        let json = r#"{
            "success":true,
            "ret_msg":"",
            "conn_id":"abcd1234",
            "op":"subscribe",
            "topic":null,
            "type":null,
            "ts":null,
            "data":null
        }"#;

        let response: WsResponse = serde_json::from_str(json).unwrap();
        
        assert_eq!(response.success, Some(true));
        assert_eq!(response.ret_msg, Some("".to_string()));
        assert_eq!(response.conn_id, Some("abcd1234".to_string()));
        assert_eq!(response.op, Some("subscribe".to_string()));
        assert!(response.topic.is_none());
        assert!(response.data.is_none());
    }

    #[test]
    fn test_parse_pong_response() {
        let json = r#"{
            "success":true,
            "ret_msg":"pong",
            "conn_id":"abcd1234",
            "op":"pong",
            "topic":null,
            "type":null,
            "ts":null,
            "data":null
        }"#;

        let response: WsResponse = serde_json::from_str(json).unwrap();
        
        assert_eq!(response.op, Some("pong".to_string()));
        assert_eq!(response.ret_msg, Some("pong".to_string()));
        assert!(response.data.is_none());
    }

    #[test]
    fn test_create_subscribe_request() {
        let req = WsRequest::subscribe(vec!["ETHUSDT".to_string()], 50);
        
        assert_eq!(req.op, "subscribe");
        assert_eq!(req.args, vec!["orderbook.50.ETHUSDT"]);
        
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"op\":\"subscribe\""));
        assert!(json.contains("orderbook.50.ETHUSDT"));
    }

    #[test]
    fn test_create_ping_request() {
        let req = WsRequest::ping();
        
        assert_eq!(req.op, "ping");
        assert!(req.args.is_empty());
        
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"op\":\"ping\""));
    }
}