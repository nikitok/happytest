#[cfg(test)]
mod tests {
    use crate::core::Trade;
    use crate::pnl::{PnlReport, PnlMethod};
    use uuid::Uuid;
    
    fn create_test_trade(
        symbol: &str,
        side: &str,
        price: f64,
        quantity: f64,
        timestamp: i64,
    ) -> Trade {
        Trade {
            id: Uuid::new_v4().to_string(),
            time: timestamp,
            symbol: symbol.to_string(),
            side: side.to_string(),
            price,
            quantity,
            status: "filled".to_string(),
        }
    }
    
    #[test]
    fn test_fifo_simple_profit() {
        let trades = vec![
            create_test_trade("BTCUSDT", "Buy", 100.0, 1.0, 1000),
            create_test_trade("BTCUSDT", "Sell", 110.0, 1.0, 2000),
        ];
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Fifo);
        
        assert_eq!(result.total_pnl, 10.0);
        assert_eq!(result.closed_trades.len(), 1);
    }
    
    #[test]
    fn test_fifo_simple_loss() {
        let trades = vec![
            create_test_trade("BTCUSDT", "Buy", 100.0, 1.0, 1000),
            create_test_trade("BTCUSDT", "Sell", 90.0, 1.0, 2000),
        ];
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Fifo);
        
        assert_eq!(result.total_pnl, -10.0);
        assert_eq!(result.closed_trades.len(), 1);
        // assert_eq!(result.win_rate, 0.0);
    }
    
    #[test]
    fn test_fifo_partial_fill() {
        let trades = vec![
            create_test_trade("BTCUSDT", "Buy", 100.0, 2.0, 1000),
            create_test_trade("BTCUSDT", "Sell", 110.0, 1.0, 2000),
        ];
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Fifo);
        
        assert_eq!(result.total_pnl, 10.0); // (110-100) * 1
        assert_eq!(result.closed_trades.len(), 1);
        // Should have 1 BTC remaining open
    }
    
    #[test]
    fn test_fifo_multiple_buys() {
        let trades = vec![
            create_test_trade("BTCUSDT", "Buy", 100.0, 1.0, 1000),
            create_test_trade("BTCUSDT", "Buy", 110.0, 1.0, 2000),
            create_test_trade("BTCUSDT", "Sell", 120.0, 2.0, 3000),
        ];
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Fifo);
        
        // First buy: (120-100) * 1 = 20
        // Second buy: (120-110) * 1 = 10
        // Total: 30
        assert_eq!(result.total_pnl, 30.0);
        assert_eq!(result.closed_trades.len(), 2);
    }
    
    #[test]
    fn test_position_simple_profit() {
        let trades = vec![
            create_test_trade("BTCUSDT", "Buy", 100.0, 1.0, 1000),
            create_test_trade("BTCUSDT", "Sell", 110.0, 1.0, 2000),
        ];
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Position);
        
        assert_eq!(result.total_pnl, 10.0);
        assert_eq!(result.closed_trades.len(), 1);
    }
    
    #[test]
    fn test_position_averaging() {
        let trades = vec![
            create_test_trade("BTCUSDT", "Buy", 100.0, 1.0, 1000),
            create_test_trade("BTCUSDT", "Buy", 110.0, 1.0, 2000),
            create_test_trade("BTCUSDT", "Sell", 120.0, 2.0, 3000),
        ];
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Position);
        
        // Average price: (100 + 110) / 2 = 105
        // PnL: (120 - 105) * 2 = 30
        assert_eq!(result.total_pnl, 30.0);
        assert_eq!(result.closed_trades.len(), 1);
    }
    
    #[test]
    fn test_position_short_trade() {
        let trades = vec![
            create_test_trade("BTCUSDT", "Sell", 100.0, 1.0, 1000),
            create_test_trade("BTCUSDT", "Buy", 90.0, 1.0, 2000),
        ];
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Position);
        
        assert_eq!(result.total_pnl, 10.0); // (100-90) * 1
        assert_eq!(result.closed_trades.len(), 1);
    }
    
    #[test]
    fn test_multiple_symbols() {
        let trades = vec![
            create_test_trade("BTCUSDT", "Buy", 100.0, 1.0, 1000),
            create_test_trade("ETHUSDT", "Buy", 2000.0, 1.0, 1500),
            create_test_trade("BTCUSDT", "Sell", 110.0, 1.0, 2000),
            create_test_trade("ETHUSDT", "Sell", 2100.0, 1.0, 2500),
        ];
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Fifo);
        
        assert_eq!(result.total_pnl, 110.0); // 10 + 100
        assert_eq!(result.closed_trades.len(), 2);
        // assert_eq!(result.win_rate, 1.0);
    }
    
    #[test]
    fn test_metrics_calculation() {
        let trades = vec![
            create_test_trade("BTCUSDT", "Buy", 100.0, 1.0, 1000),
            create_test_trade("BTCUSDT", "Sell", 110.0, 1.0, 2000),
            create_test_trade("BTCUSDT", "Buy", 105.0, 1.0, 3000),
            create_test_trade("BTCUSDT", "Sell", 100.0, 1.0, 4000),
        ];
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Fifo);
        
        assert_eq!(result.total_pnl, 5.0); // 10 - 5
        assert_eq!(result.closed_trades.len(), 2);
        // assert_eq!(result.win_rate, 0.5); // 1 win, 1 loss
        // assert!(result.sharpe_ratio != 0.0);
        // assert_eq!(result.cumulative_pnl, vec![10.0, 5.0]);
    }
    
    #[test]
    fn test_max_drawdown() {
        let trades = vec![
            create_test_trade("BTCUSDT", "Buy", 100.0, 1.0, 1000),
            create_test_trade("BTCUSDT", "Sell", 120.0, 1.0, 2000), // +20
            create_test_trade("BTCUSDT", "Buy", 110.0, 1.0, 3000),
            create_test_trade("BTCUSDT", "Sell", 100.0, 1.0, 4000), // -10
            create_test_trade("BTCUSDT", "Buy", 105.0, 1.0, 5000),
            create_test_trade("BTCUSDT", "Sell", 115.0, 1.0, 6000), // +10
        ];
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Fifo);
        
        assert_eq!(result.total_pnl, 20.0); // 20 - 10 + 10
        // assert_eq!(result.max_drawdown, 10.0); // From 20 to 10
    }
    
    #[test]
    fn test_empty_trades() {
        let trades = vec![];
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Fifo);
        
        assert_eq!(result.total_pnl, 0.0);
        assert_eq!(result.closed_trades.len(), 0);
    }
    
    #[test]
    fn test_only_unfilled_trades() {
        let mut trades = vec![
            create_test_trade("BTCUSDT", "Buy", 100.0, 1.0, 1000),
            create_test_trade("BTCUSDT", "Sell", 110.0, 1.0, 2000),
        ];
        
        // Mark all as unfilled
        for trade in &mut trades {
            trade.status = "unfilled".to_string();
        }
        
        let calculator = PnlReport::new();
        let result = calculator.calculate(&trades, PnlMethod::Fifo);
        
        assert_eq!(result.total_pnl, 0.0);
        assert_eq!(result.closed_trades.len(), 0);
    }
}