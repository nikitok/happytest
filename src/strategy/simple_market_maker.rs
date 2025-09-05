use crate::core::{Trade, OrderBook};
use crate::strategy::Strategy;
use log::info;

/// Simple Market Maker - classic textbook market making strategy
/// Places bid and ask orders at a fixed spread from mid price
/// No position management, no inventory tracking, just pure market making
pub struct SimpleMarketMaker {
    symbol: String,
    order_size: f64,
    spread_bps: f64,
    last_side: String,
    trades_count: usize,
}

impl SimpleMarketMaker {
    pub fn new(symbol: String, order_size: f64, spread_bps: f64) -> Self {
        info!("Initializing Simple Market Maker for {} with order_size={}, spread_bps={}", 
              symbol, order_size, spread_bps);
        
        Self {
            symbol,
            order_size,
            spread_bps,
            last_side: "Buy".to_string(),
            trades_count: 0,
        }
    }
    
    pub fn propose_trade(&mut self, order_book: &OrderBook) -> Option<Trade> {
        // Get mid price
        let mid_price = order_book.mid_price();
        if mid_price <= 0.0 {
            return None;
        }
        
        // Calculate spread
        let spread_factor = self.spread_bps / 10000.0;
        
        // Alternate between bid and ask orders
        let trade = if self.last_side == "Buy" {
            // Place ask order
            let ask_price = mid_price * (1.0 + spread_factor);
            self.last_side = "Sell".to_string();
            
            info!("Simple MM: Placing ASK @ {:.4} (mid: {:.4}, spread: {} bps)", 
                  ask_price, mid_price, self.spread_bps);
            
            Trade::new(
                order_book.current_time,
                order_book.symbol.clone(),
                "Sell".to_string(),
                ask_price,
                self.order_size,
            )
        } else {
            // Place bid order
            let bid_price = mid_price * (1.0 - spread_factor);
            self.last_side = "Buy".to_string();
            
            info!("Simple MM: Placing BID @ {:.4} (mid: {:.4}, spread: {} bps)", 
                  bid_price, mid_price, self.spread_bps);
            
            Trade::new(
                order_book.current_time,
                order_book.symbol.clone(),
                "Buy".to_string(),
                bid_price,
                self.order_size,
            )
        };
        
        self.trades_count += 1;
        Some(trade)
    }
    
    pub fn update_position(&mut self, trade: &Trade, filled: bool) {
        if filled {
            info!("Simple MM: Trade #{} filled - {} {} @ {}", 
                  self.trades_count, trade.side, trade.quantity, trade.price);
        }
        // No position tracking in simple strategy
    }
}

impl Strategy for SimpleMarketMaker {
    fn name(&self) -> &str {
        "Simple Market Maker"
    }
    
    fn propose_trade(&mut self, order_book: &OrderBook) -> Option<Trade> {
        self.propose_trade(order_book)
    }
    
    fn update_position(&mut self, trade: &Trade, filled: bool) {
        self.update_position(trade, filled)
    }
    
    fn get_position(&self, _symbol: &str) -> f64 {
        // Simple strategy doesn't track positions
        0.0
    }
    
    fn reset(&mut self) {
        self.last_side = "Buy".to_string();
        self.trades_count = 0;
        info!("Simple Market Maker reset");
    }
}