use crate::core::{Trade, OrderBook, PnLResult, ClosedTrade, CapitalMetrics, TradeState};
use std::collections::HashMap;
use log::info;
use comfy_table::Table;

pub struct TradeDashboard {
    pub trade_state: TradeState,
    positions: HashMap<String, f64>,
    avg_prices: HashMap<String, f64>,
    capital_history: Vec<f64>,
    pnl_history: Vec<f64>,
    timestamp_history: Vec<i64>,
    margin_history: Vec<f64>,
    open_positions_value_history: Vec<f64>,
    max_order_volume: f64,
    margin_rate: f64,
}

impl TradeDashboard {
    pub fn new(trade_state: TradeState, max_order_volume: f64, margin_rate: f64) -> Self {
        Self {
            trade_state,
            positions: HashMap::new(),
            avg_prices: HashMap::new(),
            capital_history: Vec::new(),
            pnl_history: Vec::new(),
            timestamp_history: Vec::new(),
            margin_history: Vec::new(),
            open_positions_value_history: Vec::new(),
            max_order_volume,
            margin_rate,
        }
    }

    pub fn pnl(&mut self, symbol: &str) -> HashMap<String, PnLResult> {
        let mut pnl_results = HashMap::new();
        
        let trades = self.trade_state.get_trades_history();
        info!("Calculating PNL for {} with {} trades", symbol, trades.len());
        
        let pnl_result = self.process_trades(&trades, symbol);
        pnl_results.insert(symbol.to_string(), pnl_result);
        
        pnl_results
    }

    fn process_trades(&self, trades: &[&Trade], symbol: &str) -> PnLResult {
        let mut total_pnl = 0.0;
        let mut closed_trades = Vec::new();
        let mut positions: HashMap<String, Vec<(f64, f64)>> = HashMap::new(); // symbol -> Vec<(quantity, price)>
        
        for trade in trades {
            if trade.symbol != symbol {
                continue;
            }
            
            let pos_list = positions.entry(trade.symbol.clone()).or_insert_with(Vec::new);
            
            if trade.side == "Buy" {
                pos_list.push((trade.quantity, trade.price));
            } else if trade.side == "Sell" {
                let mut remaining_quantity = trade.quantity;
                let mut i = 0;
                
                while remaining_quantity > 0.0 && i < pos_list.len() {
                    let (pos_quantity, pos_price) = pos_list[i];
                    
                    if pos_quantity > 0.0 {
                        let close_quantity = remaining_quantity.min(pos_quantity);
                        let pnl = (trade.price - pos_price) * close_quantity;
                        total_pnl += pnl;
                        
                        closed_trades.push(ClosedTrade {
                            open_side: "Buy".to_string(),
                            quantity: close_quantity,
                            open_price: pos_price,
                            close_side: "Sell".to_string(),
                            close_price: trade.price,
                            pnl,
                        });
                        
                        pos_list[i].0 -= close_quantity;
                        remaining_quantity -= close_quantity;
                        
                        if pos_list[i].0 == 0.0 {
                            pos_list.remove(i);
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
            }
        }
        
        // Calculate unrealized PnL
        let mut unrealized_pnl = 0.0;
        let last_price = trades.last().map(|t| t.price).unwrap_or(0.0);
        
        for (_, pos_list) in &positions {
            for (quantity, price) in pos_list {
                if *quantity > 0.0 {
                    unrealized_pnl += (last_price - price) * quantity;
                }
            }
        }
        
        PnLResult {
            total_pnl,
            unrealized_pnl,
            closed_trades,
            total_fees: 0.0,
        }
    }

    pub fn calculate_trading_costs(&self, _symbol: &str) -> HashMap<&str, f64> {
        let trades = self.trade_state.get_trades_history();
        let failed_trades = self.trade_state.get_failed_trades();
        
        let total_attempted = trades.len() + failed_trades.len();
        let fill_rate = if total_attempted > 0 {
            trades.len() as f64 / total_attempted as f64
        } else {
            0.0
        };
        
        let buy_trades = trades.iter().filter(|t| t.side.to_uppercase() == "BUY").count();
        let sell_trades = trades.iter().filter(|t| t.side.to_uppercase() == "SELL").count();
        
        let mut costs = HashMap::new();
        costs.insert("total_attempted_trades", total_attempted as f64);
        costs.insert("successful_trades", trades.len() as f64);
        costs.insert("failed_trades", failed_trades.len() as f64);
        costs.insert("buy_trades", buy_trades as f64);
        costs.insert("sell_trades", sell_trades as f64);
        costs.insert("fill_rate", fill_rate);
        
        costs
    }

    pub fn recalculate_capital_metrics(&mut self, symbol: &str) {
        self.capital_history.clear();
        self.pnl_history.clear();
        self.timestamp_history.clear();
        self.margin_history.clear();
        self.open_positions_value_history.clear();
        
        self.positions.clear();
        self.avg_prices.clear();
        
        let all_trades: Vec<Trade> = self.trade_state.get_trades_history().into_iter().cloned().collect();
        let orderbooks: Vec<OrderBook> = self.trade_state.get_orderbooks().clone();
        let mut orderbook_idx = 0;
        
        for trade in all_trades {
            if trade.symbol != symbol {
                continue;
            }
            
            // Find the closest orderbook after this trade
            while orderbook_idx < orderbooks.len() && orderbooks[orderbook_idx].current_time < trade.time {
                orderbook_idx += 1;
            }
            
            let current_price = if orderbook_idx < orderbooks.len() {
                orderbooks[orderbook_idx].mid_price()
            } else {
                trade.price
            };
            
            // Update position tracking
            let quantity = self.positions.get(&trade.symbol).cloned().unwrap_or(0.0);
            let avg_price = self.avg_prices.get(&trade.symbol).cloned().unwrap_or(0.0);
            
            let trade_quantity = if trade.side.to_lowercase() == "buy" {
                trade.quantity
            } else {
                -trade.quantity
            };
            
            // Calculate new average price
            let new_avg_price = if quantity == 0.0 {
                trade.price
            } else if (quantity > 0.0 && trade_quantity > 0.0) || (quantity < 0.0 && trade_quantity < 0.0) {
                let total_value = quantity.abs() * avg_price + trade_quantity.abs() * trade.price;
                let total_quantity = quantity.abs() + trade_quantity.abs();
                total_value / total_quantity
            } else {
                avg_price
            };
            
            // Update quantity
            let new_quantity = quantity + trade_quantity;
            
            if new_quantity.abs() < 1e-8 {
                self.positions.insert(trade.symbol.clone(), 0.0);
                self.avg_prices.insert(trade.symbol.clone(), 0.0);
            } else {
                self.positions.insert(trade.symbol.clone(), new_quantity);
                self.avg_prices.insert(trade.symbol.clone(), new_avg_price);
            }
            
            // Calculate capital metrics
            self.calculate_capital_metrics(trade.time, &HashMap::from([(symbol.to_string(), current_price)]));
        }
    }

    fn calculate_capital_metrics(&mut self, timestamp: i64, current_prices: &HashMap<String, f64>) {
        let mut total_unrealized_pnl = 0.0;
        let mut total_margin_requirement = 0.0;
        let mut total_open_positions_value = 0.0;
        
        for (symbol, quantity) in &self.positions {
            if quantity.abs() < 1e-8 {
                continue;
            }
            
            let current_price = current_prices.get(symbol)
                .unwrap_or(self.avg_prices.get(symbol).unwrap_or(&0.0));
            let avg_price = self.avg_prices.get(symbol).unwrap_or(&0.0);
            
            // Unrealized PnL
            let unrealized_pnl = if *quantity > 0.0 {
                (current_price - avg_price) * quantity
            } else {
                (avg_price - current_price) * quantity.abs()
            };
            
            total_unrealized_pnl += unrealized_pnl;
            
            // Open positions value
            let position_value = quantity.abs() * current_price;
            total_open_positions_value += position_value;
            
            // Margin requirement
            let margin_requirement = position_value * self.margin_rate;
            total_margin_requirement += margin_requirement;
        }
        
        // Required capital = margin + buffer for covering losses
        let safety_buffer = total_open_positions_value * 0.02;
        let required_capital = total_margin_requirement + 0.0_f64.max(-total_unrealized_pnl) + safety_buffer;
        
        // Save history
        self.capital_history.push(required_capital);
        self.pnl_history.push(total_unrealized_pnl);
        self.timestamp_history.push(timestamp);
        self.margin_history.push(total_margin_requirement);
        self.open_positions_value_history.push(total_open_positions_value);
    }

    pub fn get_capital_metrics(&mut self, symbol: &str) -> CapitalMetrics {
        self.recalculate_capital_metrics(symbol);
        
        if self.capital_history.is_empty() {
            return CapitalMetrics {
                max_required_capital: 0.0,
                max_drawdown: 0.0,
                max_open_positions_value: 0.0,
                average_capital_utilization: 0.0,
                peak_margin_requirement: 0.0,
                max_unrealized_loss: 0.0,
            };
        }
        
        let max_required_capital = self.capital_history.iter().fold(0.0_f64, |a, &b| a.max(b));
        let max_drawdown = self.pnl_history.iter().fold(0.0_f64, |a, &b| a.min(b));
        let max_open_positions_value = self.open_positions_value_history.iter().fold(0.0_f64, |a, &b| a.max(b));
        let average_capital_utilization = self.capital_history.iter().sum::<f64>() / self.capital_history.len() as f64;
        let peak_margin_requirement = self.margin_history.iter().fold(0.0_f64, |a, &b| a.max(b));
        let max_unrealized_loss = self.pnl_history.iter().fold(0.0_f64, |a, &b| a.min(b));
        
        CapitalMetrics {
            max_required_capital,
            max_drawdown: max_drawdown.abs(),
            max_open_positions_value,
            average_capital_utilization,
            peak_margin_requirement,
            max_unrealized_loss: max_unrealized_loss.abs(),
        }
    }

    pub fn print_pnl_metrics(&self, symbol: &str, pnl_results: &HashMap<String, PnLResult>) -> HashMap<&str, f64> {
        let costs = self.calculate_trading_costs(symbol);
        
        let pnl_result = pnl_results.get(symbol);
        if pnl_result.is_none() {
            return HashMap::new();
        }
        
        let pnl_result = pnl_result.unwrap();
        let total_pnl = pnl_result.total_pnl;
        let total_unrealized_pnl = pnl_result.unrealized_pnl;
        let total_pnl_with_unrealized = total_pnl + total_unrealized_pnl;
        let total_fees = pnl_result.total_fees;
        
        let mut table = Table::new();
        table.set_header(vec!["Metric", "Value"]);
        table.add_row(vec!["Total realized PnL", &format!("${:.2}", total_pnl)]);
        table.add_row(vec!["Trading fees", &format!("${:.2}", total_fees)]);
        table.add_row(vec!["Net realized PnL", &format!("${:.2}", total_pnl)]);
        table.add_row(vec!["Unrealized PnL", &format!("${:.2}", total_unrealized_pnl)]);
        table.add_row(vec!["Total PnL", &format!("${:.2}", total_pnl_with_unrealized)]);
        table.add_row(vec!["Fill rate", &format!("{:.2}%", costs.get("fill_rate").unwrap_or(&0.0) * 100.0)]);
        table.add_row(vec!["Buy trades", &costs.get("buy_trades").unwrap_or(&0.0).to_string()]);
        table.add_row(vec!["Sell trades", &costs.get("sell_trades").unwrap_or(&0.0).to_string()]);
        
        info!("P&L METRICS + EXECUTION METRICS");
        info!("{}", table);
        
        let mut summary = HashMap::new();
        summary.insert("total_pnl", total_pnl);
        summary.insert("total_fees", total_fees);
        summary.insert("unrealized_pnl", total_unrealized_pnl);
        summary.insert("total_pnl_with_unrealized", total_pnl_with_unrealized);
        summary.insert("buy_trades", *costs.get("buy_trades").unwrap_or(&0.0));
        summary.insert("sell_trades", *costs.get("sell_trades").unwrap_or(&0.0));
        summary.insert("fill_rate", *costs.get("fill_rate").unwrap_or(&0.0));
        
        summary
    }

    pub fn print_capital_metrics(&self, capital_metrics: &HashMap<String, CapitalMetrics>) {
        for (symbol, metrics) in capital_metrics {
            let mut table = Table::new();
            table.set_header(vec!["Metric", "Value"]);
            table.add_row(vec!["Max required capital", &format!("${:.2}", metrics.max_required_capital)]);
            table.add_row(vec!["Max drawdown", &format!("${:.2}", metrics.max_drawdown)]);
            table.add_row(vec!["Peak margin requirement", &format!("${:.2}", metrics.peak_margin_requirement)]);
            table.add_row(vec!["Max open positions value", &format!("${:.2}", metrics.max_open_positions_value)]);
            table.add_row(vec!["Average capital utilization", &format!("${:.2}", metrics.average_capital_utilization)]);
            table.add_row(vec!["Max unrealized loss", &format!("${:.2}", metrics.max_unrealized_loss)]);
            
            info!("\nCAPITAL REQUIREMENTS FOR {}", symbol);
            info!("{}", table);
        }
    }

    pub fn to_console(&self, symbol: &str, pnl_results: &HashMap<String, PnLResult>, capital_metrics: &HashMap<String, CapitalMetrics>) {
        info!("\nComplete summary for {}:", symbol);
        
        self.print_pnl_metrics(symbol, pnl_results);
        
        if !capital_metrics.is_empty() {
            self.print_capital_metrics(capital_metrics);
        }
    }
}