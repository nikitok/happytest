use crate::core::{Trade, OrderBook};
use crate::strategy::Strategy;
use std::collections::VecDeque;
use log::info;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GptMarketMakerConfig {
    pub fix_order_volume: f64,
    pub vwap_window: usize,
    pub obi_threshold: f64,
    pub max_inventory: f64,
    pub use_limit_orders: bool,
    pub limit_order_spread_bps: f64,
    // Position management parameters
    pub take_profit_bps: f64,
    pub stop_loss_bps: f64,
    pub max_position_age_ms: i64,
    pub inventory_reduction_threshold: f64,
    pub aggressive_close_threshold: f64,
    pub min_profit_bps: f64,
    // Volatility detection parameters
    pub volatility_window: usize,
    pub max_volatility_threshold: f64,
    pub volatility_cooldown_ms: i64,
    // Momentum filter parameters
    pub momentum_window: usize,
    pub momentum_threshold: f64,
    pub momentum_cooldown_ms: i64,
}

impl Default for GptMarketMakerConfig {
    fn default() -> Self {
        Self {
            fix_order_volume: 0.005,
            vwap_window: 100,
            obi_threshold: 0.1,
            max_inventory: 10.0,
            use_limit_orders: true,
            limit_order_spread_bps: 5.0,
            take_profit_bps: 20.0,
            stop_loss_bps: 50.0,
            max_position_age_ms: 300000,
            inventory_reduction_threshold: 0.7,
            aggressive_close_threshold: 0.9,
            min_profit_bps: 5.0,
            volatility_window: 30,
            max_volatility_threshold: 0.000005,
            volatility_cooldown_ms: 5000,
            momentum_window: 10,
            momentum_threshold: 0.0015,
            momentum_cooldown_ms: 3000,
        }
    }
}

#[derive(Debug, Clone)]
struct Position {
    quantity: f64,
    entry_price: f64,
    entry_time: i64,
    side: String,
}

impl Position {
    fn get_pnl_bps(&self, current_price: f64) -> f64 {
        if self.side == "Buy" {
            ((current_price - self.entry_price) / self.entry_price) * 10000.0
        } else {
            ((self.entry_price - current_price) / self.entry_price) * 10000.0
        }
    }

    fn get_age_ms(&self, current_time: i64) -> i64 {
        current_time - self.entry_time
    }
}

pub struct GptMarketMaker {
    symbol: String,
    config: GptMarketMakerConfig,
    prices: VecDeque<f64>,
    volumes: VecDeque<f64>,
    positions: Vec<Position>,
    net_inventory: f64,
    avg_entry_price: f64,
    price_history: VecDeque<f64>,
    last_high_volatility_time: i64,
    momentum_prices: VecDeque<f64>,
    last_strong_momentum_time: i64,
}

impl GptMarketMaker {
    pub fn new(symbol: String, config: GptMarketMakerConfig) -> Self {
        let vwap_window = config.vwap_window;
        let volatility_window = config.volatility_window;
        let momentum_window = config.momentum_window;
        
        Self {
            symbol,
            config,
            prices: VecDeque::with_capacity(vwap_window),
            volumes: VecDeque::with_capacity(vwap_window),
            positions: Vec::new(),
            net_inventory: 0.0,
            avg_entry_price: 0.0,
            price_history: VecDeque::with_capacity(volatility_window),
            last_high_volatility_time: 0,
            momentum_prices: VecDeque::with_capacity(momentum_window),
            last_strong_momentum_time: 0,
        }
    }

    fn update_vwap(&mut self, price: f64, volume: f64) -> Option<f64> {
        self.prices.push_back(price * volume);
        self.volumes.push_back(volume);
        
        if self.prices.len() > self.config.vwap_window {
            self.prices.pop_front();
            self.volumes.pop_front();
        }
        
        if self.volumes.len() < self.config.vwap_window {
            return None;
        }
        
        let sum_prices: f64 = self.prices.iter().sum();
        let sum_volumes: f64 = self.volumes.iter().sum();
        
        if sum_volumes == 0.0 {
            None
        } else {
            Some(sum_prices / sum_volumes)
        }
    }

    fn compute_obi(&self, order_book: &OrderBook) -> f64 {
        if order_book.bids.is_empty() || order_book.asks.is_empty() {
            return 0.0;
        }

        let bid_vol: f64 = order_book.bids.iter().take(5).map(|(_, v)| v).sum();
        let ask_vol: f64 = order_book.asks.iter().take(5).map(|(_, v)| v).sum();

        if bid_vol + ask_vol == 0.0 {
            return 0.0;
        }

        (bid_vol - ask_vol) / (bid_vol + ask_vol)
    }

    fn calculate_volatility(&self) -> f64 {
        if self.price_history.len() < 2 {
            return 0.0;
        }

        let prices: Vec<f64> = self.price_history.iter().cloned().collect();
        let mut returns = Vec::new();
        
        for i in 1..prices.len() {
            let ret = (prices[i] - prices[i - 1]) / prices[i - 1];
            returns.push(ret);
        }

        if returns.is_empty() {
            return 0.0;
        }

        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns.iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>() / returns.len() as f64;
        
        variance.sqrt()
    }

    fn calculate_momentum(&self) -> f64 {
        if self.momentum_prices.len() < 2 {
            return 0.0;
        }

        let prices: Vec<f64> = self.momentum_prices.iter().cloned().collect();
        (prices[prices.len() - 1] - prices[0]) / prices[0]
    }

    fn check_market_conditions(&mut self, current_time: i64) -> (bool, String) {
        // Check volatility cooldown
        if current_time - self.last_high_volatility_time < self.config.volatility_cooldown_ms {
            let time_left = (self.config.volatility_cooldown_ms - (current_time - self.last_high_volatility_time)) as f64 / 1000.0;
            return (false, format!("VOLATILITY_COOLDOWN: {:.1}s remaining", time_left));
        }

        // Check momentum cooldown
        if current_time - self.last_strong_momentum_time < self.config.momentum_cooldown_ms {
            let time_left = (self.config.momentum_cooldown_ms - (current_time - self.last_strong_momentum_time)) as f64 / 1000.0;
            return (false, format!("MOMENTUM_COOLDOWN: {:.1}s remaining", time_left));
        }

        // Calculate current volatility
        let volatility = self.calculate_volatility();
        if volatility > self.config.max_volatility_threshold {
            self.last_high_volatility_time = current_time;
            return (false, format!("HIGH_VOLATILITY: {:.4} > {:.4}", volatility, self.config.max_volatility_threshold));
        }

        // Calculate current momentum
        let momentum = self.calculate_momentum();
        if momentum.abs() > self.config.momentum_threshold {
            self.last_strong_momentum_time = current_time;
            return (false, format!("STRONG_MOMENTUM: {:.4} > {:.4}", momentum, self.config.momentum_threshold));
        }

        (true, "OK".to_string())
    }

    fn calculate_average_entry_price(&self) -> f64 {
        if self.positions.is_empty() {
            return 0.0;
        }

        let total_value: f64 = self.positions.iter()
            .map(|pos| pos.entry_price * pos.quantity)
            .sum();
        let total_quantity: f64 = self.positions.iter()
            .map(|pos| pos.quantity)
            .sum();

        if total_quantity > 0.0 {
            total_value / total_quantity
        } else {
            0.0
        }
    }

    fn should_close_position(&self, mid_price: f64, current_time: i64) -> (bool, String) {
        if self.net_inventory == 0.0 {
            return (false, String::new());
        }

        let mut total_pnl_bps = 0.0;
        let mut oldest_position_age = 0;

        for pos in &self.positions {
            let pnl_bps = pos.get_pnl_bps(mid_price);
            total_pnl_bps += pnl_bps * (pos.quantity / self.net_inventory.abs());

            let age = pos.get_age_ms(current_time);
            oldest_position_age = oldest_position_age.max(age);
        }

        let inventory_ratio = self.net_inventory.abs() / self.config.max_inventory;

        // Check various closing conditions
        if total_pnl_bps >= self.config.take_profit_bps {
            return (true, format!("TAKE_PROFIT: {:.1} bps", total_pnl_bps));
        }

        if total_pnl_bps <= -self.config.stop_loss_bps {
            return (true, format!("STOP_LOSS: {:.1} bps", total_pnl_bps));
        }

        if oldest_position_age > self.config.max_position_age_ms {
            return (true, format!("POSITION_AGE: {:.1}s", oldest_position_age as f64 / 1000.0));
        }

        if inventory_ratio >= self.config.inventory_reduction_threshold {
            if total_pnl_bps >= self.config.min_profit_bps {
                return (true, format!("INVENTORY_REDUCTION: {:.1}% full, {:.1} bps profit", inventory_ratio * 100.0, total_pnl_bps));
            }
        }

        if inventory_ratio >= self.config.aggressive_close_threshold {
            if total_pnl_bps >= -self.config.min_profit_bps {
                return (true, format!("AGGRESSIVE_CLOSE: {:.1}% full, {:.1} bps", inventory_ratio * 100.0, total_pnl_bps));
            }
        }

        (false, String::new())
    }

    pub fn propose_trade(&mut self, order_book: &OrderBook) -> Option<Trade> {
        if order_book.bids.is_empty() || order_book.asks.is_empty() {
            return None;
        }

        let best_bid = order_book.bids[0].0;
        let bid_vol = order_book.bids[0].1;
        let best_ask = order_book.asks[0].0;
        let ask_vol = order_book.asks[0].1;

        let mid_price = (best_bid + best_ask) / 2.0;
        let current_time = order_book.current_time;

        // Update price histories
        self.price_history.push_back(mid_price);
        if self.price_history.len() > self.config.volatility_window {
            self.price_history.pop_front();
        }
        
        self.momentum_prices.push_back(mid_price);
        if self.momentum_prices.len() > self.config.momentum_window {
            self.momentum_prices.pop_front();
        }

        // Update VWAP
        let vwap = self.update_vwap(mid_price, bid_vol + ask_vol);
        if vwap.is_none() {
            return None;
        }
        let vwap = vwap.unwrap();

        // Check market conditions
        let (can_trade, market_condition) = self.check_market_conditions(current_time);

        // Check if we should close positions
        let (should_close, close_reason) = self.should_close_position(mid_price, current_time);

        if should_close && self.net_inventory != 0.0 {
            let (side, limit_price) = if self.net_inventory > 0.0 {
                // We're long, so sell to close
                let price = if self.config.use_limit_orders {
                    best_bid.max(self.avg_entry_price * (1.0 + self.config.min_profit_bps / 10000.0))
                } else {
                    best_bid
                };
                ("Sell", price)
            } else {
                // We're short, so buy to close
                let price = if self.config.use_limit_orders {
                    best_ask.min(self.avg_entry_price * (1.0 - self.config.min_profit_bps / 10000.0))
                } else {
                    best_ask
                };
                ("Buy", price)
            };

            let quantity = self.config.fix_order_volume.min(self.net_inventory.abs());

            let trade = Trade::new(
                current_time,
                self.symbol.clone(),
                side.to_string(),
                limit_price,
                quantity,
            );

            info!("GPT Maker CLOSING: {} {} @ {:.4} (reason: {}, inventory: {})",
                side, quantity, limit_price, close_reason, self.net_inventory);

            return Some(trade);
        }

        // Check if we can open new positions
        if !can_trade {
            info!("GPT Maker PAUSED: {}", market_condition);
            return None;
        }

        // Regular trading logic
        let obi = self.compute_obi(order_book);
        let inventory_ratio = self.net_inventory.abs() / self.config.max_inventory;
        let adjusted_obi_threshold = self.config.obi_threshold * (1.0 + inventory_ratio);

        if obi > adjusted_obi_threshold && 
           mid_price < vwap && 
           self.net_inventory < self.config.max_inventory * self.config.inventory_reduction_threshold {
            // Buy signal
            let limit_price = if self.config.use_limit_orders {
                best_bid * (1.0 - self.config.limit_order_spread_bps / 10000.0)
            } else {
                best_ask
            };

            let trade = Trade::new(
                current_time,
                self.symbol.clone(),
                "Buy".to_string(),
                limit_price,
                self.config.fix_order_volume,
            );

            info!("GPT Maker OPENING: Buy {} @ {:.4} (OBI: {:.3}, VWAP: {:.4}, inventory: {})",
                self.config.fix_order_volume, limit_price, obi, vwap, self.net_inventory);

            Some(trade)
        } else if obi < -adjusted_obi_threshold && 
                  mid_price > vwap && 
                  self.net_inventory > -self.config.max_inventory * self.config.inventory_reduction_threshold {
            // Sell signal
            let limit_price = if self.config.use_limit_orders {
                best_ask * (1.0 + self.config.limit_order_spread_bps / 10000.0)
            } else {
                best_bid
            };

            let trade = Trade::new(
                current_time,
                self.symbol.clone(),
                "Sell".to_string(),
                limit_price,
                self.config.fix_order_volume,
            );

            info!("GPT Maker OPENING: Sell {} @ {:.4} (OBI: {:.3}, VWAP: {:.4}, inventory: {})",
                self.config.fix_order_volume, limit_price, obi, vwap, self.net_inventory);

            Some(trade)
        } else {
            None
        }
    }

    pub fn update_position(&mut self, trade: &Trade, filled: bool) {
        if !filled {
            return;
        }

        // Check if this is a closing trade
        let is_closing = (self.net_inventory > 0.0 && trade.side == "Sell") ||
                        (self.net_inventory < 0.0 && trade.side == "Buy");

        if is_closing {
            // Remove positions being closed (FIFO)
            let mut remaining_to_close = trade.quantity;
            let mut positions_to_remove = Vec::new();

            for (i, pos) in self.positions.iter_mut().enumerate() {
                if remaining_to_close <= 0.0 {
                    break;
                }

                if (self.net_inventory > 0.0 && pos.side == "Buy") ||
                   (self.net_inventory < 0.0 && pos.side == "Sell") {
                    if pos.quantity <= remaining_to_close {
                        positions_to_remove.push(i);
                        remaining_to_close -= pos.quantity;
                    } else {
                        pos.quantity -= remaining_to_close;
                        remaining_to_close = 0.0;
                    }
                }
            }

            // Remove closed positions
            for i in positions_to_remove.into_iter().rev() {
                self.positions.remove(i);
            }
        } else {
            // Opening new position
            self.positions.push(Position {
                quantity: trade.quantity,
                entry_price: trade.price,
                entry_time: trade.time,
                side: trade.side.clone(),
            });
        }

        // Update net inventory
        if trade.side == "Buy" {
            self.net_inventory += trade.quantity;
        } else {
            self.net_inventory -= trade.quantity;
        }

        // Recalculate average entry price
        self.avg_entry_price = self.calculate_average_entry_price();
    }
}

impl Strategy for GptMarketMaker {
    fn name(&self) -> &str {
        "GPT Market Maker"
    }

    fn propose_trade(&mut self, order_book: &OrderBook) -> Option<Trade> {
        self.propose_trade(order_book)
    }

    fn update_position(&mut self, trade: &Trade, filled: bool) {
        self.update_position(trade, filled)
    }
    
    fn get_position(&self, symbol: &str) -> f64 {
        if self.symbol == symbol {
            self.net_inventory
        } else {
            0.0
        }
    }
    
    fn reset(&mut self) {
        self.prices.clear();
        self.volumes.clear();
        self.positions.clear();
        self.net_inventory = 0.0;
        self.avg_entry_price = 0.0;
        self.price_history.clear();
        self.last_high_volatility_time = 0;
        self.momentum_prices.clear();
        self.last_strong_momentum_time = 0;
    }
}