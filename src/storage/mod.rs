//! Storage abstraction for data sources and sinks.
//!
//! This module provides a unified interface for reading and writing
//! orderbook data from various storage formats.
//!
//! ## Structure
//! - `source/` - Reading data (JSONL, Parquet, multi-file)
//! - `sink/` - Writing data (JSONL, Parquet)
//!
//! ## Usage
//! ```rust,ignore
//! use happytest::storage::source::{FileDataSource, ParquetDataSource};
//! use happytest::storage::sink::{JsonlWriter, ParquetWriter, StorageWriter};
//! ```

pub mod sink;
pub mod source;

// Re-export commonly used types at module level
pub use sink::{JsonlWriter, ParquetWriter, S3Uploader, StorageWriter, WriterConfig};
pub use source::{
    extract_symbol_from_filename, FileDataSource, OrderBookMessage, ParquetDataSource,
};
