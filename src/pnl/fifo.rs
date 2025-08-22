use std::collections::HashMap;
use crate::core::{Trade, ClosedTrade, PnLResult};
use crate::pnl::models::PnlRecord;

/// FIFO (First-In-First-Out) PnL processor
pub struct FifoPnlProcessor;

impl FifoPnlProcessor {
    pub fn new() -> Self {
        Self
    }
    
    /// Process trades and calculate realized P&L using FIFO method
    ///
    /// # Arguments
    /// * `trades` - List of Trade objects containing trading logs
    ///
    /// # Returns
    /// * `PnLResult` - Object containing open_trades, closed_trades, and pnl_records
    pub fn process_realized(&self, trades: &[Trade]) -> PnLResult {
        // Dictionary to store open trades by asset
        let mut open_trades: HashMap<String, Vec<Trade>> = HashMap::new();
        
        // Lists to store closed trades and PnL records
        let mut closed_trades: Vec<ClosedTrade> = Vec::new();
        let mut pnl_records = Vec::new();
        
        // Process each filled trade chronologically
        for order in trades {
            let time = order.time;
            let symbol = order.symbol.clone();
            let side = order.side.clone();
            let price = order.price;
            let quantity = order.quantity;
            
            // Create a new trade
            let trade = Trade {
                id: order.id.clone(),
                time,
                symbol: symbol.clone(),
                side: side.clone(),
                price,
                quantity,
                status: order.status.clone(),
            };
            
            // Initialize the asset's open trades list if it doesn't exist
            let asset_trades = open_trades.entry(symbol.clone()).or_insert_with(Vec::new);
            
            // If there are no open trades for this asset or the side is the same as the first open trade,
            // add this trade to the open trades list
            if asset_trades.is_empty() || asset_trades[0].side == side {
                asset_trades.push(trade);
                continue;
            }
            
            // Process matching trades (opposite sides)
            let mut remaining_quantity = quantity;
            
            // Match with existing open trades using FIFO
            while remaining_quantity > 0.0 && !asset_trades.is_empty() {
                let open_trade = &mut asset_trades[0];
                
                // Calculate the matched quantity
                let matched_quantity = remaining_quantity.min(open_trade.quantity);
                
                // Calculate PnL for this match
                let pnl = if side.to_lowercase() == "buy" {
                    // Current trade is buy, open trade is sell
                    (open_trade.price - price) * matched_quantity
                } else {
                    // Current trade is sell, open trade is buy
                    (price - open_trade.price) * matched_quantity
                };
                
                // Create a closed trade record
                let closed_trade = ClosedTrade {
                    quantity: matched_quantity,
                    pnl,
                    open_side: open_trade.side.clone(),
                    close_side: side.clone(),
                    open_price: open_trade.price,
                    close_price: price,
                };
                closed_trades.push(closed_trade);
                
                // Add to PnL records for visualization
                pnl_records.push(PnlRecord {
                    timestamp: time,
                    symbol: symbol.clone(),
                    profit: pnl,
                });
                
                // Update remaining quantities
                remaining_quantity -= matched_quantity;
                open_trade.quantity -= matched_quantity;
                
                // Remove the open trade if it's fully matched
                if open_trade.quantity == 0.0 {
                    asset_trades.remove(0);
                }
            }
            
            // If there's still remaining quantity, add it as a new open trade
            if remaining_quantity > 0.0 {
                let new_trade = Trade {
                    id: order.id.clone(),
                    time,
                    symbol: symbol.clone(),
                    side: side.clone(),
                    price,
                    quantity: remaining_quantity,
                    status: order.status.clone(),
                };
                asset_trades.push(new_trade);
            }
        }
        
        // Calculate total realized PnL
        let total_pnl = closed_trades.iter().map(|t| t.pnl).sum();
        
        // Remove empty entries from open_trades
        open_trades.retain(|_, trades| !trades.is_empty());
        
        // Create and return PnLResult object
        PnLResult {
            total_pnl,
            unrealized_pnl: 0.0,
            closed_trades,
            total_fees: 0.0,
        }
    }
}

impl Default for FifoPnlProcessor {
    fn default() -> Self {
        Self::new()
    }
}