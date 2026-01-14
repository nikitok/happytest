//! Binance Futures USDT-M exchange connector.
//!
//! Provides WebSocket-based orderbook data collection from Binance Futures.

pub mod models;
pub mod reader;

pub use reader::BinanceReader;
