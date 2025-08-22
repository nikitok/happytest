use crate::core::{Trade, PnLResult};
use crate::pnl::{
    models::Method,
    fifo::FifoProcessor,
    position::PositionProcessor,
};
use std::collections::HashMap;
use comfy_table::Table;

/// Trait for calculation
pub trait Processor {
    /// Process trades and calculate P&L
    fn process(&self, trades: &[Trade], method: Method) -> PnLResult;
}

/// Main PnL report generator that delegates to specific implementations
pub struct PnlReport {
    fifo_processor: FifoProcessor,
    position_processor: PositionProcessor,
}

impl PnlReport {
    pub fn new() -> Self {
        Self {
            fifo_processor: FifoProcessor::new(),
            position_processor: PositionProcessor::new(),
        }
    }
    
    /// Calculate P&L metrics from trading logs using specified method
    ///
    /// # Arguments
    /// * `trades` - List of Trade objects
    /// * `method` - Calculation method (Fifo or Position)
    ///
    /// # Returns
    /// * `PnLResult` - Complete P&L result including realized and unrealized P&L
    pub fn calculate(&self, trades: &[Trade], method: Method) -> PnLResult {
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
            Method::Fifo => self.fifo_processor.process_realized(&filled_trades),
            Method::Position => self.position_processor.process_position(&filled_trades),
        };
        
        // For now, we'll skip unrealized PnL calculation as it requires open trades tracking
        // This can be enhanced later to maintain state of open positions
        
        result
    }
    
    /// Generate a tabular report of P&L by symbol
    pub fn report(&self, trades: &[Trade], method: Method) -> String {
        // Group trades by symbol
        let mut trades_by_symbol: HashMap<String, Vec<Trade>> = HashMap::new();
        
        for trade in trades {
            trades_by_symbol
                .entry(trade.symbol.clone())
                .or_default()
                .push(trade.clone());
        }
        
        // Create table
        let mut table = Table::new();
        table.set_header(vec![
            "Symbol",
            "Trades",
            "Last Price",
            "Realized P&L",
            "Unrealized P&L",
            "Remaining Shares",
            "Total P&L"
        ]);
        
        // Sort symbols for consistent output
        let mut symbols: Vec<String> = trades_by_symbol.keys().cloned().collect();
        symbols.sort();
        
        let mut total_trades = 0;
        let mut total_realized = 0.0;
        let mut total_unrealized = 0.0;
        let mut total_remaining = 0.0;
        
        // Process each symbol
        for symbol in symbols {
            if let Some(symbol_trades) = trades_by_symbol.get(&symbol) {
                let result = self.calculate(symbol_trades, method);
                let last_price = symbol_trades.last().map(|t| t.price).unwrap_or(0.0);
                let total_pnl = result.total_pnl + result.unrealized_pnl;
                
                table.add_row(vec![
                    symbol.clone(),
                    symbol_trades.len().to_string(),
                    format!("${:.2}", last_price),
                    format!("${:.2}", result.total_pnl),
                    format!("${:.2}", result.unrealized_pnl),
                    format!("{:.0}", result.remaining_shares),
                    format!("${:.2}", total_pnl),
                ]);
                
                total_trades += symbol_trades.len();
                total_realized += result.total_pnl;
                total_unrealized += result.unrealized_pnl;
                total_remaining += result.remaining_shares;
            }
        }
        
        let grand_total = total_realized + total_unrealized;
        
        // Add separator
        table.add_row(vec![
            "─────────".to_string(),
            "─────────".to_string(),
            "─────────────".to_string(),
            "─────────────".to_string(),
            "─────────────".to_string(),
            "─────────────".to_string(),
            "─────────────".to_string(),
        ]);
        
        // Add totals row
        table.add_row(vec![
            "TOTAL".to_string(),
            total_trades.to_string(),
            "-".to_string(),
            format!("${:.2}", total_realized),
            format!("${:.2}", total_unrealized),
            format!("{:.0}", total_remaining),
            format!("${:.2}", grand_total),
        ]);
        
        format!("\n=== P&L Summary by Symbol ===\n{}", table)
    }
}

impl Default for PnlReport {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for PnlReport {
    fn process(&self, trades: &[Trade], method: Method) -> PnLResult {
        self.calculate(trades, method)
    }
}