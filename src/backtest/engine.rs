use std::path::Path;
use std::time::Instant;
use log::{info, warn};

use crate::core::{TradeState, Result, TradeError};
use crate::utils::{FileDataSource, extract_symbol_from_filename};
use crate::strategy::{Strategy, GptMarketMaker, GptMarketMakerConfig};
use crate::trading::{BacktestTradeEmitter, BacktestConfig, TradeEmitter};
use crate::core::DataSource;

pub struct BacktestEngine {
    config: BacktestConfig,
}

impl BacktestEngine {
    pub fn new(config: BacktestConfig) -> Self {
        Self { config }
    }
    
    pub fn run_backtest(
        &self,
        data_file: &Path,
        strategy_name: &str,
    ) -> Result<TradeState> {
        let start_time = Instant::now();
        
        let filename = data_file.file_name()
            .ok_or_else(|| TradeError::DataLoadingError("Invalid file path".to_string()))?
            .to_str()
            .ok_or_else(|| TradeError::DataLoadingError("Invalid filename encoding".to_string()))?;
        let symbol = extract_symbol_from_filename(filename);
        
        println!("Processing file: {:?}", data_file);
        println!("Extracted symbol: {}", symbol);
        println!("Using strategy: {}", strategy_name);
        
        // Initialize components
        let mut trade_state = TradeState::new();
        
        // Create strategy based on name
        let mut strategy: Box<dyn Strategy> = match strategy_name {
            "gpt" => {
                let gpt_config = GptMarketMakerConfig::default();
                Box::new(GptMarketMaker::new(symbol.clone(), gpt_config))
            }
            _ => return Err(TradeError::InvalidTradeParameters(
                format!("Unknown strategy: {}", strategy_name)
            )),
        };
        
        // Create executor
        let mut executor = BacktestTradeEmitter::new(self.config.clone());
        
        // Create data source
        let mut data_source = FileDataSource::new(data_file)?
            .with_batch_size(10000);
        
        // Count messages for progress tracking
        let total_messages = data_source.count_messages()?;
        if total_messages == 0 {
            warn!("No data found in {:?}, skipping", data_file);
            return Ok(trade_state);
        }
        
        info!("Running pre-backtest analysis...");
        info!("Running backtest for {} with {} orderbook messages", symbol, total_messages);
        
        let mut processed = 0;
        let mut last_progress = 0;
        
        // Process each orderbook
        while let Some(order_book) = data_source.next_orderbook()? {
            // Propose trade
            if let Some(pending_order) = strategy.propose_trade(&order_book) {
                trade_state.add(pending_order.clone());
                trade_state.add_orderbook(order_book.clone());
                
                // Execute trade
                if let Some(executed_trade) = executor.execute_trade(Some(pending_order)) {
                    trade_state.change_status(&executed_trade.id, executed_trade.status.clone());
                    
                    if executed_trade.status == "filled" {
                        strategy.update_position(&executed_trade, true);
                    } else {
                        strategy.update_position(&executed_trade, false);
                    }
                }
            }
            
            // Progress tracking
            processed += 1;
            let progress = (processed * 100) / total_messages;
            if progress > last_progress + 10 {
                info!("Progress: {}% ({}/{} messages)", progress, processed, total_messages);
                last_progress = progress;
            }
        }
        
        let execution_time = start_time.elapsed();
        println!("Backtest completed in {:.2} seconds", execution_time.as_secs_f64());
        info!("Backtest completed in {:.2} seconds", execution_time.as_secs_f64());
        
        Ok(trade_state)
    }
    
    pub fn run_backtest_with_custom_strategy(
        &self,
        data_file: &Path,
        mut strategy: Box<dyn Strategy>,
    ) -> Result<TradeState> {
        let start_time = Instant::now();
        
        let filename = data_file.file_name()
            .ok_or_else(|| TradeError::DataLoadingError("Invalid file path".to_string()))?
            .to_str()
            .ok_or_else(|| TradeError::DataLoadingError("Invalid filename encoding".to_string()))?;
        let symbol = extract_symbol_from_filename(filename);
        
        println!("Processing file: {:?}", data_file);
        println!("Extracted symbol: {}", symbol);
        println!("Using custom strategy: {}", strategy.name());
        
        // Initialize components
        let mut trade_state = TradeState::new();
        
        // Create executor
        let mut executor = BacktestTradeEmitter::new(self.config.clone());
        
        // Create data source
        let mut data_source = FileDataSource::new(data_file)?
            .with_batch_size(10000);
        
        // Count messages for progress tracking
        let total_messages = data_source.count_messages()?;
        if total_messages == 0 {
            warn!("No data found in {:?}, skipping", data_file);
            return Ok(trade_state);
        }
        
        info!("Running backtest for {} with {} orderbook messages", symbol, total_messages);
        
        let mut processed = 0;
        
        // Process each orderbook
        while let Some(order_book) = data_source.next_orderbook()? {
            // Propose trade
            if let Some(pending_order) = strategy.propose_trade(&order_book) {
                trade_state.add(pending_order.clone());
                trade_state.add_orderbook(order_book.clone());
                
                // Execute trade
                if let Some(executed_trade) = executor.execute_trade(Some(pending_order)) {
                    trade_state.change_status(&executed_trade.id, executed_trade.status.clone());
                    
                    if executed_trade.status == "filled" {
                        strategy.update_position(&executed_trade, true);
                    } else {
                        strategy.update_position(&executed_trade, false);
                    }
                }
            }
            
            processed += 1;
        }
        
        let execution_time = start_time.elapsed();
        info!("Backtest completed in {:.2} seconds ({} messages processed)", 
             execution_time.as_secs_f64(), processed);
        
        Ok(trade_state)
    }
}