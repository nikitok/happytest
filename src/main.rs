use clap::{Parser, Subcommand};
use env_logger;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use happytest::{
    data::extract_symbol_from_filename, BacktestConfig, BacktestEngine, TradeDashboard,
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

    /// Margin requirement rate (0.0-1.0)
    #[arg(long, default_value_t = 0.05)]
    margin_rate: f64,

    /// Strategy selection and configuration
    #[command(subcommand)]
    strategy: StrategyCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum StrategyCommand {
    /// GPT Market Maker strategy
    Gpt(happytest::strategy::GptMarketMakerArgs),
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let main_start = Instant::now();
    let args = Args::parse();

    // Create backtest config (only backtest-specific parameters)
    let backtest_config = BacktestConfig {
        fill_rate: args.fill_rate,
        slippage_bps: args.slippage_bps,
        rejection_rate: args.rejection_rate,
        margin_rate: args.margin_rate,
        min_spread_pct: 0.0005, // Default value, could be made a CLI arg if needed
        spread_percent: 0.005, // Default value, could be made a CLI arg if needed
        max_order_volume: 0.0,
    };

    // Extract symbol for strategy creation
    let data_file = Path::new(&args.file);
    let filename = data_file
        .file_name()
        .ok_or("Invalid file path")?
        .to_str()
        .ok_or("Invalid filename encoding")?;
    let symbol = extract_symbol_from_filename(filename);

    // Create strategy from command line arguments
    let strategy = match &args.strategy {
        StrategyCommand::Gpt(gpt_args) => {
            gpt_args.build_strategy(symbol.clone())
        }
    };

    // Create backtest engine
    let engine = BacktestEngine::new(backtest_config.clone());

    // Run backtest with the constructed strategy
    let trade_state = engine.run_backtest_with_custom_strategy(data_file, strategy)?;

    // Symbol already extracted above

    // Create dashboard for analysis
    let mut dashboard = TradeDashboard::new(
        trade_state,
        backtest_config.margin_rate,
    );

    // Calculate PnL
    let pnl_results = dashboard.pnl(&symbol);

    // Print diagnostic info and PNL results
    if let Some(result) = pnl_results.get(&symbol) {
        println!("\n=== DIAGNOSTIC INFO ===");
        println!(
            "Total trades: {}",
            dashboard.trade_state.get_all_trades().len()
        );
        println!(
            "Filled trades: {}",
            dashboard.trade_state.get_trades_history().len()
        );
        println!("Closed positions: {}", result.closed_trades.len());
        println!("======================\n");

        println!("=== PNL RESULTS ===");
        println!("Total PNL: ${:.2}", result.total_pnl);
        println!("Unrealized PNL: ${:.2}", result.unrealized_pnl);
        println!("Total Fees: ${:.2}", result.total_fees);
        println!("==================\n");
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
    println!(
        "Total execution time: {:.2} seconds",
        total_time.as_secs_f64()
    );
    log::info!(
        "Total execution time: {:.2} seconds",
        total_time.as_secs_f64()
    );

    Ok(())
}
