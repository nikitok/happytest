//! Data sink implementations for writing orderbook data.
//!
//! Provides unified writers for persisting orderbook data to various formats.

pub mod base;
pub mod jsonl;
pub mod parquet;

pub use base::{StorageWriter, WriterConfig};
pub use jsonl::JsonlWriter;
pub use parquet::ParquetWriter;
