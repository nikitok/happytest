//! Position model representing an open trading position.
//!
//! This is the unified Position struct used across the codebase.

use serde::{Deserialize, Serialize};

/// Trading side (buy or sell).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    /// Get the opposite side.
    pub fn opposite(&self) -> Side {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }

    /// Check if this side is buying.
    pub fn is_buy(&self) -> bool {
        matches!(self, Side::Buy)
    }

    /// Check if this side is selling.
    pub fn is_sell(&self) -> bool {
        matches!(self, Side::Sell)
    }
}

impl From<&str> for Side {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            _ => panic!("Invalid side: {}", s),
        }
    }
}

impl From<String> for Side {
    fn from(s: String) -> Self {
        Side::from(s.as_str())
    }
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "Buy"),
            Side::Sell => write!(f, "Sell"),
        }
    }
}

/// A trading position with entry details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Trading symbol (e.g., "BTCUSDT")
    pub symbol: String,
    /// Position quantity
    pub quantity: f64,
    /// Entry price
    pub entry_price: f64,
    /// Position side
    pub side: Side,
    /// Entry timestamp in milliseconds
    pub entry_time: i64,
}

impl Position {
    /// Create a new position.
    pub fn new(
        symbol: impl Into<String>,
        quantity: f64,
        entry_price: f64,
        side: Side,
        entry_time: i64,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            quantity,
            entry_price,
            side,
            entry_time,
        }
    }

    /// Get the age of this position in milliseconds.
    pub fn age_ms(&self, current_time: i64) -> i64 {
        current_time - self.entry_time
    }

    /// Calculate unrealized P&L at current price.
    pub fn unrealized_pnl(&self, current_price: f64) -> f64 {
        match self.side {
            Side::Buy => (current_price - self.entry_price) * self.quantity,
            Side::Sell => (self.entry_price - current_price) * self.quantity,
        }
    }

    /// Calculate P&L in basis points at current price.
    pub fn pnl_bps(&self, current_price: f64) -> f64 {
        if self.entry_price == 0.0 {
            return 0.0;
        }

        let pnl_pct = match self.side {
            Side::Buy => (current_price - self.entry_price) / self.entry_price,
            Side::Sell => (self.entry_price - current_price) / self.entry_price,
        };

        pnl_pct * 10000.0
    }

    /// Get the notional value of this position.
    pub fn notional_value(&self) -> f64 {
        self.entry_price * self.quantity
    }

    /// Check if this position would be closed by a trade on the given side.
    pub fn is_closing_side(&self, trade_side: Side) -> bool {
        self.side != trade_side
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_side_opposite() {
        assert_eq!(Side::Buy.opposite(), Side::Sell);
        assert_eq!(Side::Sell.opposite(), Side::Buy);
    }

    #[test]
    fn test_side_from_str() {
        assert_eq!(Side::from("buy"), Side::Buy);
        assert_eq!(Side::from("Buy"), Side::Buy);
        assert_eq!(Side::from("BUY"), Side::Buy);
        assert_eq!(Side::from("sell"), Side::Sell);
        assert_eq!(Side::from("Sell"), Side::Sell);
    }

    #[test]
    fn test_side_display() {
        assert_eq!(Side::Buy.to_string(), "Buy");
        assert_eq!(Side::Sell.to_string(), "Sell");
    }

    #[test]
    fn test_position_age() {
        let pos = Position::new("BTCUSDT", 1.0, 50000.0, Side::Buy, 1000);
        assert_eq!(pos.age_ms(5000), 4000);
    }

    #[test]
    fn test_position_pnl_long() {
        let pos = Position::new("BTCUSDT", 1.0, 50000.0, Side::Buy, 0);

        // Price goes up 1%
        let pnl = pos.unrealized_pnl(50500.0);
        assert!((pnl - 500.0).abs() < 0.01);

        let pnl_bps = pos.pnl_bps(50500.0);
        assert!((pnl_bps - 100.0).abs() < 0.1); // 1% = 100 bps
    }

    #[test]
    fn test_position_pnl_short() {
        let pos = Position::new("BTCUSDT", 1.0, 50000.0, Side::Sell, 0);

        // Price goes down 1%
        let pnl = pos.unrealized_pnl(49500.0);
        assert!((pnl - 500.0).abs() < 0.01);

        let pnl_bps = pos.pnl_bps(49500.0);
        assert!((pnl_bps - 100.0).abs() < 0.1); // 1% = 100 bps
    }

    #[test]
    fn test_position_is_closing_side() {
        let long_pos = Position::new("BTCUSDT", 1.0, 50000.0, Side::Buy, 0);
        assert!(long_pos.is_closing_side(Side::Sell));
        assert!(!long_pos.is_closing_side(Side::Buy));

        let short_pos = Position::new("BTCUSDT", 1.0, 50000.0, Side::Sell, 0);
        assert!(short_pos.is_closing_side(Side::Buy));
        assert!(!short_pos.is_closing_side(Side::Sell));
    }

    #[test]
    fn test_position_notional_value() {
        let pos = Position::new("BTCUSDT", 0.5, 50000.0, Side::Buy, 0);
        assert!((pos.notional_value() - 25000.0).abs() < 0.01);
    }
}
