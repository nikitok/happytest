use clap::Parser;
use env_logger;
use std::path::Path;
use std::time::Instant;
use std::collections::HashMap;

use happytest::{
    BacktestConfig, BacktestEngine, TradeDashboard,
    data::extract_symbol_from_filename,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// JSONL file with orderbook data
    #[arg(short, long)]
    file: String,

    /// Order fill rate (0.0-1.0)
    #[arg(long, default_value_t = 0.98)]
    fill_rate: f64,

    /// Slippage in basis points
    #[arg(long, default_value_t = 2.0)]
    slippage_bps: f64,

    /// Order rejection rate (0.0-1.0)
    #[arg(long, default_value_t = 0.01)]
    rejection_rate: f64,

    /// Minimum spread percentage
    #[arg(long, default_value_t = 0.0005)]
    min_spread_pct: f64,

    /// Margin requirement rate (0.0-1.0)
    #[arg(long, default_value_t = 0.05)]
    margin_rate: f64,

    /// Fixed order volume for each trade
    #[arg(long, default_value_t = 0.005)]
    fix_order_volume: f64,

    /// Spread percentage for market making
    #[arg(long, default_value_t = 0.005)]
    spread_percent: f64,
}



fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let main_start = Instant::now();
    let args = Args::parse();
    
    // Create backtest config
    let backtest_config = BacktestConfig {
        fill_rate: args.fill_rate,
        slippage_bps: args.slippage_bps,
        rejection_rate: args.rejection_rate,
        margin_rate: args.margin_rate,
        fix_order_volume: args.fix_order_volume,
        min_spread_pct: args.min_spread_pct,
        spread_percent: args.spread_percent,
        max_order_volume: 0.0,
    };
    
    // Create backtest engine
    let engine = BacktestEngine::new(backtest_config.clone());
    
    // Run backtest
    let data_file = Path::new(&args.file);
    let trade_state = engine.run_backtest(data_file, "gpt")?;
    
    // Extract symbol for analysis
    let filename = data_file.file_name()
        .ok_or("Invalid file path")?
        .to_str()
        .ok_or("Invalid filename encoding")?;
    let symbol = extract_symbol_from_filename(filename);
    
    // Create dashboard for analysis
    let mut dashboard = TradeDashboard::new(
        trade_state,
        backtest_config.max_order_volume,
        backtest_config.margin_rate,
    );
    
    // Calculate PnL
    let pnl_results = dashboard.calculate_pnl(&symbol);
    
    // Print diagnostic info
    if let Some(result) = pnl_results.get(&symbol) {
        println!("\n=== DIAGNOSTIC INFO ===");
        println!("Total trades: {}", dashboard.trade_state.get_all_trades().len());
        println!("Filled trades: {}", dashboard.trade_state.get_trades_history().len());
        println!("Closed positions: {}", result.closed_trades.len());
        println!("======================\n");
    }
    
    // Get capital metrics
    let capital_metrics = dashboard.get_capital_metrics(&symbol);
    let mut capital_metrics_map = HashMap::new();
    capital_metrics_map.insert(symbol.clone(), capital_metrics);
    
    // Print metrics to console
    let _metrics_summary = dashboard.print_pnl_metrics(&symbol, &pnl_results);
    
    log::info!("============================================================");
    dashboard.to_console(&symbol, &pnl_results, &capital_metrics_map);
    
    let total_time = main_start.elapsed();
    println!("{}", "=".repeat(60));
    println!("Total execution time: {:.2} seconds", total_time.as_secs_f64());
    log::info!("Total execution time: {:.2} seconds", total_time.as_secs_f64());
    
    Ok(())
}