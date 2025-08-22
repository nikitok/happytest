use std::collections::HashMap;
use crate::core::{Trade, ClosedTrade, PnLResult};
use crate::pnl::models::{PnlRecord, PositionInfo};

/// Position-based PnL processor
pub struct PositionPnlProcessor;

impl PositionPnlProcessor {
    pub fn new() -> Self {
        Self
    }
    
    /// Process trades and calculate realized P&L using Position-based model
    /// 
    /// In the position-based model, we track the net position for each asset
    /// and calculate P&L based on average cost basis.
    /// 
    /// # Arguments
    /// * `trades` - List of Trade objects
    /// 
    /// # Returns
    /// * `PnLResult` - Object containing open_trades, closed_trades, and pnl_records
    pub fn process_position(&self, trades: &[Trade]) -> PnLResult {
        // Dictionary to store positions by asset
        let mut positions: HashMap<String, PositionInfo> = HashMap::new();
        
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
            
            // Initialize position for the asset if it doesn't exist
            let pos = positions.entry(symbol.clone()).or_insert_with(|| PositionInfo {
                quantity: 0.0,
                avg_price: 0.0,
                total_cost: 0.0,
                side: None,
                trades: Vec::new(),
            });
            
            // Determine if this is increasing or reducing position
            if pos.quantity == 0.0 {
                // New position
                pos.quantity = if side.to_lowercase() == "buy" { quantity } else { -quantity };
                pos.avg_price = price;
                pos.total_cost = price * quantity;
                pos.side = Some(side.clone());
                pos.trades.push(order.clone());
                
            } else if (pos.quantity > 0.0 && side.to_lowercase() == "buy") || 
                      (pos.quantity < 0.0 && side.to_lowercase() == "sell") {
                // Increasing position (same direction)
                if side.to_lowercase() == "buy" {
                    let new_quantity = pos.quantity + quantity;
                    pos.total_cost += price * quantity;
                    pos.avg_price = pos.total_cost / new_quantity;
                    pos.quantity = new_quantity;
                } else {  // sell (short)
                    let new_quantity = pos.quantity - quantity;
                    pos.total_cost += price * quantity;
                    pos.avg_price = pos.total_cost / new_quantity.abs();
                    pos.quantity = new_quantity;
                }
                pos.trades.push(order.clone());
                
            } else {
                // Reducing or closing position (opposite direction)
                let mut remaining_quantity = quantity;
                
                // Calculate P&L
                if pos.quantity > 0.0 {  // Long position being reduced
                    let matched_quantity = remaining_quantity.min(pos.quantity);
                    let pnl = (price - pos.avg_price) * matched_quantity;
                    pos.quantity -= matched_quantity;
                    remaining_quantity -= matched_quantity;
                    
                    // Create closed trade record
                    let closed_trade = ClosedTrade {
                        quantity: matched_quantity,
                        pnl,
                        open_side: "Buy".to_string(),
                        close_side: "Sell".to_string(),
                        open_price: pos.avg_price,
                        close_price: price,
                    };
                    closed_trades.push(closed_trade);
                    
                    // Add to PnL records
                    pnl_records.push(PnlRecord {
                        timestamp: time,
                        symbol: symbol.clone(),
                        profit: pnl,
                    });
                    
                } else {  // Short position being reduced
                    let matched_quantity = remaining_quantity.min(pos.quantity.abs());
                    let pnl = (pos.avg_price - price) * matched_quantity;
                    pos.quantity += matched_quantity;
                    remaining_quantity -= matched_quantity;
                    
                    // Create closed trade record
                    let closed_trade = ClosedTrade {
                        quantity: matched_quantity,
                        pnl,
                        open_side: "Sell".to_string(),
                        close_side: "Buy".to_string(),
                        open_price: pos.avg_price,
                        close_price: price,
                    };
                    closed_trades.push(closed_trade);
                    
                    // Add to PnL records
                    pnl_records.push(PnlRecord {
                        timestamp: time,
                        symbol: symbol.clone(),
                        profit: pnl,
                    });
                }
                
                // Update position cost basis
                if pos.quantity != 0.0 {
                    pos.total_cost = pos.avg_price * pos.quantity.abs();
                } else {
                    pos.total_cost = 0.0;
                    pos.avg_price = 0.0;
                    pos.trades.clear();
                }
                
                // If position reversed (went from long to short or vice versa)
                if remaining_quantity > 0.0 {
                    pos.quantity = if side.to_lowercase() == "buy" { remaining_quantity } else { -remaining_quantity };
                    pos.avg_price = price;
                    pos.total_cost = price * remaining_quantity;
                    pos.side = Some(side.clone());
                    pos.trades = vec![Trade {
                        id: order.id.clone(),
                        time,
                        symbol: symbol.clone(),
                        side: side.clone(),
                        price,
                        quantity: remaining_quantity,
                        status: order.status.clone(),
                    }];
                }
            }
        }
        
        // Calculate total realized PnL
        let total_pnl = closed_trades.iter().map(|t| t.pnl).sum();
        
        // Create and return PnLResult object
        PnLResult {
            total_pnl,
            unrealized_pnl: 0.0,
            closed_trades,
            total_fees: 0.0,
        }
    }
    
    /// Get the current positions
    pub fn get_positions(&self) -> &HashMap<String, PositionInfo> {
        // This would be used if we stored positions as state
        // For now, we return positions from process_position
        unimplemented!("Use process_position to get positions")
    }
}

impl Default for PositionPnlProcessor {
    fn default() -> Self {
        Self::new()
    }
}