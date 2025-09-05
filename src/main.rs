use clap::{Parser, Subcommand};
use env_logger;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::fs;
use regex::Regex;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use rayon::prelude::*;

use happytest::{
    utils::extract_symbol_from_filename, BacktestConfig, BacktestEngine, TradeDashboard,
    pnl::{PnlReport, Method}, TradeState,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File path or regex pattern for orderbook data files (JSONL or Parquet)
    /// Examples: 
    ///   - Single file: data.jsonl or data.parquet
    ///   - Pattern: BTCUSDT_202509.*_mainnet.parquet
    #[arg(short, long)]
    file: String,
    
    /// Directory to search for files when using regex patterns
    #[arg(short = 'd', long, default_value = "./data")]
    directory: String,

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

    /// Treat regex-matched files as a single continuous range (for backtesting multiple periods)
    #[arg(long, default_value_t = true)]
    aggregate_files: bool,
    
    /// Process files in parallel (only when not aggregating)
    #[arg(long, default_value_t = false)]
    parallel: bool,
    
    /// Number of parallel workers (0 = number of CPU cores)
    #[arg(long, default_value_t = 0)]
    workers: usize,
    
    /// Strategy selection and configuration
    #[command(subcommand)]
    strategy: StrategyCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum StrategyCommand {
    /// GPT Market Maker strategy
    Gpt(happytest::strategy::GptMarketMakerArgs),
}

/// Find files matching a regex pattern in a directory
fn find_matching_files(directory: &Path, pattern: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let regex = Regex::new(pattern)?;
    let mut matching_files = Vec::new();
    
    // Read directory entries
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        
        // Skip directories
        if path.is_dir() {
            continue;
        }
        
        // Get filename and check if it matches the pattern
        if let Some(filename) = path.file_name() {
            if let Some(filename_str) = filename.to_str() {
                if regex.is_match(filename_str) {
                    matching_files.push(path);
                }
            }
        }
    }
    
    // Sort files for consistent processing order
    matching_files.sort();
    
    Ok(matching_files)
}

/// Process a single file with the backtest engine
fn process_single_file(
    file_path: &Path,
    args: &Args,
    backtest_config: &BacktestConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(60));
    println!("Processing file: {:?}", file_path);
    println!("{}", "=".repeat(60));
    
    // Show initialization spinner
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈", "·"])
    );
    spinner.set_message("⚙️  Initializing strategy and loading data...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    
    // Extract symbol for strategy creation
    let filename = file_path
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
    
    spinner.finish_with_message("✅ Strategy initialized");

    // Run backtest with the constructed strategy
    let trade_state = engine.run_backtest_with_custom_strategy(file_path, strategy)?;

    // Create dashboard for analysis
    let mut dashboard = TradeDashboard::new(
        trade_state,
        backtest_config.margin_rate,
    );

    // Calculate PnL
    let pnl_results = dashboard.pnl(&symbol);

    // Print diagnostic info
    println!("\n=== DIAGNOSTIC INFO ===");
    println!(
        "Total trades: {}",
        dashboard.trade_state.get_all_trades().len()
    );
    println!(
        "Filled trades: {}",
        dashboard.trade_state.get_trades_history().len()
    );
    if let Some(result) = pnl_results.get(&symbol) {
        println!("Closed positions: {}", result.closed_trades.len());
    }
    
    // Get capital metrics for Max DD
    let capital_metrics_temp = dashboard.get_capital_metrics(&symbol);
    println!("Max Drawdown: ${:.2}", capital_metrics_temp.max_drawdown);
    
    println!("======================");

    // Use PnlReport to display results in a nice table
    let pnl_report = PnlReport::new();
    let all_trades = dashboard.trade_state.get_all_trades();
    let report = pnl_report.report(all_trades, Method::Fifo);
    println!("{}", report);
    
    // Generate P&L graphs (PNG files)
    let output_name = format!("{}_{}", 
        file_path.file_stem().unwrap_or_default().to_str().unwrap_or("output"),
        "graph"
    );
    pnl_report.graph_by_minute(all_trades, Method::Fifo, None, Some(&output_name))?;
    
    // Display P&L graph in console
    pnl_report.display_console_graph(all_trades, Method::Fifo)?;

    // Get capital metrics
    let capital_metrics = dashboard.get_capital_metrics(&symbol);
    let mut capital_metrics_map = HashMap::new();
    capital_metrics_map.insert(symbol.clone(), capital_metrics);

    // Print metrics to console
    let _metrics_summary = dashboard.print_pnl_metrics(&symbol, &pnl_results);

    log::info!("============================================================");
    dashboard.to_console(&symbol, &pnl_results, &capital_metrics_map);
    
    Ok(())
}

/// Process multiple files as a continuous range
fn process_files_as_range(
    file_paths: &[PathBuf],
    args: &Args,
    backtest_config: &BacktestConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(60));
    println!("Processing {} files as a continuous range", file_paths.len());
    println!("{}", "=".repeat(60));
    
    // Show initialization spinner
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈", "·"])
    );
    spinner.set_message(format!("⚙️  Initializing strategy and loading {} files...", file_paths.len()));
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    
    // Extract symbols from all files to show what we're processing
    let mut file_symbols = Vec::new();
    println!("Files to process:");
    for (i, path) in file_paths.iter().enumerate() {
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            let sym = extract_symbol_from_filename(filename);
            file_symbols.push(sym.clone());
            println!("  {}. {} (Symbol: {})", i + 1, path.display(), sym);
        } else {
            println!("  {}. {}", i + 1, path.display());
        }
    }
    
    // Use "MULTI" as symbol for multi-symbol processing
    let symbol = "MULTI".to_string();

    // Create strategy from command line arguments
    let strategy = match &args.strategy {
        StrategyCommand::Gpt(gpt_args) => {
            gpt_args.build_strategy(symbol.clone())
        }
    };

    // Create backtest engine
    let engine = BacktestEngine::new(backtest_config.clone());
    
    spinner.finish_with_message(format!("✅ Strategy initialized, {} files ready", file_paths.len()));

    // Run backtest with multiple files as a single continuous data source
    let trade_state = engine.run_backtest_with_multiple_files(file_paths, strategy)?;

    // Create dashboard for analysis
    let mut dashboard = TradeDashboard::new(
        trade_state,
        backtest_config.margin_rate,
    );

    // Get all unique symbols from trades
    let all_trades = dashboard.trade_state.get_all_trades();
    let mut unique_symbols = HashSet::new();
    for trade in all_trades {
        unique_symbols.insert(trade.symbol.clone());
    }
    
    // Calculate PnL for each symbol
    let mut all_pnl_results = HashMap::new();
    for sym in &unique_symbols {
        let pnl_result = dashboard.pnl(sym);
        all_pnl_results.extend(pnl_result);
    }

    // Print diagnostic info
    println!("\n=== DIAGNOSTIC INFO ===");
    let all_trades = dashboard.trade_state.get_all_trades();
    println!(
        "Total trades: {}",
        all_trades.len()
    );
    println!(
        "Filled trades: {}",
        dashboard.trade_state.get_trades_history().len()
    );
    
    // Print closed positions for each symbol
    for sym in &unique_symbols {
        if let Some(result) = all_pnl_results.get(sym) {
            println!("Closed positions for {}: {}", sym, result.closed_trades.len());
        }
    }
    
    // Get capital metrics for all symbols
    let mut max_drawdown_total = 0.0;
    for sym in &unique_symbols {
        let capital_metrics_temp = dashboard.get_capital_metrics(sym);
        println!("Max Drawdown for {}: ${:.2}", sym, capital_metrics_temp.max_drawdown);
        max_drawdown_total += capital_metrics_temp.max_drawdown;
    }
    if !unique_symbols.is_empty() {
        println!("Total Max Drawdown: ${:.2}", max_drawdown_total);
    }
    
    println!("======================");

    // Use PnlReport to display results in a nice table
    let pnl_report = PnlReport::new();
    let all_trades = dashboard.trade_state.get_all_trades();
    let report = pnl_report.report(all_trades, Method::Fifo);
    println!("{}", report);
    
    // Generate P&L graphs (PNG files)
    // let output_name = format!("aggregated_{}_{}",
    //     "MULTI",
    //     "graph"
    // );
    // pnl_report.graph_by_minute(all_trades, Method::Fifo, None, Some(&output_name))?;
    //
    // Display P&L graph in console
    pnl_report.display_console_graph(all_trades, Method::Fifo)?;

    // Get capital metrics for all symbols
    let mut capital_metrics_map = HashMap::new();
    for sym in &unique_symbols {
        let capital_metrics = dashboard.get_capital_metrics(sym);
        capital_metrics_map.insert(sym.clone(), capital_metrics);
    }

    // Print metrics to console for each symbol
    for sym in &unique_symbols {
        // Filter PnL results for this specific symbol
        if let Some(result) = all_pnl_results.get(sym) {
            let mut temp_map = HashMap::new();
            temp_map.insert(sym.clone(), result.clone());
            let _metrics_summary = dashboard.print_pnl_metrics(sym, &temp_map);
        }
    }

    log::info!("============================================================");
    // Print console output for each symbol
    for sym in &unique_symbols {
        if let Some(result) = all_pnl_results.get(sym) {
            let mut sym_pnl_results = HashMap::new();
            sym_pnl_results.insert(sym.clone(), result.clone());
            dashboard.to_console(sym, &sym_pnl_results, &capital_metrics_map);
        }
    }
    
    Ok(())
}

/// Process multiple files in parallel and aggregate results
fn process_files_parallel(
    file_paths: &[PathBuf],
    args: &Args,
    backtest_config: &BacktestConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(60));
    println!("Processing {} files in parallel", file_paths.len());
    println!("{}", "=".repeat(60));
    
    // Set up thread pool
    if args.workers > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.workers)
            .build_global()
            .unwrap();
    }
    
    // Extract symbol from first file
    let first_file = &file_paths[0];
    let filename = first_file
        .file_name()
        .ok_or("Invalid file path")?
        .to_str()
        .ok_or("Invalid filename encoding")?;
    let symbol = extract_symbol_from_filename(filename);
    
    println!("Symbol: {}", symbol);
    println!("Files to process in parallel:");
    for (i, path) in file_paths.iter().enumerate() {
        println!("  {}. {}", i + 1, path.display());
    }
    println!("Workers: {}", args.workers.max(rayon::current_num_threads()));
    
    // Create multi-progress for parallel tracking
    let multi_progress = MultiProgress::new();
    let overall_pb = multi_progress.add(ProgressBar::new(file_paths.len() as u64));
    overall_pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} Overall [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} files")
            .unwrap()
            .progress_chars("██░")
    );
    overall_pb.set_message("Processing files in parallel");
    
    // Process files in parallel
    let results: Vec<_> = file_paths
        .par_iter()
        .map(|file_path| {
            // Create strategy for this file
            let strategy = match &args.strategy {
                StrategyCommand::Gpt(gpt_args) => {
                    gpt_args.build_strategy(symbol.clone())
                }
            };
            
            // Create backtest engine
            let engine = BacktestEngine::new(backtest_config.clone());
            
            // Run backtest
            let result = engine.run_backtest_with_custom_strategy(file_path, strategy);
            
            overall_pb.inc(1);
            
            result
        })
        .collect();
    
    overall_pb.finish_with_message("✅ All files processed");
    
    // Check for errors
    let mut all_trade_states = Vec::new();
    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(trade_state) => {
                println!("File {} processed successfully", i + 1);
                all_trade_states.push(trade_state);
            }
            Err(e) => {
                eprintln!("Error processing file {}: {}", i + 1, e);
                return Err(e.into());
            }
        }
    }
    
    // Aggregate results
    println!("\n{}", "=".repeat(60));
    println!("Aggregating results from {} files...", all_trade_states.len());
    println!("{}", "=".repeat(60));
    
    // Merge all trade states
    let mut merged_trade_state = TradeState::new();
    for trade_state in all_trade_states {
        // Merge trades
        for trade in trade_state.get_all_trades() {
            merged_trade_state.add(trade.clone());
        }
        // Merge orderbooks
        for orderbook in trade_state.get_orderbooks() {
            merged_trade_state.add_orderbook(orderbook.clone());
        }
    }
    
    // Create dashboard for analysis
    let mut dashboard = TradeDashboard::new(
        merged_trade_state,
        backtest_config.margin_rate,
    );
    
    // Calculate PnL
    let pnl_results = dashboard.pnl(&symbol);
    
    // Print diagnostic info
    println!("\n=== AGGREGATED DIAGNOSTIC INFO ===");
    println!(
        "Total trades: {}",
        dashboard.trade_state.get_all_trades().len()
    );
    println!(
        "Filled trades: {}",
        dashboard.trade_state.get_trades_history().len()
    );
    if let Some(result) = pnl_results.get(&symbol) {
        println!("Closed positions: {}", result.closed_trades.len());
    }
    
    // Get capital metrics for Max DD
    let capital_metrics_temp = dashboard.get_capital_metrics(&symbol);
    println!("Max Drawdown: ${:.2}", capital_metrics_temp.max_drawdown);
    
    println!("Files processed: {}", file_paths.len());
    println!("===================================");
    
    // Use PnlReport to display results in a nice table
    let pnl_report = PnlReport::new();
    let all_trades = dashboard.trade_state.get_all_trades();
    let report = pnl_report.report(all_trades, Method::Fifo);
    println!("{}", report);
    
    // Display P&L graph in console
    pnl_report.display_console_graph(all_trades, Method::Fifo)?;
    
    // Get capital metrics
    let capital_metrics = dashboard.get_capital_metrics(&symbol);
    let mut capital_metrics_map = HashMap::new();
    capital_metrics_map.insert(symbol.clone(), capital_metrics);
    
    // Print metrics to console
    let _metrics_summary = dashboard.print_pnl_metrics(&symbol, &pnl_results);
    
    log::info!("============================================================");
    dashboard.to_console(&symbol, &pnl_results, &capital_metrics_map);
    
    Ok(())
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

    // Determine if the input is a file path or a regex pattern
    let file_path = Path::new(&args.file);
    
    let files_to_process = if file_path.exists() && file_path.is_file() {
        // Single file mode
        vec![file_path.to_path_buf()]
    } else {
        // Pattern mode - search for matching files
        let search_dir = Path::new(&args.directory);
        
        if !search_dir.exists() || !search_dir.is_dir() {
            return Err(format!("Directory '{}' does not exist or is not a directory", args.directory).into());
        }
        
        println!("Searching for files matching pattern '{}' in directory '{}'", args.file, args.directory);
        
        let matching_files = find_matching_files(search_dir, &args.file)?;
        
        if matching_files.is_empty() {
            return Err(format!("No files found matching pattern '{}'", args.file).into());
        }
        
        println!("\nFound {} matching files:", matching_files.len());
        for (i, file) in matching_files.iter().enumerate() {
            println!("  {}. {}", i + 1, file.display());
        }
        
        matching_files
    };

    // Process files based on aggregate_files and parallel flags
    if files_to_process.len() > 1 {
        if args.parallel {
            // Process files in parallel and aggregate results (overrides aggregate_files)
            if let Err(e) = process_files_parallel(&files_to_process, &args, &backtest_config) {
                eprintln!("Error processing files in parallel: {}", e);
            }
        } else if args.aggregate_files {
            // Process all files as a single continuous range (sequential)
            if let Err(e) = process_files_as_range(&files_to_process, &args, &backtest_config) {
                eprintln!("Error processing files as range: {}", e);
            }
        } else {
            // Process each file individually (sequential)
            for file_path in &files_to_process {
                if let Err(e) = process_single_file(file_path, &args, &backtest_config) {
                    eprintln!("Error processing file {:?}: {}", file_path, e);
                    // Continue with next file instead of failing completely
                }
            }
        }
    } else {
        // Single file - process normally
        for file_path in &files_to_process {
            if let Err(e) = process_single_file(file_path, &args, &backtest_config) {
                eprintln!("Error processing file {:?}: {}", file_path, e);
            }
        }
    }

    let total_time = main_start.elapsed();
    println!("\n{}", "=".repeat(60));
    println!(
        "Total execution time for {} file(s): {:.2} seconds",
        files_to_process.len(),
        total_time.as_secs_f64()
    );
    log::info!(
        "Total execution time: {:.2} seconds",
        total_time.as_secs_f64()
    );

    Ok(())
}