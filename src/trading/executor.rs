use crate::core::{Trade, TradeExecutor, ExecutionStats, Result};
use log::info;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

pub trait TradeEmitter {
    fn execute_trade(&mut self, trade: Option<Trade>) -> Option<Trade>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BacktestConfig {
    pub fill_rate: f64,
    pub slippage_bps: f64,
    pub rejection_rate: f64,
    pub margin_rate: f64,
    pub fix_order_volume: f64,
    pub min_spread_pct: f64,
    pub spread_percent: f64,
    pub max_order_volume: f64,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            fill_rate: 0.95,
            slippage_bps: 0.5,
            rejection_rate: 0.02,
            margin_rate: 0.1,
            fix_order_volume: 0.0,
            min_spread_pct: 0.1,
            spread_percent: 0.10,
            max_order_volume: 0.0,
        }
    }
}

pub struct BacktestTradeEmitter {
    config: BacktestConfig,
    rng: StdRng,
    stats: ExecutionStats,
}

impl BacktestTradeEmitter {
    pub fn new(config: BacktestConfig) -> Self {
        Self {
            config,
            rng: StdRng::from_entropy(),
            stats: ExecutionStats::default(),
        }
    }
}

impl TradeEmitter for BacktestTradeEmitter {
    fn execute_trade(&mut self, trade: Option<Trade>) -> Option<Trade> {
        if let Some(mut trade) = trade {
            self.stats.total_trades += 1;
            let random_value: f64 = self.rng.gen();
            
            // Check for rejection
            if random_value < self.config.rejection_rate {
                trade.status = "rejected".to_string();
                self.stats.rejected_trades += 1;
                return Some(trade);
            }
            
            // Check for fill
            if random_value < self.config.fill_rate {
                // Apply slippage
                let slippage_factor = 1.0 + (self.config.slippage_bps / 10000.0);
                
                let original_price = trade.price;
                if trade.side == "Buy" {
                    trade.price *= slippage_factor;
                } else {
                    trade.price /= slippage_factor;
                }
                
                let slippage = (trade.price - original_price).abs();
                self.stats.total_slippage += slippage;
                
                trade.status = "filled".to_string();
                self.stats.filled_trades += 1;
                info!("Trade executed: {} {} @ {} - Status: {}", 
                    trade.side, trade.quantity, trade.price, trade.status);
            } else {
                trade.status = "unfilled".to_string();
            }
            
            Some(trade)
        } else {
            None
        }
    }
}

impl TradeExecutor for BacktestTradeEmitter {
    fn execute_trade(&mut self, trade: Trade) -> Result<Trade> {
        Ok(TradeEmitter::execute_trade(self, Some(trade)).unwrap())
    }
    
    fn get_stats(&self) -> ExecutionStats {
        self.stats.clone()
    }
}