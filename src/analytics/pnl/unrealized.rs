use std::collections::HashMap;
use crate::core::Trade;

/// Calculate unrealized P&L for remaining open trades
///
/// # Arguments
/// * `open_trades` - Dictionary of open trades by asset
/// * `filled_orders` - List of Trade objects containing filled orders
///
/// # Returns
/// * `(unrealized_pnl, unrealized_pnl_by_asset, remaining_shares_by_asset)`
pub fn calculate_unrealized_pnl(
    open_trades: &HashMap<String, Vec<Trade>>,
    filled_orders: &[Trade],
) -> (f64, HashMap<String, f64>, HashMap<String, f64>) {
    let mut unrealized_pnl = 0.0;
    let mut unrealized_pnl_by_asset = HashMap::new();
    let mut remaining_shares_by_asset = HashMap::new();
    
    // Get the last price for each asset
    let mut last_sell_prices = HashMap::new();
    let mut last_buy_prices = HashMap::new();
    
    for order in filled_orders {
        let symbol = &order.symbol;
        let price = order.price;
        let side = &order.side;
        
        if side.to_lowercase() == "buy" {
            last_buy_prices.insert(symbol.clone(), price);
        } else {
            last_sell_prices.insert(symbol.clone(), price);
        }
    }
    
    // Calculate unrealized PnL and count remaining shares
    for (symbol, trades) in open_trades {
        let mut asset_unrealized_pnl = 0.0;
        let last_buy_price_by_symbol = last_buy_prices.get(symbol).copied().unwrap_or(0.0);
        let last_sell_price_by_symbol = last_sell_prices.get(symbol).copied().unwrap_or(0.0);
        let mut remaining_shares = 0.0;
        
        for trade in trades {
            if trade.side.to_uppercase() == "BUY" {
                // For buy positions, unrealized PnL is current value - cost
                asset_unrealized_pnl += (last_sell_price_by_symbol - trade.price) * trade.quantity;
                remaining_shares += trade.quantity;
            } else {
                // For sell positions, unrealized PnL is proceeds - current value
                asset_unrealized_pnl += (trade.price - last_buy_price_by_symbol) * trade.quantity;
                remaining_shares -= trade.quantity;  // Negative for short positions
            }
        }
        
        unrealized_pnl += asset_unrealized_pnl;
        unrealized_pnl_by_asset.insert(symbol.clone(), asset_unrealized_pnl);
        remaining_shares_by_asset.insert(symbol.clone(), remaining_shares);
    }
    
    (unrealized_pnl, unrealized_pnl_by_asset, remaining_shares_by_asset)
}

/// Calculate unrealized P&L for a single position
pub fn calculate_position_unrealized_pnl(
    position_quantity: f64,
    avg_price: f64,
    current_price: f64,
    is_long: bool,
) -> f64 {
    if is_long {
        (current_price - avg_price) * position_quantity
    } else {
        (avg_price - current_price) * position_quantity.abs()
    }
}

/// Get the last traded prices for each symbol
pub fn get_last_prices(trades: &[Trade]) -> HashMap<String, (f64, f64)> {
    let mut last_prices = HashMap::new();
    
    for trade in trades {
        let entry = last_prices.entry(trade.symbol.clone())
            .or_insert((0.0, 0.0));
        
        if trade.side.to_lowercase() == "buy" {
            entry.0 = trade.price; // last buy price
        } else {
            entry.1 = trade.price; // last sell price
        }
    }
    
    last_prices
}