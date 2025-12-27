//! Momentum indicator measuring price change over a window.
//!
//! Calculates the percentage price change from the oldest to newest observation.

use std::collections::VecDeque;

/// Momentum detector using price change ratio.
#[derive(Debug, Clone)]
pub struct MomentumDetector {
    window: usize,
    prices: VecDeque<f64>,
    threshold: f64,
    cooldown_ms: i64,
    last_strong_momentum_time: i64,
}

impl MomentumDetector {
    /// Create a new momentum detector.
    ///
    /// # Arguments
    /// * `window` - Number of price observations
    /// * `threshold` - Momentum level considered "strong"
    /// * `cooldown_ms` - Cooldown period after strong momentum detection
    pub fn new(window: usize, threshold: f64, cooldown_ms: i64) -> Self {
        Self {
            window,
            prices: VecDeque::with_capacity(window),
            threshold,
            cooldown_ms,
            last_strong_momentum_time: 0,
        }
    }

    /// Update with a new price observation.
    pub fn update(&mut self, price: f64) {
        self.prices.push_back(price);
        if self.prices.len() > self.window {
            self.prices.pop_front();
        }
    }

    /// Calculate current momentum as (latest - oldest) / oldest.
    pub fn value(&self) -> f64 {
        if self.prices.len() < 2 {
            return 0.0;
        }

        let oldest = self.prices.front().unwrap();
        let newest = self.prices.back().unwrap();

        if *oldest == 0.0 {
            return 0.0;
        }

        (newest - oldest) / oldest
    }

    /// Check if momentum is currently strong (exceeds threshold in either direction).
    pub fn is_strong(&self) -> bool {
        let mom = self.value();
        mom != 0.0 && mom.abs() > self.threshold
    }

    /// Get the momentum direction: 1 for up, -1 for down, 0 for neutral.
    pub fn direction(&self) -> i8 {
        let mom = self.value();
        if mom > self.threshold {
            1
        } else if mom < -self.threshold {
            -1
        } else {
            0
        }
    }

    /// Check if we can trade (not in cooldown and momentum is acceptable).
    ///
    /// Updates the last strong momentum time if strong momentum is detected.
    pub fn can_trade(&mut self, current_time: i64) -> (bool, Option<String>) {
        // Check cooldown
        if current_time - self.last_strong_momentum_time < self.cooldown_ms {
            let remaining = (self.cooldown_ms - (current_time - self.last_strong_momentum_time)) as f64 / 1000.0;
            return (false, Some(format!("MOMENTUM_COOLDOWN: {:.1}s remaining", remaining)));
        }

        // Check current momentum
        let mom = self.value();
        if mom != 0.0 && mom.abs() > self.threshold {
            self.last_strong_momentum_time = current_time;
            return (false, Some(format!("STRONG_MOMENTUM: {:.4} > {:.4}", mom, self.threshold)));
        }

        (true, None)
    }

    /// Reset the detector, clearing all state.
    pub fn reset(&mut self) {
        self.prices.clear();
        self.last_strong_momentum_time = 0;
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
    fn test_momentum_not_ready() {
        let detector = MomentumDetector::new(10, 0.01, 3000);
        assert!(!detector.is_ready());
        assert_eq!(detector.value(), 0.0);
    }

    #[test]
    fn test_momentum_stable_prices() {
        let mut detector = MomentumDetector::new(5, 0.01, 3000);

        for _ in 0..5 {
            detector.update(100.0);
        }

        assert!(detector.is_ready());
        assert_eq!(detector.value(), 0.0);
        assert!(!detector.is_strong());
        assert_eq!(detector.direction(), 0);
    }

    #[test]
    fn test_momentum_upward() {
        let mut detector = MomentumDetector::new(3, 0.01, 3000);

        detector.update(100.0);
        detector.update(102.0);
        detector.update(105.0);

        // Momentum = (105 - 100) / 100 = 0.05
        let mom = detector.value();
        assert!((mom - 0.05).abs() < 0.001);
        assert!(detector.is_strong());
        assert_eq!(detector.direction(), 1);
    }

    #[test]
    fn test_momentum_downward() {
        let mut detector = MomentumDetector::new(3, 0.01, 3000);

        detector.update(100.0);
        detector.update(98.0);
        detector.update(95.0);

        // Momentum = (95 - 100) / 100 = -0.05
        let mom = detector.value();
        assert!((mom - (-0.05)).abs() < 0.001);
        assert!(detector.is_strong());
        assert_eq!(detector.direction(), -1);
    }

    #[test]
    fn test_momentum_cooldown() {
        let mut detector = MomentumDetector::new(2, 0.01, 3000);
        // Set last_strong_momentum_time to negative so first check passes cooldown
        detector.last_strong_momentum_time = -10000;

        // Trigger strong momentum
        detector.update(100.0);
        detector.update(110.0);  // 10% move

        let (can_trade, reason) = detector.can_trade(1000);
        assert!(!can_trade);
        assert!(reason.unwrap().contains("STRONG_MOMENTUM"));

        // Still in cooldown (1000 + 3000 = 4000, check at 2000)
        let (can_trade, reason) = detector.can_trade(2000);
        assert!(!can_trade);
        assert!(reason.unwrap().contains("COOLDOWN"));

        // After cooldown with stable prices
        detector.update(110.0);
        let (can_trade, _) = detector.can_trade(5000);
        assert!(can_trade);
    }

    #[test]
    fn test_momentum_sliding_window() {
        let mut detector = MomentumDetector::new(2, 0.01, 3000);

        detector.update(100.0);
        detector.update(105.0);

        // Momentum = (105 - 100) / 100 = 0.05
        assert!((detector.value() - 0.05).abs() < 0.001);

        // Add new price, oldest dropped
        detector.update(108.0);

        // Momentum = (108 - 105) / 105 = 0.0286
        assert!((detector.value() - 0.0286).abs() < 0.001);
    }

    #[test]
    fn test_momentum_reset() {
        let mut detector = MomentumDetector::new(3, 0.01, 3000);

        detector.update(100.0);
        detector.update(105.0);
        assert!(detector.is_ready());

        detector.reset();
        assert!(!detector.is_ready());
    }
}
