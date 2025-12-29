//! Data sink implementations for writing orderbook data.
//!
//! Provides unified writers for persisting orderbook data to various formats,
//! including S3 upload support for AWS Athena integration.

pub mod base;
pub mod jsonl;
pub mod parquet;
pub mod s3_uploader;

pub use base::{StorageWriter, WriterConfig};
pub use jsonl::JsonlWriter;
pub use parquet::ParquetWriter;
pub use s3_uploader::S3Uploader;
