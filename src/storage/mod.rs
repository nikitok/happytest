//! Storage abstraction for data sources and sinks.
//!
//! This module provides a unified interface for reading and writing
//! orderbook data from various storage formats.
//!
//! ## Structure
//! - `source/` - Reading data (JSONL, Parquet, multi-file)
//! - `sink/` - Writing data (future: move from reader/storage/)
//!
//! ## Usage
//! ```rust,ignore
//! use happytest::storage::source::{FileDataSource, ParquetDataSource};
//! ```

pub mod source;

// Re-export commonly used types at module level
pub use source::{FileDataSource, ParquetDataSource, OrderBookMessage, extract_symbol_from_filename};
