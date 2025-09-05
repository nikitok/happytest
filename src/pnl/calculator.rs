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
    commission_rate: f64,  // Commission rate as a percentage (e.g., 0.03 for 0.03%)
}

impl PnlReport {
    pub fn new() -> Self {
        Self::with_commission(0.03)  // Default commission of 0.03%
    }
    
    pub fn with_commission(commission_rate: f64) -> Self {
        Self {
            fifo_processor: FifoProcessor::new(),
            position_processor: PositionProcessor::new(),
            commission_rate,
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
            "Gross P&L",
            "Commission",
            "Net P&L",
            "Max Drawdown %",
            "Sharpe Ratio",
        ]);
        
        // Sort symbols for consistent output
        let mut symbols: Vec<String> = trades_by_symbol.keys().cloned().collect();
        symbols.sort();
        
        let mut total_trades = 0;
        let mut total_gross_pnl = 0.0;
        let mut total_commission = 0.0;
        let mut total_net_pnl = 0.0;
        let mut max_drawdown_sum = 0.0;
        let mut sharpe_sum = 0.0;
        let mut symbol_count = 0;
        
        // Process each symbol
        for symbol in symbols {
            if let Some(symbol_trades) = trades_by_symbol.get(&symbol) {
                let result = self.calculate(symbol_trades, method);
                let gross_pnl = result.total_pnl + result.unrealized_pnl;
                
                // Calculate commission
                let total_volume = symbol_trades.iter()
                    .filter(|t| t.status.to_lowercase() == "filled")
                    .map(|t| t.quantity * t.price)
                    .sum::<f64>();
                let commission = total_volume * (self.commission_rate / 100.0);
                let net_pnl = gross_pnl - commission;
                
                // Calculate metrics
                let (max_drawdown, sharpe_ratio) = self.calculate_metrics(symbol_trades, &result);
                
                table.add_row(vec![
                    symbol.clone(),
                    symbol_trades.len().to_string(),
                    format!("${:.2}", gross_pnl),
                    format!("${:.2}", commission),
                    format!("${:.2}", net_pnl),
                    format!("{:.2}%", max_drawdown),
                    format!("{:.2}", sharpe_ratio),
                ]);
                
                total_trades += symbol_trades.len();
                total_gross_pnl += gross_pnl;
                total_commission += commission;
                total_net_pnl += net_pnl;
                
                if !max_drawdown.is_nan() {
                    max_drawdown_sum += max_drawdown;
                    sharpe_sum += sharpe_ratio;
                    symbol_count += 1;
                }
            }
        }
        
        // Calculate averages for metrics
        let avg_drawdown = if symbol_count > 0 { max_drawdown_sum / symbol_count as f64 } else { 0.0 };
        let avg_sharpe = if symbol_count > 0 { sharpe_sum / symbol_count as f64 } else { 0.0 };
        
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
            format!("${:.2}", total_gross_pnl),
            format!("${:.2}", total_commission),
            format!("${:.2}", total_net_pnl),
            format!("{:.2}%", avg_drawdown),
            format!("{:.2}", avg_sharpe),
        ]);
        
        format!("\n=== P&L Summary by Symbol ===\n{}", table)
    }
    
    /// Generate P&L graphs for each symbol (without aggregation)
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
    
    /// Generate P&L graphs with time-based aggregation
    /// 
    /// # Arguments
    /// * `trades` - List of trades to analyze
    /// * `method` - Calculation method (Fifo or Position)
    /// * `output_dir` - Output directory for charts (default: "./data")
    /// * `prefix` - Filename prefix (default: "pnl_")
    /// * `aggregation_ms` - Aggregation interval in milliseconds (e.g., 1000 for 1 second, 60000 for 1 minute)
    pub fn graph_with_aggregation(
        &self, 
        trades: &[Trade], 
        method: Method,
        output_dir: Option<&str>,
        prefix: Option<&str>,
        aggregation_ms: i64,
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
            
            // Aggregate trades by time buckets
            let mut aggregated_data: Vec<(i64, Vec<Trade>)> = Vec::new();
            
            for trade in symbol_trades.iter() {
                let bucket_time = (trade.time / aggregation_ms) * aggregation_ms;
                
                if let Some(last) = aggregated_data.last_mut() {
                    if last.0 == bucket_time {
                        last.1.push(trade.clone());
                    } else {
                        aggregated_data.push((bucket_time, vec![trade.clone()]));
                    }
                } else {
                    aggregated_data.push((bucket_time, vec![trade.clone()]));
                }
            }
            
            // Calculate cumulative P&L for each time bucket
            let mut cumulative_pnl = Vec::new();
            let mut timestamps = Vec::new();
            let mut all_trades_so_far = Vec::new();
            
            for (bucket_time, bucket_trades) in aggregated_data {
                // Add trades from this bucket to all trades
                all_trades_so_far.extend(bucket_trades);
                
                // Calculate P&L up to this point
                let result = self.calculate(&all_trades_so_far, method);
                let current_pnl = result.total_pnl + result.unrealized_pnl;
                
                cumulative_pnl.push(current_pnl);
                timestamps.push(bucket_time);
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
                .caption(&format!("P&L Chart for {} ({}ms aggregation)", symbol, aggregation_ms), ("sans-serif", 40).into_font())
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
                .caption(&format!("Combined P&L Chart - All Symbols ({}ms aggregation)", aggregation_ms), ("sans-serif", 45).into_font())
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
    
    /// Generate P&L graphs with second-level aggregation
    pub fn graph_by_second(
        &self, 
        trades: &[Trade], 
        method: Method,
        output_dir: Option<&str>,
        prefix: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_with_aggregation(trades, method, output_dir, prefix, 1000)
    }
    
    /// Generate P&L graphs with minute-level aggregation
    pub fn graph_by_minute(
        &self, 
        trades: &[Trade], 
        method: Method,
        output_dir: Option<&str>,
        prefix: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_with_aggregation(trades, method, output_dir, prefix, 60000)
    }
    
    /// Generate P&L graphs with default parameters (./data directory, pnl_ prefix)
    pub fn graph_default(&self, trades: &[Trade], method: Method) -> Result<(), Box<dyn std::error::Error>> {
        self.graph(trades, method, None, None)
    }
    
    /// Format number in compact form (e.g., 1234 -> 1.2k)
    fn format_value_compact(value: f64) -> String {
        let abs_value = value.abs();
        let formatted = if abs_value >= 1_000_000.0 {
            format!("{:.1}M", value / 1_000_000.0)
        } else if abs_value >= 1_000.0 {
            format!("{:.1}k", value / 1_000.0)
        } else {
            format!("{:.0}", value)
        };
        formatted
    }
    
    /// Display P&L graph in console using ASCII/Unicode characters
    pub fn display_console_graph(&self, trades: &[Trade], method: Method) -> Result<(), Box<dyn std::error::Error>> {
        // Group trades by symbol
        let mut trades_by_symbol: HashMap<String, Vec<Trade>> = HashMap::new();
        
        for trade in trades {
            trades_by_symbol
                .entry(trade.symbol.clone())
                .or_default()
                .push(trade.clone());
        }
        
        // Process each symbol
        for (symbol, symbol_trades) in trades_by_symbol.iter() {
            let result = self.calculate(symbol_trades, method);
            
            // Get filled trades sorted by time
            let mut filled_trades: Vec<&Trade> = symbol_trades.iter()
                .filter(|t| t.status.to_lowercase() == "filled")
                .collect();
            filled_trades.sort_by_key(|t| t.time);
            
            if filled_trades.is_empty() {
                continue;
            }
            
            // Calculate cumulative P&L over time
            let mut cumulative_pnl = Vec::new();
            let mut running_pnl = 0.0;
            
            // Build cumulative P&L based on closed trades
            for closed_trade in &result.closed_trades {
                running_pnl += closed_trade.pnl;
                cumulative_pnl.push(running_pnl);
            }
            
            if cumulative_pnl.is_empty() {
                println!("No P&L data to display for {}", symbol);
                continue;
            }
            
            // Create data points for the chart (index, pnl)
            // Aggregate data if there are too many points for console display
            let max_console_points = 100;
            let data_points: Vec<(f32, f32)> = if cumulative_pnl.len() > max_console_points {
                // Sample every nth point to reduce data
                let step = cumulative_pnl.len() / max_console_points;
                cumulative_pnl
                    .iter()
                    .enumerate()
                    .step_by(step.max(1))
                    .map(|(i, &pnl)| (i as f32, pnl as f32))
                    .collect()
            } else {
                cumulative_pnl
                    .iter()
                    .enumerate()
                    .map(|(i, &pnl)| (i as f32, pnl as f32))
                    .collect()
            };
            
            // Calculate commission for net P&L
            let total_volume = symbol_trades.iter()
                .filter(|t| t.status.to_lowercase() == "filled")
                .map(|t| t.quantity * t.price)
                .sum::<f64>();
            let commission = total_volume * (self.commission_rate / 100.0);
            let net_pnl = running_pnl - commission;
            
            // Calculate Max Drawdown
            let (max_dd_pct, max_dd_value) = self.calculate_max_drawdown(&cumulative_pnl);
            
            // Get min and max P&L for proper Y-axis range
            let min_pnl = cumulative_pnl.iter().fold(f64::INFINITY, |a, &b| a.min(b)) as f32;
            let max_pnl = cumulative_pnl.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)) as f32;
            let pnl_range = if (max_pnl - min_pnl).abs() < 1.0 {
                100.0
            } else {
                max_pnl - min_pnl
            };
            let y_min = min_pnl - pnl_range * 0.1;
            let y_max = max_pnl + pnl_range * 0.1;
            
            // Create the console chart
            println!("\n{}", "=".repeat(80));
            println!("P&L Chart for {} (Console View)", symbol);
            println!("{}", "=".repeat(80));
            
            // Calculate time duration from filled trades
            let time_duration_minutes = if !filled_trades.is_empty() {
                let first_time = filled_trades.first().map(|t| t.time).unwrap_or(0);
                let last_time = filled_trades.last().map(|t| t.time).unwrap_or(0);
                (last_time - first_time) / 60_000 // Convert ms to minutes
            } else {
                0
            };
            
            // Display a simple ASCII chart
            if !data_points.is_empty() && data_points.len() > 1 {
                println!("\nP&L Progression ({} closed trades, {} min):", 
                    cumulative_pnl.len(), time_duration_minutes);
                
                // Create a simple bar chart using ASCII
                let chart_height = 15;
                let chart_width = 60;
                
                // Sample data points for display
                let samples = chart_width.min(data_points.len());
                let step = data_points.len() / samples;
                
                // Find min/max for scaling
                let min_val = data_points.iter().map(|(_, y)| *y).fold(f32::INFINITY, f32::min);
                let max_val = data_points.iter().map(|(_, y)| *y).fold(f32::NEG_INFINITY, f32::max);
                let range = max_val - min_val;
                
                // Print the chart with formatted Y-axis
                let max_label = Self::format_value_compact(max_val as f64);
                let min_label = Self::format_value_compact(min_val as f64);
                
                // Calculate the maximum label width for proper alignment
                let max_label_width = max_label.len().max(min_label.len()).max(6);
                
                // Print top Y-axis label
                println!("\n{:>width$} ┤", format!("${}", max_label), width = max_label_width + 2);
                
                for row in 0..chart_height {
                    print!("{:width$} │", "", width = max_label_width + 2);
                    let threshold = max_val - (row as f32 * range / chart_height as f32);
                    
                    for col in 0..samples {
                        let idx = col * step;
                        if idx < data_points.len() {
                            let val = data_points[idx].1;
                            if val >= threshold - (range / chart_height as f32 / 2.0) {
                                print!("█");
                            } else {
                                print!(" ");
                            }
                        }
                    }
                    println!();
                }
                
                // Print bottom Y-axis label
                println!("{:>width$} └{}", format!("${}", min_label), "─".repeat(chart_width), width = max_label_width + 2);
                
                // X-axis labels with trades and time
                let spaces = chart_width.saturating_sub(cumulative_pnl.len().to_string().len()) - 1;
                
                println!("{:width$} 0{}{}", "", " ".repeat(spaces), cumulative_pnl.len(), width = max_label_width + 2);
                println!("{:width$} │{}│", "", " ".repeat(chart_width - 2), width = max_label_width + 2);
                let time_spaces = spaces.saturating_sub(time_duration_minutes.to_string().len() + 6);
                println!("{:width$}(0 min){}({} min)", "", " ".repeat(time_spaces), time_duration_minutes, width = max_label_width - 1);
                println!("\n                     Trades / Time");
            } else {
                println!("[Insufficient data points for chart]");
            }
            
            println!("\n{}", "-".repeat(80));
            println!("Summary:");
            println!("  Total Trades: {}", filled_trades.len());
            println!("  Gross P&L: ${:.2}", running_pnl);
            println!("  Commission ({}%): ${:.2}", self.commission_rate, commission);
            println!("  Net P&L: ${:.2}", net_pnl);
            println!("  Max Drawdown: ${:.2} ({:.2}%)", max_dd_value, max_dd_pct);
            println!("{}", "=".repeat(80));
        }
        
        Ok(())
    }
    
    /// Calculate metrics including Max Drawdown and Sharpe Ratio
    fn calculate_metrics(&self, trades: &[Trade], result: &PnLResult) -> (f64, f64) {
        // Get filled trades sorted by timestamp
        let mut filled_trades: Vec<&Trade> = trades.iter()
            .filter(|t| t.status.to_lowercase() == "filled")
            .collect();
        filled_trades.sort_by_key(|t| t.time);
        
        if filled_trades.is_empty() {
            return (0.0, 0.0);
        }
        
        // Calculate cumulative P&L over time
        let mut cumulative_pnl = vec![0.0];
        let mut running_pnl = 0.0;
        
        for closed_trade in result.closed_trades.iter() {
            running_pnl += closed_trade.pnl;
            cumulative_pnl.push(running_pnl);
        }
        
        // Calculate Max Drawdown
        let mut max_drawdown_pct: f64 = 0.0;
        if !cumulative_pnl.is_empty() {
            let mut peak = cumulative_pnl[0];
            for &pnl in &cumulative_pnl {
                if pnl > peak {
                    peak = pnl;
                }
                if peak > 0.0 {
                    let drawdown = ((peak - pnl) / peak) * 100.0;
                    max_drawdown_pct = max_drawdown_pct.max(drawdown);
                }
            }
        }
        
        // Calculate Sharpe Ratio (annualized)
        let sharpe_ratio = if cumulative_pnl.len() > 2 {
            // Calculate returns between periods
            let mut returns = Vec::new();
            for i in 1..cumulative_pnl.len() {
                if cumulative_pnl[i-1] != 0.0 {
                    returns.push((cumulative_pnl[i] - cumulative_pnl[i-1]) / cumulative_pnl[i-1].abs());
                } else if cumulative_pnl[i] != 0.0 {
                    // Handle case where previous value is 0
                    returns.push(if cumulative_pnl[i] > 0.0 { 1.0 } else { -1.0 });
                }
            }
            
            if !returns.is_empty() {
                // Calculate mean return
                let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
                
                // Calculate standard deviation
                let variance = returns.iter()
                    .map(|r| (r - mean_return).powi(2))
                    .sum::<f64>() / returns.len() as f64;
                let std_dev = variance.sqrt();
                
                // Calculate annualized Sharpe ratio
                // Assuming daily returns and 252 trading days per year
                if std_dev > 0.0 {
                    let annualized_return = mean_return * (252.0_f64).sqrt();
                    let annualized_std = std_dev * (252.0_f64).sqrt();
                    annualized_return / annualized_std
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        (max_drawdown_pct, sharpe_ratio)
    }
    
    /// Calculate maximum drawdown from cumulative P&L series
    fn calculate_max_drawdown(&self, cumulative_pnl: &[f64]) -> (f64, f64) {
        if cumulative_pnl.is_empty() {
            return (0.0, 0.0);
        }
        
        let mut max_drawdown_pct = 0.0;
        let mut max_drawdown_value = 0.0;
        let mut peak = cumulative_pnl[0];
        
        for &pnl in cumulative_pnl {
            if pnl > peak {
                peak = pnl;
            }
            let drawdown_value = peak - pnl;
            if drawdown_value > max_drawdown_value {
                max_drawdown_value = drawdown_value;
                if peak > 0.0 {
                    max_drawdown_pct = (drawdown_value / peak) * 100.0;
                }
            }
        }
        
        (max_drawdown_pct, max_drawdown_value)
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