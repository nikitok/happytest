use crate::core::{OrderBook, Trade};

/// Base trait for all trading strategies
pub trait Strategy: Send + Sync {
    /// Propose a trade based on the current order book
    fn propose_trade(&mut self, order_book: &OrderBook) -> Option<Trade>;
    
    /// Update internal position tracking after trade execution
    fn update_position(&mut self, trade: &Trade, filled: bool);
    
    /// Get the strategy name for identification
    fn name(&self) -> &str;
    
    /// Get current net position for a symbol
    fn get_position(&self, symbol: &str) -> f64;
    
    /// Reset strategy state
    fn reset(&mut self);
}