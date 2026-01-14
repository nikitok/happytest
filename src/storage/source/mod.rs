//! Data source implementations for reading orderbook data.
//!
//! Provides unified access to orderbook data from various file formats.

// Re-export from utils (will be migrated in future)
pub use crate::utils::loader::{extract_symbol_from_filename, FileDataSource, OrderBookMessage};
pub use crate::utils::parquet_loader::ParquetDataSource;

// Future: MultiFileDataSource will be here
