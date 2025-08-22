use crate::core::{OrderBook, Trade};
use crate::core::errors::Result;

/// Trait for data sources that provide order book updates
pub trait DataSource: Send {
    /// Get the next order book update
    fn next_orderbook(&mut self) -> Result<Option<OrderBook>>;
    
    /// Reset the data source to the beginning
    fn reset(&mut self) -> Result<()>;
    
    /// Get total number of order books available
    fn total_count(&self) -> Option<usize>;
}

/// Trait for trade execution
pub trait TradeExecutor: Send {
    /// Execute a trade and return the result
    fn execute_trade(&mut self, trade: Trade) -> Result<Trade>;
    
    /// Get execution statistics
    fn get_stats(&self) -> ExecutionStats;
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    pub total_trades: usize,
    pub filled_trades: usize,
    pub rejected_trades: usize,
    pub partial_fills: usize,
    pub total_slippage: f64,
}