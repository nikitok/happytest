use super::models::{Trade, OrderBook};
use chrono::Utc;
use log::{debug, warn};

pub struct TradeState {
    all_trades: Vec<Trade>,
    orderbooks: Vec<OrderBook>
}

impl TradeState {
    pub fn new() -> Self {
        Self {
            all_trades: Vec::new(),
            orderbooks: Vec::new()
        }
    }

    pub fn add(&mut self, trade: Trade) {
        self.all_trades.push(trade);
    }

    pub fn get_trades_history(&self) -> Vec<&Trade> {
        self.all_trades
            .iter()
            .filter(|t| t.status == "filled")
            .collect()
    }

    pub fn get_all_trades(&self) -> &Vec<Trade> {
        &self.all_trades
    }

    pub fn change_status(&mut self, trade_id: &str, new_status: String) -> bool {
        for trade in &mut self.all_trades {
            if trade.id == trade_id {
                let old_status = trade.status.clone();
                trade.status = new_status.clone();
                debug!("Trade {} status changed from {} to {}", trade_id, old_status, new_status);
                return true;
            }
        }
        warn!("Trade with ID {} not found", trade_id);
        false
    }

    pub fn get_position(&self, symbol: &str) -> f64 {
        let mut position = 0.0;
        for trade in &self.all_trades {
            if trade.symbol != symbol || trade.status != "filled" {
                continue;
            }
            if trade.side == "Buy" {
                position += trade.quantity;
            } else if trade.side == "Sell" {
                position -= trade.quantity;
            }
        }
        position
    }

    pub fn get_position_age(&self, symbol: &str) -> i64 {
        let now = Utc::now().timestamp_millis();
        let mut last_time = 0;
        
        for trade in self.all_trades.iter().rev() {
            if trade.symbol == symbol && trade.status == "filled" {
                last_time = trade.time;
                break;
            }
        }
        
        if last_time > 0 {
            now - last_time
        } else {
            0
        }
    }

    pub fn get_recent_fills(&self, symbol: &str, window_ms: i64) -> Vec<String> {
        let now = Utc::now().timestamp_millis();
        let mut result = Vec::new();
        
        for trade in self.all_trades.iter().rev() {
            if trade.symbol == symbol && trade.status == "filled" && now - trade.time <= window_ms {
                result.push("filled".to_string());
            }
        }
        result
    }

    pub fn add_orderbook(&mut self, orderbook: OrderBook) {
        self.orderbooks.push(orderbook);
    }

    pub fn get_orderbooks(&self) -> &Vec<OrderBook> {
        &self.orderbooks
    }

    pub fn get_failed_trades(&self) -> Vec<&Trade> {
        self.all_trades
            .iter()
            .filter(|t| t.status != "filled")
            .collect()
    }
}