//! S3 uploader for uploading Parquet files to AWS S3.
//!
//! Provides Athena-compatible partitioning for querying data.

use anyhow::{Context, Result};
use aws_sdk_s3::Client;
use std::path::Path;
use chrono::{DateTime, Utc};

/// S3 Uploader for uploading files to AWS S3 with Athena-compatible partitioning.
pub struct S3Uploader {
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3Uploader {
    /// Create a new S3Uploader
    ///
    /// # Arguments
    /// * `bucket` - S3 bucket name
    /// * `prefix` - Key prefix (e.g., "orderbook/v1")
    /// * `region` - Optional AWS region (uses default if not specified)
    pub async fn new(bucket: String, prefix: String, region: Option<String>) -> Result<Self> {
        let config = if let Some(region) = region {
            aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(region))
                .load()
                .await
        } else {
            aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await
        };

        let client = Client::new(&config);

        // Verify bucket exists by checking if we can access it
        client
            .head_bucket()
            .bucket(&bucket)
            .send()
            .await
            .context(format!("Cannot access S3 bucket: {}", bucket))?;

        log::info!("S3Uploader initialized for bucket: {}", bucket);

        Ok(Self {
            client,
            bucket,
            prefix: prefix.trim_matches('/').to_string(),
        })
    }

    /// Generate Athena-compatible S3 key with partitioning
    ///
    /// Format: `prefix/symbol=BTCUSDT/date=2024-12-28/filename.parquet`
    pub fn generate_athena_key(&self, symbol: &str, timestamp: i64, filename: &str) -> String {
        let datetime = DateTime::<Utc>::from_timestamp_millis(timestamp)
            .unwrap_or_else(Utc::now);
        let date = datetime.format("%Y-%m-%d").to_string();

        if self.prefix.is_empty() {
            format!("symbol={}/date={}/{}", symbol, date, filename)
        } else {
            format!("{}/symbol={}/date={}/{}", self.prefix, symbol, date, filename)
        }
    }

    /// Upload a file to S3
    ///
    /// # Arguments
    /// * `local_path` - Path to the local file
    /// * `s3_key` - S3 object key (use `generate_athena_key` for Athena-compatible keys)
    pub async fn upload_file(&self, local_path: &Path, s3_key: &str) -> Result<()> {
        let body = aws_sdk_s3::primitives::ByteStream::from_path(local_path)
            .await
            .context(format!("Failed to read file: {:?}", local_path))?;

        let content_type = if local_path.extension().map_or(false, |ext| ext == "parquet") {
            "application/vnd.apache.parquet"
        } else if local_path.extension().map_or(false, |ext| ext == "jsonl") {
            "application/x-ndjson"
        } else {
            "application/octet-stream"
        };

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(s3_key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .context(format!("Failed to upload to S3: s3://{}/{}", self.bucket, s3_key))?;

        log::info!("Uploaded to S3: s3://{}/{}", self.bucket, s3_key);

        Ok(())
    }

    /// Upload a file with automatic Athena-compatible key generation
    ///
    /// # Arguments
    /// * `local_path` - Path to the local file
    /// * `symbol` - Trading symbol (e.g., "BTCUSDT")
    /// * `timestamp` - Timestamp in milliseconds
    pub async fn upload_with_partitioning(
        &self,
        local_path: &Path,
        symbol: &str,
        timestamp: i64,
    ) -> Result<String> {
        let filename = local_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("data.parquet");

        let s3_key = self.generate_athena_key(symbol, timestamp, filename);
        self.upload_file(local_path, &s3_key).await?;

        Ok(format!("s3://{}/{}", self.bucket, s3_key))
    }

    /// Get the full S3 URI for a key
    pub fn get_s3_uri(&self, s3_key: &str) -> String {
        format!("s3://{}/{}", self.bucket, s3_key)
    }

    /// Get the bucket name
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Get the prefix
    pub fn prefix(&self) -> &str {
        &self.prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to generate Athena-compatible S3 key (for testing without S3 client)
    fn generate_key(prefix: &str, symbol: &str, timestamp: i64, filename: &str) -> String {
        let datetime = DateTime::<Utc>::from_timestamp_millis(timestamp)
            .unwrap_or_else(Utc::now);
        let date = datetime.format("%Y-%m-%d").to_string();
        let prefix = prefix.trim_matches('/');

        if prefix.is_empty() {
            format!("symbol={}/date={}/{}", symbol, date, filename)
        } else {
            format!("{}/symbol={}/date={}/{}", prefix, symbol, date, filename)
        }
    }

    #[test]
    fn test_generate_athena_key() {
        // 2024-12-28 12:30:00 UTC in milliseconds
        let timestamp = 1735388400000_i64;
        let key = generate_key("orderbook/v1", "BTCUSDT", timestamp, "data.parquet");

        assert!(key.starts_with("orderbook/v1/symbol=BTCUSDT/date=2024-12-28/"));
        assert!(key.ends_with("data.parquet"));
    }

    #[test]
    fn test_generate_athena_key_empty_prefix() {
        let timestamp = 1735388400000_i64;
        let key = generate_key("", "ETHUSDT", timestamp, "data.parquet");

        assert!(key.starts_with("symbol=ETHUSDT/date=2024-12-28/"));
    }

    #[test]
    fn test_generate_athena_key_with_slashes() {
        let timestamp = 1735388400000_i64;
        let key = generate_key("/orderbook/v1/", "BTCUSDT", timestamp, "data.parquet");

        // Should trim leading/trailing slashes
        assert!(key.starts_with("orderbook/v1/symbol="));
    }
}
