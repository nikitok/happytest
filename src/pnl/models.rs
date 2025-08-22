use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PnlMethod {
    Fifo,
    Position,
}

impl Default for PnlMethod {
    fn default() -> Self {
        PnlMethod::Fifo
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnlRecord {
    pub timestamp: i64,
    pub symbol: String,
    pub profit: f64,
}

#[derive(Debug, Clone)]
pub struct PositionInfo {
    pub quantity: f64,
    pub avg_price: f64,
    pub total_cost: f64,
    pub side: Option<String>,
    pub trades: Vec<crate::core::Trade>,
}