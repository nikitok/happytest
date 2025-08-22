use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub time: i64,
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub status: String,
    pub id: String,
}

impl Trade {
    pub fn new(
        time: i64,
        symbol: String,
        side: String,
        price: f64,
        quantity: f64,
    ) -> Self {
        Self {
            time,
            symbol,
            side,
            price,
            quantity,
            status: "pending".to_string(),
            id: Uuid::new_v4().to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    pub bids: Vec<(f64, f64)>, // (price, quantity)
    pub asks: Vec<(f64, f64)>, // (price, quantity)
    pub current_time: i64,
}

impl OrderBook {
    pub fn new(bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>, current_time: i64) -> Self {
        Self {
            bids,
            asks,
            current_time,
        }
    }

    pub fn mid_price(&self) -> f64 {
        if self.bids.is_empty() || self.asks.is_empty() {
            return 0.0;
        }
        (self.bids[0].0 + self.asks[0].0) / 2.0
    }

    pub fn spread_abs(&self) -> f64 {
        if self.bids.is_empty() || self.asks.is_empty() {
            return 0.0;
        }
        self.asks[0].0 - self.bids[0].0
    }

    pub fn spread_pct(&self) -> f64 {
        let mid = self.mid_price();
        if mid == 0.0 {
            return 0.0;
        }
        self.spread_abs() / mid
    }

    pub fn order_book_imbalance(&self) -> f64 {
        if self.bids.is_empty() || self.asks.is_empty() {
            return 0.0;
        }
        
        let bid_vol = self.bids.iter().take(5).map(|(_, v)| v).sum::<f64>();
        let ask_vol = self.asks.iter().take(5).map(|(_, v)| v).sum::<f64>();
        
        if bid_vol + ask_vol == 0.0 {
            return 0.0;
        }
        
        (bid_vol - ask_vol) / (bid_vol + ask_vol)
    }

    pub fn avg_top_bid_depth(&self) -> f64 {
        if self.bids.is_empty() {
            return 0.0;
        }
        self.bids.iter().take(5).map(|(_, v)| v).sum::<f64>() / self.bids.len().min(5) as f64
    }
}

#[derive(Debug, Clone)]
pub struct ClosedTrade {
    pub open_side: String,
    pub quantity: f64,
    pub open_price: f64,
    pub close_side: String,
    pub close_price: f64,
    pub pnl: f64,
}

#[derive(Debug)]
pub struct PnLResult {
    pub total_pnl: f64,
    pub unrealized_pnl: f64,
    pub closed_trades: Vec<ClosedTrade>,
    pub total_fees: f64,
}

#[derive(Debug, Clone)]
pub struct CapitalMetrics {
    pub max_required_capital: f64,
    pub max_drawdown: f64,
    pub max_open_positions_value: f64,
    pub average_capital_utilization: f64,
    pub peak_margin_requirement: f64,
    pub max_unrealized_loss: f64,
}