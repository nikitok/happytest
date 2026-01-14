//! Volatility indicator based on price returns standard deviation.
//!
//! Measures price volatility using a sliding window of returns.

use std::collections::VecDeque;

/// Volatility detector using rolling standard deviation of returns.
#[derive(Debug, Clone)]
pub struct VolatilityDetector {
    window: usize,
    prices: VecDeque<f64>,
    threshold: f64,
    cooldown_ms: i64,
    last_high_volatility_time: i64,
}

impl VolatilityDetector {
    /// Create a new volatility detector.
    ///
    /// # Arguments
    /// * `window` - Number of price observations for calculation
    /// * `threshold` - Volatility level considered "high"
    /// * `cooldown_ms` - Cooldown period after high volatility detection
    pub fn new(window: usize, threshold: f64, cooldown_ms: i64) -> Self {
        Self {
            window,
            prices: VecDeque::with_capacity(window),
            threshold,
            cooldown_ms,
            last_high_volatility_time: 0,
        }
    }

    /// Update with a new price observation.
    pub fn update(&mut self, price: f64) {
        self.prices.push_back(price);
        if self.prices.len() > self.window {
            self.prices.pop_front();
        }
    }

    /// Calculate current volatility (standard deviation of returns).
    pub fn value(&self) -> f64 {
        if self.prices.len() < 2 {
            return 0.0;
        }

        let prices: Vec<f64> = self.prices.iter().cloned().collect();
        let mut returns = Vec::with_capacity(prices.len() - 1);

        for i in 1..prices.len() {
            let ret = (prices[i] - prices[i - 1]) / prices[i - 1];
            returns.push(ret);
        }

        if returns.is_empty() {
            return 0.0;
        }

        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance =
            returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;

        variance.sqrt()
    }

    /// Check if volatility is currently high.
    pub fn is_high(&self) -> bool {
        let vol = self.value();
        vol > 0.0 && vol > self.threshold
    }

    /// Check if we can trade (not in cooldown and volatility is acceptable).
    ///
    /// Updates the last high volatility time if high volatility is detected.
    pub fn can_trade(&mut self, current_time: i64) -> (bool, Option<String>) {
        // Check cooldown
        if current_time - self.last_high_volatility_time < self.cooldown_ms {
            let remaining = (self.cooldown_ms - (current_time - self.last_high_volatility_time))
                as f64
                / 1000.0;
            return (
                false,
                Some(format!("VOLATILITY_COOLDOWN: {:.1}s remaining", remaining)),
            );
        }

        // Check current volatility
        let vol = self.value();
        if vol > 0.0 && vol > self.threshold {
            self.last_high_volatility_time = current_time;
            return (
                false,
                Some(format!(
                    "HIGH_VOLATILITY: {:.4} > {:.4}",
                    vol, self.threshold
                )),
            );
        }

        (true, None)
    }

    /// Reset the detector, clearing all state.
    pub fn reset(&mut self) {
        self.prices.clear();
        self.last_high_volatility_time = 0;
    }

    /// Check if enough data for calculation.
    pub fn is_ready(&self) -> bool {
        self.prices.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_not_ready() {
        let detector = VolatilityDetector::new(10, 0.01, 5000);
        assert!(!detector.is_ready());
        assert_eq!(detector.value(), 0.0);
    }

    #[test]
    fn test_volatility_stable_prices() {
        let mut detector = VolatilityDetector::new(5, 0.01, 5000);

        // All same price = zero volatility
        for _ in 0..5 {
            detector.update(100.0);
        }

        assert!(detector.is_ready());
        assert_eq!(detector.value(), 0.0);
        assert!(!detector.is_high());
    }

    #[test]
    fn test_volatility_detection() {
        let mut detector = VolatilityDetector::new(3, 0.01, 5000);

        // Volatile prices
        detector.update(100.0);
        detector.update(105.0); // +5%
        detector.update(95.0); // -9.5%

        let vol = detector.value();
        assert!(vol > 0.01); // Should be high
        assert!(detector.is_high());
    }

    #[test]
    fn test_volatility_cooldown() {
        let mut detector = VolatilityDetector::new(3, 0.001, 5000);
        // Set last_high_volatility_time to negative so first check passes cooldown
        detector.last_high_volatility_time = -10000;

        // Trigger high volatility
        detector.update(100.0);
        detector.update(105.0);
        detector.update(95.0);

        let (can_trade, reason) = detector.can_trade(1000);
        assert!(!can_trade);
        assert!(reason.unwrap().contains("HIGH_VOLATILITY"));

        // Still in cooldown (1000 + 5000 = 6000, check at 3000)
        let (can_trade, reason) = detector.can_trade(3000);
        assert!(!can_trade);
        assert!(reason.unwrap().contains("COOLDOWN"));

        // After cooldown, with stable prices
        detector.update(100.0);
        detector.update(100.0);
        detector.update(100.0);
        let (can_trade, _) = detector.can_trade(7000);
        assert!(can_trade);
    }

    #[test]
    fn test_volatility_reset() {
        let mut detector = VolatilityDetector::new(3, 0.01, 5000);

        detector.update(100.0);
        detector.update(105.0);
        assert!(detector.is_ready());

        detector.reset();
        assert!(!detector.is_ready());
    }
}
