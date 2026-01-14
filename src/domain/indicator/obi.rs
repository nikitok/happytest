//! Order Book Imbalance (OBI) indicator.
//!
//! Measures the imbalance between bid and ask volumes in the order book.

use crate::core::OrderBook;

/// Order Book Imbalance calculator.
///
/// OBI = (bid_volume - ask_volume) / (bid_volume + ask_volume)
///
/// Range: -1.0 to 1.0
/// - Positive: More buying pressure
/// - Negative: More selling pressure
#[derive(Debug, Clone, Copy)]
pub struct OrderBookImbalance {
    depth: usize,
}

impl OrderBookImbalance {
    /// Create a new OBI calculator.
    ///
    /// # Arguments
    /// * `depth` - Number of price levels to consider (default: 5)
    pub fn new(depth: usize) -> Self {
        Self { depth }
    }

    /// Calculate OBI from an order book.
    pub fn calculate(&self, order_book: &OrderBook) -> f64 {
        if order_book.bids.is_empty() || order_book.asks.is_empty() {
            return 0.0;
        }

        let bid_vol: f64 = order_book
            .bids
            .iter()
            .take(self.depth)
            .map(|(_, v)| v)
            .sum();

        let ask_vol: f64 = order_book
            .asks
            .iter()
            .take(self.depth)
            .map(|(_, v)| v)
            .sum();

        let total = bid_vol + ask_vol;
        if total == 0.0 {
            return 0.0;
        }

        (bid_vol - ask_vol) / total
    }

    /// Check if OBI indicates buying pressure above threshold.
    pub fn is_bullish(&self, order_book: &OrderBook, threshold: f64) -> bool {
        self.calculate(order_book) > threshold
    }

    /// Check if OBI indicates selling pressure above threshold.
    pub fn is_bearish(&self, order_book: &OrderBook, threshold: f64) -> bool {
        self.calculate(order_book) < -threshold
    }
}

impl Default for OrderBookImbalance {
    fn default() -> Self {
        Self::new(5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_order_book(bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) -> OrderBook {
        OrderBook::new(bids, asks, 0)
    }

    #[test]
    fn test_obi_balanced() {
        let obi = OrderBookImbalance::new(5);
        let book = create_order_book(
            vec![(100.0, 10.0), (99.0, 10.0)],
            vec![(101.0, 10.0), (102.0, 10.0)],
        );

        assert_eq!(obi.calculate(&book), 0.0);
    }

    #[test]
    fn test_obi_bullish() {
        let obi = OrderBookImbalance::new(5);
        let book = create_order_book(
            vec![(100.0, 30.0), (99.0, 20.0)],  // 50 total
            vec![(101.0, 10.0), (102.0, 10.0)], // 20 total
        );

        // OBI = (50 - 20) / (50 + 20) = 30/70 = 0.4286
        let obi_val = obi.calculate(&book);
        assert!((obi_val - 0.4286).abs() < 0.001);
        assert!(obi.is_bullish(&book, 0.1));
        assert!(!obi.is_bearish(&book, 0.1));
    }

    #[test]
    fn test_obi_bearish() {
        let obi = OrderBookImbalance::new(5);
        let book = create_order_book(
            vec![(100.0, 10.0), (99.0, 10.0)],  // 20 total
            vec![(101.0, 30.0), (102.0, 20.0)], // 50 total
        );

        // OBI = (20 - 50) / (20 + 50) = -30/70 = -0.4286
        let obi_val = obi.calculate(&book);
        assert!((obi_val - (-0.4286)).abs() < 0.001);
        assert!(!obi.is_bullish(&book, 0.1));
        assert!(obi.is_bearish(&book, 0.1));
    }

    #[test]
    fn test_obi_empty_book() {
        let obi = OrderBookImbalance::new(5);

        let book_no_bids = create_order_book(vec![], vec![(101.0, 10.0)]);
        assert_eq!(obi.calculate(&book_no_bids), 0.0);

        let book_no_asks = create_order_book(vec![(100.0, 10.0)], vec![]);
        assert_eq!(obi.calculate(&book_no_asks), 0.0);
    }

    #[test]
    fn test_obi_depth_limit() {
        let obi = OrderBookImbalance::new(2); // Only top 2 levels
        let book = create_order_book(
            vec![(100.0, 10.0), (99.0, 10.0), (98.0, 100.0)], // Top 2: 20
            vec![(101.0, 10.0), (102.0, 10.0), (103.0, 100.0)], // Top 2: 20
        );

        // Should ignore the 100-volume levels
        assert_eq!(obi.calculate(&book), 0.0);
    }

    #[test]
    fn test_obi_default() {
        let obi = OrderBookImbalance::default();
        assert_eq!(obi.depth, 5);
    }
}
