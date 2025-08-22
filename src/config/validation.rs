use serde::{Deserialize, Serialize};
use crate::core::{Result, TradeError};
use crate::trading::BacktestConfig;
use crate::strategy::GptMarketMakerConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub backtest: BacktestConfig,
    pub strategy: StrategyConfig,
    pub data: DataConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub name: String,
    pub gpt_market_maker: Option<GptMarketMakerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    pub batch_size: usize,
    pub show_progress: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            backtest: BacktestConfig::default(),
            strategy: StrategyConfig {
                name: "gpt".to_string(),
                gpt_market_maker: Some(GptMarketMakerConfig::default()),
            },
            data: DataConfig {
                batch_size: 10000,
                show_progress: true,
            },
        }
    }
}

pub fn validate_config(config: &AppConfig) -> Result<()> {
    // Validate backtest config
    if config.backtest.fill_rate < 0.0 || config.backtest.fill_rate > 1.0 {
        return Err(TradeError::InvalidTradeParameters(
            format!("Fill rate must be between 0.0 and 1.0, got {}", config.backtest.fill_rate)
        ));
    }
    
    if config.backtest.rejection_rate < 0.0 || config.backtest.rejection_rate > 1.0 {
        return Err(TradeError::InvalidTradeParameters(
            format!("Rejection rate must be between 0.0 and 1.0, got {}", config.backtest.rejection_rate)
        ));
    }
    
    if config.backtest.margin_rate < 0.0 || config.backtest.margin_rate > 1.0 {
        return Err(TradeError::InvalidTradeParameters(
            format!("Margin rate must be between 0.0 and 1.0, got {}", config.backtest.margin_rate)
        ));
    }
    
    if config.backtest.slippage_bps < 0.0 {
        return Err(TradeError::InvalidTradeParameters(
            format!("Slippage must be non-negative, got {}", config.backtest.slippage_bps)
        ));
    }
    
    // Validate strategy config
    match config.strategy.name.as_str() {
        "gpt" => {
            if config.strategy.gpt_market_maker.is_none() {
                return Err(TradeError::InvalidTradeParameters(
                    "GPT Market Maker config is required when using 'gpt' strategy".to_string()
                ));
            }
        }
        _ => {
            return Err(TradeError::InvalidTradeParameters(
                format!("Unknown strategy: {}", config.strategy.name)
            ));
        }
    }
    
    // Validate data config
    if config.data.batch_size == 0 {
        return Err(TradeError::InvalidTradeParameters(
            "Batch size must be greater than 0".to_string()
        ));
    }
    
    Ok(())
}

