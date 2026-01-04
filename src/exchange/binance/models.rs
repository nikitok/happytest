//! Binance Futures USDT-M WebSocket protocol models.

use serde::{Deserialize, Serialize};

/// WebSocket request message for Binance
#[derive(Debug, Serialize)]
pub struct WsRequest {
    pub method: String,
    pub params: Vec<String>,
    pub id: u64,
}

impl WsRequest {
    /// Create a subscription request for orderbook depth
    /// Symbol must be lowercase (e.g., "btcusdt")
    pub fn subscribe(symbol: &str, depth: u32) -> Self {
        // Binance uses lowercase symbols and specific depth levels
        // Available depths: 5, 10, 20
        let depth = match depth {
            d if d <= 5 => 5,
            d if d <= 10 => 10,
            _ => 20,
        };

        Self {
            method: "SUBSCRIBE".to_string(),
            params: vec![format!("{}@depth{}@100ms", symbol.to_lowercase(), depth)],
            id: 1,
        }
    }
}

/// WebSocket stream response wrapper from Binance
#[derive(Debug, Deserialize)]
pub struct WsStreamResponse {
    pub stream: Option<String>,
    pub data: Option<DepthData>,
    // For subscription responses
    pub result: Option<serde_json::Value>,
    pub id: Option<u64>,
}

/// Depth update event from Binance Futures
#[derive(Debug, Deserialize)]
pub struct DepthData {
    /// Event type: "depthUpdate"
    pub e: String,
    /// Event time
    #[serde(rename = "E")]
    pub event_time: i64,
    /// Transaction time
    #[serde(rename = "T")]
    pub transaction_time: i64,
    /// Symbol
    pub s: String,
    /// First update ID in event
    #[serde(rename = "U")]
    pub first_update_id: i64,
    /// Final update ID in event
    pub u: i64,
    /// Previous final update ID
    pub pu: i64,
    /// Bids [price, quantity]
    pub b: Vec<[String; 2]>,
    /// Asks [price, quantity]
    pub a: Vec<[String; 2]>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_subscribe_request() {
        let req = WsRequest::subscribe("BTCUSDT", 50);

        assert_eq!(req.method, "SUBSCRIBE");
        assert_eq!(req.params, vec!["btcusdt@depth20@100ms"]);
        assert_eq!(req.id, 1);

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"method\":\"SUBSCRIBE\""));
        assert!(json.contains("btcusdt@depth20@100ms"));
    }

    #[test]
    fn test_depth_levels() {
        // depth <= 5 -> 5
        let req = WsRequest::subscribe("btcusdt", 3);
        assert_eq!(req.params[0], "btcusdt@depth5@100ms");

        // depth <= 10 -> 10
        let req = WsRequest::subscribe("btcusdt", 10);
        assert_eq!(req.params[0], "btcusdt@depth10@100ms");

        // depth > 10 -> 20
        let req = WsRequest::subscribe("btcusdt", 50);
        assert_eq!(req.params[0], "btcusdt@depth20@100ms");
    }

    #[test]
    fn test_parse_depth_update() {
        let json = r#"{
            "stream": "btcusdt@depth20@100ms",
            "data": {
                "e": "depthUpdate",
                "E": 1683849600000,
                "T": 1683849600001,
                "s": "BTCUSDT",
                "U": 123456789,
                "u": 123456790,
                "pu": 123456788,
                "b": [["28000.00", "1.5"], ["27999.00", "2.0"]],
                "a": [["28001.00", "2.0"], ["28002.00", "3.0"]]
            }
        }"#;

        let response: WsStreamResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.stream, Some("btcusdt@depth20@100ms".to_string()));

        let data = response.data.unwrap();
        assert_eq!(data.e, "depthUpdate");
        assert_eq!(data.s, "BTCUSDT");
        assert_eq!(data.event_time, 1683849600000);
        assert_eq!(data.u, 123456790);
        assert_eq!(data.b.len(), 2);
        assert_eq!(data.a.len(), 2);
    }

    #[test]
    fn test_parse_subscribe_response() {
        // Binance returns {"result": null, "id": 1} for successful subscription
        let json = r#"{
            "result": null,
            "id": 1
        }"#;

        let response: WsStreamResponse = serde_json::from_str(json).unwrap();

        // result is null (parsed as Some(Value::Null))
        assert_eq!(response.id, Some(1));
        assert!(response.data.is_none());
        assert!(response.stream.is_none());
    }
}
