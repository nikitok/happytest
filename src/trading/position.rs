use std::collections::HashMap;
use crate::core::{Trade, TradeError, Result};

#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub side: String,
    pub timestamp: i64,
}

impl Position {
    pub fn new(symbol: String, quantity: f64, entry_price: f64, side: String, timestamp: i64) -> Self {
        Self {
            symbol,
            quantity,
            entry_price,
            side,
            timestamp,
        }
    }
    
    pub fn get_age_ms(&self, current_time: i64) -> i64 {
        current_time - self.timestamp
    }
    
    pub fn get_pnl(&self, current_price: f64) -> f64 {
        if self.side == "Buy" {
            (current_price - self.entry_price) * self.quantity
        } else {
            (self.entry_price - current_price) * self.quantity
        }
    }
    
    pub fn get_pnl_bps(&self, current_price: f64) -> f64 {
        let pnl_pct = if self.side == "Buy" {
            (current_price - self.entry_price) / self.entry_price
        } else {
            (self.entry_price - current_price) / self.entry_price
        };
        pnl_pct * 10000.0 // Convert to basis points
    }
}

/// Tracks positions for multiple symbols
pub struct PositionTracker {
    positions: HashMap<String, Vec<Position>>,
    net_positions: HashMap<String, f64>,
}

impl PositionTracker {
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
            net_positions: HashMap::new(),
        }
    }
    
    pub fn add_position(&mut self, position: Position) {
        let symbol = position.symbol.clone();
        let quantity = if position.side == "Buy" { 
            position.quantity 
        } else { 
            -position.quantity 
        };
        
        self.positions
            .entry(symbol.clone())
            .or_insert_with(Vec::new)
            .push(position);
            
        *self.net_positions.entry(symbol).or_insert(0.0) += quantity;
    }
    
    pub fn close_position(&mut self, symbol: &str, quantity: f64, side: &str) -> Result<Vec<Position>> {
        let positions = self.positions.get_mut(symbol)
            .ok_or_else(|| TradeError::TradeNotFound(format!("No positions for symbol {}", symbol)))?;
            
        let mut closed_positions = Vec::new();
        let mut remaining_to_close = quantity;
        let mut indices_to_remove = Vec::new();
        
        // Close positions FIFO
        for (i, pos) in positions.iter_mut().enumerate() {
            if remaining_to_close <= 0.0 {
                break;
            }
            
            // Check if this position matches the closing side
            let is_closing = (side == "Sell" && pos.side == "Buy") || 
                           (side == "Buy" && pos.side == "Sell");
                           
            if is_closing {
                if pos.quantity <= remaining_to_close {
                    closed_positions.push(pos.clone());
                    indices_to_remove.push(i);
                    remaining_to_close -= pos.quantity;
                } else {
                    let mut closed_pos = pos.clone();
                    closed_pos.quantity = remaining_to_close;
                    closed_positions.push(closed_pos);
                    pos.quantity -= remaining_to_close;
                    remaining_to_close = 0.0;
                }
            }
        }
        
        // Remove fully closed positions
        for i in indices_to_remove.into_iter().rev() {
            positions.remove(i);
        }
        
        // Update net position
        let quantity_delta = if side == "Buy" { quantity } else { -quantity };
        *self.net_positions.get_mut(symbol).unwrap() += quantity_delta;
        
        Ok(closed_positions)
    }
    
    pub fn get_net_position(&self, symbol: &str) -> f64 {
        self.net_positions.get(symbol).copied().unwrap_or(0.0)
    }
    
    pub fn get_positions(&self, symbol: &str) -> Option<&Vec<Position>> {
        self.positions.get(symbol)
    }
    
    pub fn get_all_positions(&self) -> &HashMap<String, Vec<Position>> {
        &self.positions
    }
    
    pub fn calculate_average_entry_price(&self, symbol: &str) -> f64 {
        if let Some(positions) = self.positions.get(symbol) {
            if positions.is_empty() {
                return 0.0;
            }
            
            let total_value: f64 = positions.iter()
                .map(|pos| pos.entry_price * pos.quantity)
                .sum();
            let total_quantity: f64 = positions.iter()
                .map(|pos| pos.quantity)
                .sum();
                
            if total_quantity > 0.0 {
                total_value / total_quantity
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
    
    pub fn reset(&mut self) {
        self.positions.clear();
        self.net_positions.clear();
    }
}

impl Default for PositionTracker {
    fn default() -> Self {
        Self::new()
    }
}