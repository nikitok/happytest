//! Volume Weighted Average Price (VWAP) indicator.
//!
//! Calculates the average price weighted by volume over a sliding window.

use std::collections::VecDeque;

/// VWAP calculator using a sliding window approach.
#[derive(Debug, Clone)]
pub struct Vwap {
    window: usize,
    price_volume_products: VecDeque<f64>,
    volumes: VecDeque<f64>,
}

impl Vwap {
    /// Create a new VWAP calculator with the specified window size.
    pub fn new(window: usize) -> Self {
        Self {
            window,
            price_volume_products: VecDeque::with_capacity(window),
            volumes: VecDeque::with_capacity(window),
        }
    }

    /// Update the VWAP with a new price and volume observation.
    pub fn update(&mut self, price: f64, volume: f64) {
        self.price_volume_products.push_back(price * volume);
        self.volumes.push_back(volume);

        if self.price_volume_products.len() > self.window {
            self.price_volume_products.pop_front();
            self.volumes.pop_front();
        }
    }

    /// Get the current VWAP value.
    /// Returns None if the window is not yet filled or if total volume is zero.
    pub fn value(&self) -> Option<f64> {
        if self.volumes.len() < self.window {
            return None;
        }

        let sum_pv: f64 = self.price_volume_products.iter().sum();
        let sum_vol: f64 = self.volumes.iter().sum();

        if sum_vol == 0.0 {
            None
        } else {
            Some(sum_pv / sum_vol)
        }
    }

    /// Check if the VWAP has enough data to produce a value.
    pub fn is_ready(&self) -> bool {
        self.volumes.len() >= self.window
    }

    /// Reset the calculator, clearing all data.
    pub fn reset(&mut self) {
        self.price_volume_products.clear();
        self.volumes.clear();
    }

    /// Get the current window size.
    pub fn window_size(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vwap_not_ready_until_window_filled() {
        let mut vwap = Vwap::new(3);

        vwap.update(100.0, 10.0);
        assert!(!vwap.is_ready());
        assert!(vwap.value().is_none());

        vwap.update(101.0, 20.0);
        assert!(!vwap.is_ready());

        vwap.update(102.0, 15.0);
        assert!(vwap.is_ready());
        assert!(vwap.value().is_some());
    }

    #[test]
    fn test_vwap_calculation() {
        let mut vwap = Vwap::new(3);

        // Prices: 100, 102, 104 with volumes: 10, 20, 10
        vwap.update(100.0, 10.0);
        vwap.update(102.0, 20.0);
        vwap.update(104.0, 10.0);

        // VWAP = (100*10 + 102*20 + 104*10) / (10 + 20 + 10)
        // VWAP = (1000 + 2040 + 1040) / 40 = 4080 / 40 = 102
        let value = vwap.value().unwrap();
        assert!((value - 102.0).abs() < 0.001);
    }

    #[test]
    fn test_vwap_sliding_window() {
        let mut vwap = Vwap::new(2);

        vwap.update(100.0, 10.0);
        vwap.update(102.0, 10.0);

        // VWAP = (100*10 + 102*10) / 20 = 101
        assert!((vwap.value().unwrap() - 101.0).abs() < 0.001);

        // Add new value, oldest should be dropped
        vwap.update(106.0, 10.0);

        // VWAP = (102*10 + 106*10) / 20 = 104
        assert!((vwap.value().unwrap() - 104.0).abs() < 0.001);
    }

    #[test]
    fn test_vwap_zero_volume() {
        let mut vwap = Vwap::new(2);

        vwap.update(100.0, 0.0);
        vwap.update(102.0, 0.0);

        assert!(vwap.value().is_none());
    }

    #[test]
    fn test_vwap_reset() {
        let mut vwap = Vwap::new(2);

        vwap.update(100.0, 10.0);
        vwap.update(102.0, 10.0);
        assert!(vwap.is_ready());

        vwap.reset();
        assert!(!vwap.is_ready());
        assert!(vwap.value().is_none());
    }
}
