use crate::core::{Trade, PnLResult};
use crate::pnl::{
    models::PnlMethod,
    fifo::FifoPnlProcessor,
    position::PositionPnlProcessor,
};

/// Trait for PnL calculation
pub trait PnlProcessor {
    /// Process trades and calculate P&L
    fn process(&self, trades: &[Trade], method: PnlMethod) -> PnLResult;
}

/// Main PnL report generator that delegates to specific implementations
pub struct PnlReport {
    fifo_processor: FifoPnlProcessor,
    position_processor: PositionPnlProcessor,
}

impl PnlReport {
    pub fn new() -> Self {
        Self {
            fifo_processor: FifoPnlProcessor::new(),
            position_processor: PositionPnlProcessor::new(),
        }
    }
    
    /// Calculate P&L metrics from trading logs using specified method
    ///
    /// # Arguments
    /// * `trades` - List of Trade objects
    /// * `method` - PnL calculation method (Fifo or Position)
    ///
    /// # Returns
    /// * `PnLResult` - Complete PnL result including realized and unrealized P&L
    pub fn calculate(&self, trades: &[Trade], method: PnlMethod) -> PnLResult {
        // Filter only filled orders (actual trades)
        let filled_orders: Vec<&Trade> = trades.iter()
            .filter(|t| t.status.to_lowercase() == "filled")
            .collect();
        
        if filled_orders.is_empty() {
            return PnLResult {
                total_pnl: 0.0,
                unrealized_pnl: 0.0,
                closed_trades: Vec::new(),
                total_fees: 0.0,
                remaining_shares: 0.0,
            };
        }
        
        // Convert to owned trades for processing
        let filled_trades: Vec<Trade> = filled_orders.into_iter().cloned().collect();
        
        // Process trades based on selected method
        let result = match method {
            PnlMethod::Fifo => self.fifo_processor.process_realized(&filled_trades),
            PnlMethod::Position => self.position_processor.process_position(&filled_trades),
        };
        
        // For now, we'll skip unrealized PnL calculation as it requires open trades tracking
        // This can be enhanced later to maintain state of open positions
        
        result
    }
    
}

impl Default for PnlReport {
    fn default() -> Self {
        Self::new()
    }
}

impl PnlProcessor for PnlReport {
    fn process(&self, trades: &[Trade], method: PnlMethod) -> PnLResult {
        self.calculate(trades, method)
    }
}