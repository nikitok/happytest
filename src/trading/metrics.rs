use std::collections::HashMap;
use crate::core::{Trade, ClosedTrade};

#[derive(Debug, Clone)]
pub struct TradingMetrics {
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub total_pnl: f64,
    pub max_drawdown: f64,
    pub sharpe_ratio: f64,
    pub win_rate: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
}

impl Default for TradingMetrics {
    fn default() -> Self {
        Self {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            total_pnl: 0.0,
            max_drawdown: 0.0,
            sharpe_ratio: 0.0,
            win_rate: 0.0,
            avg_win: 0.0,
            avg_loss: 0.0,
            profit_factor: 0.0,
        }
    }
}

pub struct MetricsCalculator {
    closed_trades: Vec<ClosedTrade>,
    cumulative_pnl: Vec<f64>,
}

impl MetricsCalculator {
    pub fn new() -> Self {
        Self {
            closed_trades: Vec::new(),
            cumulative_pnl: Vec::new(),
        }
    }
    
    pub fn add_closed_trade(&mut self, trade: ClosedTrade) {
        self.closed_trades.push(trade);
        
        let cumulative = if let Some(last) = self.cumulative_pnl.last() {
            last + self.closed_trades.last().unwrap().pnl
        } else {
            self.closed_trades.last().unwrap().pnl
        };
        self.cumulative_pnl.push(cumulative);
    }
    
    pub fn calculate_metrics(&self) -> TradingMetrics {
        if self.closed_trades.is_empty() {
            return TradingMetrics::default();
        }
        
        let total_trades = self.closed_trades.len();
        let winning_trades = self.closed_trades.iter().filter(|t| t.pnl > 0.0).count();
        let losing_trades = self.closed_trades.iter().filter(|t| t.pnl < 0.0).count();
        
        let total_pnl: f64 = self.closed_trades.iter().map(|t| t.pnl).sum();
        
        let wins: Vec<f64> = self.closed_trades.iter()
            .filter(|t| t.pnl > 0.0)
            .map(|t| t.pnl)
            .collect();
        
        let losses: Vec<f64> = self.closed_trades.iter()
            .filter(|t| t.pnl < 0.0)
            .map(|t| t.pnl.abs())
            .collect();
        
        let avg_win = if !wins.is_empty() {
            wins.iter().sum::<f64>() / wins.len() as f64
        } else {
            0.0
        };
        
        let avg_loss = if !losses.is_empty() {
            losses.iter().sum::<f64>() / losses.len() as f64
        } else {
            0.0
        };
        
        let win_rate = if total_trades > 0 {
            winning_trades as f64 / total_trades as f64
        } else {
            0.0
        };
        
        let profit_factor = if losses.iter().sum::<f64>() > 0.0 {
            wins.iter().sum::<f64>() / losses.iter().sum::<f64>()
        } else if !wins.is_empty() {
            f64::INFINITY
        } else {
            0.0
        };
        
        let max_drawdown = self.calculate_max_drawdown();
        let sharpe_ratio = self.calculate_sharpe_ratio();
        
        TradingMetrics {
            total_trades,
            winning_trades,
            losing_trades,
            total_pnl,
            max_drawdown,
            sharpe_ratio,
            win_rate,
            avg_win,
            avg_loss,
            profit_factor,
        }
    }
    
    fn calculate_max_drawdown(&self) -> f64 {
        if self.cumulative_pnl.is_empty() {
            return 0.0;
        }
        
        let mut max_drawdown = 0.0;
        let mut peak = self.cumulative_pnl[0];
        
        for &value in &self.cumulative_pnl {
            if value > peak {
                peak = value;
            }
            let drawdown = peak - value;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
        
        max_drawdown
    }
    
    fn calculate_sharpe_ratio(&self) -> f64 {
        if self.closed_trades.len() < 2 {
            return 0.0;
        }
        
        let returns: Vec<f64> = self.closed_trades.iter()
            .map(|t| t.pnl / t.quantity / t.open_price)
            .collect();
        
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        
        let variance = returns.iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>() / (returns.len() - 1) as f64;
        
        let std_dev = variance.sqrt();
        
        if std_dev > 0.0 {
            mean_return / std_dev * (252.0_f64).sqrt() // Annualized
        } else {
            0.0
        }
    }
    
    pub fn get_cumulative_pnl(&self) -> &[f64] {
        &self.cumulative_pnl
    }
    
    pub fn get_closed_trades(&self) -> &[ClosedTrade] {
        &self.closed_trades
    }
}

impl Default for MetricsCalculator {
    fn default() -> Self {
        Self::new()
    }
}