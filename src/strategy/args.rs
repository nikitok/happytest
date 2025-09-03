use clap::Args;
use crate::strategy::Strategy;

/// Trait for strategy-specific command line arguments
pub trait StrategyArgs: Args {
    /// Create a strategy instance from the parsed arguments
    fn build_strategy(&self, symbol: String) -> Box<dyn Strategy>;
}

/// Command line arguments for GPT Market Maker strategy
#[derive(Debug, Clone, Args)]
pub struct GptMarketMakerArgs {
    /// Fixed order volume for each trade
    #[arg(long, default_value_t = 0.005)]
    pub fix_order_volume: f64,

    /// VWAP window size
    #[arg(long, default_value_t = 100)]
    pub vwap_window: usize,

    /// Order Book Imbalance threshold
    #[arg(long, default_value_t = 0.1)]
    pub obi_threshold: f64,

    /// Maximum inventory allowed
    #[arg(long, default_value_t = 10.0)]
    pub max_inventory: f64,

    /// Use limit orders instead of market orders
    #[arg(long, default_value_t = true)]
    pub use_limit_orders: bool,

    /// Limit order spread in basis points
    #[arg(long, default_value_t = 5.0)]
    pub limit_order_spread_bps: f64,

    /// Take profit threshold in basis points
    #[arg(long, default_value_t = 20.0)]
    pub take_profit_bps: f64,

    /// Stop loss threshold in basis points
    #[arg(long, default_value_t = 50.0)]
    pub stop_loss_bps: f64,

    /// Maximum position age in milliseconds
    #[arg(long, default_value_t = 300000)]
    pub max_position_age_ms: i64,

    /// Inventory reduction threshold (0.0-1.0)
    #[arg(long, default_value_t = 0.7)]
    pub inventory_reduction_threshold: f64,

    /// Aggressive close threshold (0.0-1.0)
    #[arg(long, default_value_t = 0.9)]
    pub aggressive_close_threshold: f64,

    /// Minimum profit in basis points
    #[arg(long, default_value_t = 5.0)]
    pub min_profit_bps: f64,

    /// Volatility window size
    #[arg(long, default_value_t = 30)]
    pub volatility_window: usize,

    /// Maximum volatility threshold
    #[arg(long, default_value_t = 0.0001)]
    pub max_volatility_threshold: f64,

    /// Volatility cooldown in milliseconds
    #[arg(long, default_value_t = 5000)]
    pub volatility_cooldown_ms: i64,

    /// Momentum window size
    #[arg(long, default_value_t = 10)]
    pub momentum_window: usize,

    /// Momentum threshold
    #[arg(long, default_value_t = 0.0015)]
    pub momentum_threshold: f64,

    /// Momentum cooldown in milliseconds
    #[arg(long, default_value_t = 3000)]
    pub momentum_cooldown_ms: i64,
}

impl GptMarketMakerArgs {
    pub fn build_strategy(&self, symbol: String) -> Box<dyn Strategy> {
        use crate::strategy::{GptMarketMaker, GptMarketMakerConfig};
        
        let config = GptMarketMakerConfig {
            fix_order_volume: self.fix_order_volume,
            vwap_window: self.vwap_window,
            obi_threshold: self.obi_threshold,
            max_inventory: self.max_inventory,
            use_limit_orders: self.use_limit_orders,
            limit_order_spread_bps: self.limit_order_spread_bps,
            take_profit_bps: self.take_profit_bps,
            stop_loss_bps: self.stop_loss_bps,
            max_position_age_ms: self.max_position_age_ms,
            inventory_reduction_threshold: self.inventory_reduction_threshold,
            aggressive_close_threshold: self.aggressive_close_threshold,
            min_profit_bps: self.min_profit_bps,
            volatility_window: self.volatility_window,
            max_volatility_threshold: self.max_volatility_threshold,
            volatility_cooldown_ms: self.volatility_cooldown_ms,
            momentum_window: self.momentum_window,
            momentum_threshold: self.momentum_threshold,
            momentum_cooldown_ms: self.momentum_cooldown_ms,
        };
        
        Box::new(GptMarketMaker::new(symbol, config))
    }
}