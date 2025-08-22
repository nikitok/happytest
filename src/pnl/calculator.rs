use crate::core::{Trade, PnLResult};
use crate::pnl::{
    models::Method,
    fifo::FifoProcessor,
    position::PositionProcessor,
};
use std::collections::HashMap;
use comfy_table::Table;
use plotters::prelude::*;

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
    
    /// Generate P&L graphs for each symbol
    /// 
    /// # Arguments
    /// * `trades` - List of trades to analyze
    /// * `method` - Calculation method (Fifo or Position)
    /// * `output_dir` - Output directory for charts (default: "./data")
    /// * `prefix` - Filename prefix (default: "pnl_")
    pub fn graph(
        &self, 
        trades: &[Trade], 
        method: Method,
        output_dir: Option<&str>,
        prefix: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output_dir = output_dir.unwrap_or("./data");
        let prefix = prefix.unwrap_or("pnl_");
        
        // Create output directory if it doesn't exist
        std::fs::create_dir_all(output_dir)?;
        
        // Group trades by symbol
        let mut trades_by_symbol: HashMap<String, Vec<Trade>> = HashMap::new();
        
        for trade in trades {
            trades_by_symbol
                .entry(trade.symbol.clone())
                .or_default()
                .push(trade.clone());
        }
        
        // Store data for combined chart
        let mut all_symbol_data: Vec<(String, Vec<(i64, f64)>)> = Vec::new();
        
        // Process each symbol
        for (symbol, mut symbol_trades) in trades_by_symbol {
            // Sort trades by time
            symbol_trades.sort_by_key(|t| t.time);
            
            // Calculate cumulative P&L over time
            let mut cumulative_pnl = Vec::new();
            let mut timestamps = Vec::new();
            
            // Process trades incrementally
            for i in 1..=symbol_trades.len() {
                let trades_slice = &symbol_trades[0..i];
                let result = self.calculate(trades_slice, method);
                let current_pnl = result.total_pnl + result.unrealized_pnl;
                
                cumulative_pnl.push(current_pnl);
                timestamps.push(symbol_trades[i-1].time);
            }
            
            // Create the chart
            let filename = format!("{}/{}{}.png", output_dir, prefix, symbol);
            let root = BitMapBackend::new(&filename, (1024, 768)).into_drawing_area();
            root.fill(&WHITE)?;
            
            // Find min and max values for the chart
            let min_pnl = cumulative_pnl.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_pnl = cumulative_pnl.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let pnl_range = if (max_pnl - min_pnl).abs() < 1.0 {
                min_pnl - 100.0..max_pnl + 100.0
            } else {
                min_pnl * 1.1..max_pnl * 1.1
            };
            
            let min_time = *timestamps.first().unwrap_or(&0);
            let max_time = *timestamps.last().unwrap_or(&1);
            
            let mut chart = ChartBuilder::on(&root)
                .caption(&format!("P&L Chart for {}", symbol), ("sans-serif", 40).into_font())
                .margin(10)
                .x_label_area_size(40)
                .y_label_area_size(60)
                .build_cartesian_2d(min_time..max_time, pnl_range)?;
            
            chart.configure_mesh()
                .x_desc("Time")
                .y_desc("P&L ($)")
                .x_label_formatter(&|x| {
                    chrono::DateTime::from_timestamp_millis(*x)
                        .map(|dt| dt.format("%H:%M").to_string())
                        .unwrap_or_else(|| x.to_string())
                })
                .draw()?;
            
            // Draw the P&L line
            let data: Vec<(i64, f64)> = timestamps.into_iter()
                .zip(cumulative_pnl.iter().cloned())
                .collect();
                
            chart.draw_series(LineSeries::new(
                data.clone(),
                &BLUE,
            ))?
            .label("Cumulative P&L")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &BLUE));
            
            // Add a zero line
            chart.draw_series(LineSeries::new(
                vec![(min_time, 0.0), (max_time, 0.0)],
                &BLACK.mix(0.3),
            ))?;
            
            // Draw points for each trade
            chart.draw_series(PointSeries::of_element(
                data.clone(),
                3,
                &BLUE,
                &|c, s, st| {
                    return EmptyElement::at(c)
                        + Circle::new((0, 0), s, st.filled());
                },
            ))?;
            
            // Add final P&L annotation
            if let Some((last_time, last_pnl)) = data.last() {
                let color = if *last_pnl >= 0.0 { &GREEN } else { &RED };
                chart.draw_series(PointSeries::of_element(
                    vec![(*last_time, *last_pnl)],
                    5,
                    color,
                    &|c, s, st| {
                        return EmptyElement::at(c)
                            + Circle::new((0, 0), s, st.filled())
                            + Text::new(format!("${:.2}", last_pnl), (10, 0), ("sans-serif", 15).into_font());
                    },
                ))?;
            }
            
            chart.configure_series_labels()
                .background_style(&WHITE.mix(0.8))
                .border_style(&BLACK)
                .draw()?;
                
            root.present()?;
            println!("Generated P&L chart: {}", filename);
            
            // Store data for combined chart
            all_symbol_data.push((symbol.clone(), data));
        }
        
        // Generate combined chart with all symbols
        if !all_symbol_data.is_empty() {
            let combined_filename = format!("{}/{}combined.png", output_dir, prefix);
            let root = BitMapBackend::new(&combined_filename, (1200, 800)).into_drawing_area();
            root.fill(&WHITE)?;
            
            // Find global min/max values for all symbols
            let mut global_min_time = i64::MAX;
            let mut global_max_time = i64::MIN;
            let mut global_min_pnl = f64::INFINITY;
            let mut global_max_pnl = f64::NEG_INFINITY;
            
            for (_, data) in &all_symbol_data {
                for (time, pnl) in data {
                    global_min_time = global_min_time.min(*time);
                    global_max_time = global_max_time.max(*time);
                    global_min_pnl = global_min_pnl.min(*pnl);
                    global_max_pnl = global_max_pnl.max(*pnl);
                }
            }
            
            let pnl_range = if (global_max_pnl - global_min_pnl).abs() < 1.0 {
                global_min_pnl - 100.0..global_max_pnl + 100.0
            } else {
                global_min_pnl * 1.1..global_max_pnl * 1.1
            };
            
            let mut chart = ChartBuilder::on(&root)
                .caption("Combined P&L Chart - All Symbols", ("sans-serif", 45).into_font())
                .margin(15)
                .x_label_area_size(40)
                .y_label_area_size(70)
                .build_cartesian_2d(global_min_time..global_max_time, pnl_range)?;
            
            chart.configure_mesh()
                .x_desc("Time")
                .y_desc("P&L ($)")
                .x_label_formatter(&|x| {
                    chrono::DateTime::from_timestamp_millis(*x)
                        .map(|dt| dt.format("%H:%M").to_string())
                        .unwrap_or_else(|| x.to_string())
                })
                .draw()?;
            
            // Define colors for different symbols
            let colors = [&BLUE, &RED, &GREEN, &MAGENTA, &CYAN, &BLACK];
            
            // Draw zero line
            chart.draw_series(LineSeries::new(
                vec![(global_min_time, 0.0), (global_max_time, 0.0)],
                &BLACK.mix(0.2),
            ))?;
            
            // Draw each symbol's P&L line
            for (idx, (symbol, data)) in all_symbol_data.iter().enumerate() {
                let color = colors[idx % colors.len()];
                
                chart.draw_series(LineSeries::new(
                    data.clone(),
                    color,
                ))?
                .label(symbol)
                .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], color));
                
                // Add final value annotation
                if let Some((last_time, last_pnl)) = data.last() {
                    let label_color = if *last_pnl >= 0.0 { &GREEN } else { &RED };
                    chart.draw_series(PointSeries::of_element(
                        vec![(*last_time, *last_pnl)],
                        5,
                        color,
                        &|c, s, st| {
                            return EmptyElement::at(c)
                                + Circle::new((0, 0), s, st.filled())
                                + Text::new(
                                    format!("{}: ${:.2}", symbol, last_pnl), 
                                    (10, -5 - (idx as i32 * 15)), 
                                    ("sans-serif", 12).into_font().color(label_color)
                                );
                        },
                    ))?;
                }
            }
            
            // Draw legend
            chart.configure_series_labels()
                .background_style(&WHITE.mix(0.8))
                .border_style(&BLACK)
                .draw()?;
                
            root.present()?;
            println!("Generated combined P&L chart: {}", combined_filename);
        }
        
        Ok(())
    }
    
    /// Generate P&L graphs with default parameters (./data directory, pnl_ prefix)
    pub fn graph_default(&self, trades: &[Trade], method: Method) -> Result<(), Box<dyn std::error::Error>> {
        self.graph(trades, method, None, None)
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