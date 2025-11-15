// Copyright 2025 Rigatoni Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! S3 destination configuration.
//!
//! This module provides configuration options for the S3 destination, including:
//! - Bucket and region configuration
//! - Serialization format selection
//! - Compression options
//! - Key generation strategies
//! - Retry and performance tuning

use crate::s3::key_gen::KeyGenerationStrategy;

/// Serialization format for S3 objects.
///
/// Different formats have different trade-offs:
/// - **JSON**: Human-readable, easy to query with S3 Select, moderate size
/// - **Parquet**: Columnar format, excellent compression, fast queries, requires schema
/// - **CSV**: Simple, widely supported, but limited type support
/// - **Avro**: Schema evolution support, compact, good for streaming
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    /// Newline-delimited JSON (JSONL) - one JSON object per line.
    ///
    /// Best for: Human readability, S3 Select queries, mixed schemas
    /// File extension: `.jsonl` or `.ndjson`
    Json,

    /// Apache Parquet columnar format.
    ///
    /// Best for: Analytics, compression ratio, fast queries
    /// File extension: `.parquet`
    /// Requires: Schema definition
    #[cfg(feature = "parquet")]
    Parquet,

    /// Comma-separated values.
    ///
    /// Best for: Excel compatibility, simple data
    /// File extension: `.csv`
    #[cfg(feature = "csv")]
    Csv,

    /// Apache Avro binary format.
    ///
    /// Best for: Schema evolution, streaming, Kafka integration
    /// File extension: `.avro`
    #[cfg(feature = "avro")]
    Avro,
}

impl SerializationFormat {
    /// Returns the file extension for this format (without the dot).
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Json => "jsonl",
            #[cfg(feature = "parquet")]
            Self::Parquet => "parquet",
            #[cfg(feature = "csv")]
            Self::Csv => "csv",
            #[cfg(feature = "avro")]
            Self::Avro => "avro",
        }
    }

    /// Returns the MIME type for this format.
    #[must_use]
    pub const fn content_type(&self) -> &'static str {
        match self {
            Self::Json => "application/x-ndjson",
            #[cfg(feature = "parquet")]
            Self::Parquet => "application/octet-stream",
            #[cfg(feature = "csv")]
            Self::Csv => "text/csv",
            #[cfg(feature = "avro")]
            Self::Avro => "application/avro",
        }
    }
}

/// Compression algorithm for S3 objects.
///
/// Compression reduces storage costs and bandwidth but adds CPU overhead.
///
/// **Benchmarks** (approximate, data-dependent):
/// - **None**: 0ms CPU, 100MB storage, 100MB bandwidth
/// - **Gzip**: 50ms CPU, 20MB storage, 20MB bandwidth, wide compatibility
/// - **Zstd**: 30ms CPU, 18MB storage, 18MB bandwidth, better ratio & speed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    /// No compression - fastest writes, largest files.
    #[default]
    None,

    /// Gzip compression (RFC 1952).
    ///
    /// Compression level: 6 (default balance of speed/size)
    /// File extension: `.gz`
    /// Best for: Wide compatibility, moderate compression
    #[cfg(feature = "gzip")]
    Gzip,

    /// Zstandard compression.
    ///
    /// Compression level: 3 (default, good balance)
    /// File extension: `.zst`
    /// Best for: Better compression ratio and speed than gzip
    #[cfg(feature = "zstandard")]
    Zstd,
}

impl Compression {
    /// Returns the file extension suffix for this compression (with the dot).
    ///
    /// This is appended to the serialization format extension.
    /// Example: `events.jsonl.gz`
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::None => "",
            #[cfg(feature = "gzip")]
            Self::Gzip => ".gz",
            #[cfg(feature = "zstandard")]
            Self::Zstd => ".zst",
        }
    }

    /// Returns the Content-Encoding header value.
    #[must_use]
    pub const fn encoding(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            #[cfg(feature = "gzip")]
            Self::Gzip => Some("gzip"),
            #[cfg(feature = "zstandard")]
            Self::Zstd => Some("zstd"),
        }
    }
}

/// Configuration for S3 destination.
///
/// # Examples
///
/// ## Basic configuration
///
/// ```rust,ignore
/// use rigatoni_destinations::s3::S3Config;
///
/// let config = S3Config::builder()
///     .bucket("my-data-lake")
///     .region("us-east-1")
///     .prefix("mongodb/events")
///     .build()
///     .unwrap();
/// ```
///
/// ## With compression and custom key strategy
///
/// ```rust,ignore
/// use rigatoni_destinations::s3::{S3Config, Compression, KeyGenerationStrategy};
///
/// let config = S3Config::builder()
///     .bucket("my-data-lake")
///     .region("us-west-2")
///     .prefix("events")
///     .compression(Compression::Zstd)
///     .key_strategy(KeyGenerationStrategy::HivePartitioned)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct S3Config {
    /// S3 bucket name (required).
    pub bucket: String,

    /// AWS region (required).
    ///
    /// Examples: "us-east-1", "eu-west-1", "ap-southeast-2"
    pub region: String,

    /// Key prefix (optional).
    ///
    /// All generated keys will start with this prefix.
    /// Example: "mongodb/prod/events" → keys like "mongodb/prod/events/users/2025/01/15/..."
    pub prefix: Option<String>,

    /// Serialization format (default: JSON).
    pub format: SerializationFormat,

    /// Compression algorithm (default: None).
    pub compression: Compression,

    /// Key generation strategy (default: `DateHourPartitioned`).
    pub key_strategy: KeyGenerationStrategy,

    /// Maximum retries for S3 operations (default: 3).
    ///
    /// The SDK will retry on throttling errors (429, 503) with exponential backoff.
    pub max_retries: u32,

    /// Custom endpoint URL for S3-compatible storage (e.g., MinIO, LocalStack).
    ///
    /// Example: "http://localhost:4566" for LocalStack
    pub endpoint_url: Option<String>,

    /// Whether to use path-style addressing (default: false).
    ///
    /// Path-style: `https://s3.region.amazonaws.com/bucket/key`
    /// Virtual-hosted: `https://bucket.s3.region.amazonaws.com/key`
    ///
    /// Required for: LocalStack, MinIO
    pub force_path_style: bool,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            region: String::from("us-east-1"),
            prefix: None,
            format: SerializationFormat::Json,
            compression: Compression::None,
            key_strategy: KeyGenerationStrategy::DateHourPartitioned,
            max_retries: 3,
            endpoint_url: None,
            force_path_style: false,
        }
    }
}

/// Builder for `S3Config`.
///
/// Provides a fluent API for constructing S3 configuration with validation.
#[derive(Debug, Default)]
pub struct S3ConfigBuilder {
    bucket: Option<String>,
    region: Option<String>,
    prefix: Option<String>,
    format: Option<SerializationFormat>,
    compression: Option<Compression>,
    key_strategy: Option<KeyGenerationStrategy>,
    max_retries: Option<u32>,
    endpoint_url: Option<String>,
    force_path_style: Option<bool>,
}

impl S3ConfigBuilder {
    /// Sets the S3 bucket name (required).
    #[must_use]
    pub fn bucket(mut self, bucket: impl Into<String>) -> Self {
        self.bucket = Some(bucket.into());
        self
    }

    /// Sets the AWS region (required).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// builder.region("us-east-1")
    /// builder.region("eu-west-1")
    /// builder.region("ap-southeast-2")
    /// ```
    #[must_use]
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Sets the key prefix (optional).
    ///
    /// All S3 keys will start with this prefix.
    #[must_use]
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Sets the serialization format (default: JSON).
    #[must_use]
    pub fn format(mut self, format: SerializationFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Sets the compression algorithm (default: None).
    #[must_use]
    pub fn compression(mut self, compression: Compression) -> Self {
        self.compression = Some(compression);
        self
    }

    /// Sets the key generation strategy (default: `DateHourPartitioned`).
    #[must_use]
    pub fn key_strategy(mut self, strategy: KeyGenerationStrategy) -> Self {
        self.key_strategy = Some(strategy);
        self
    }

    /// Sets the maximum number of retries (default: 3).
    #[must_use]
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Sets a custom S3 endpoint URL (for S3-compatible storage).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // LocalStack
    /// builder.endpoint_url("http://localhost:4566")
    ///
    /// // MinIO
    /// builder.endpoint_url("http://minio:9000")
    /// ```
    #[must_use]
    pub fn endpoint_url(mut self, url: impl Into<String>) -> Self {
        self.endpoint_url = Some(url.into());
        self
    }

    /// Forces path-style addressing (required for LocalStack/MinIO).
    ///
    /// When enabled, URLs will be: `https://s3.region.amazonaws.com/bucket/key`
    /// instead of `https://bucket.s3.region.amazonaws.com/key`
    #[must_use]
    pub fn force_path_style(mut self, force: bool) -> Self {
        self.force_path_style = Some(force);
        self
    }

    /// Builds the `S3Config`.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing or invalid:
    /// - `bucket` is required and must not be empty
    /// - `region` is required and must not be empty
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let config = S3Config::builder()
    ///     .bucket("my-bucket")
    ///     .region("us-east-1")
    ///     .build()?;
    /// ```
    pub fn build(self) -> Result<S3Config, String> {
        let bucket = self.bucket.ok_or("bucket is required")?;
        if bucket.is_empty() {
            return Err("bucket cannot be empty".to_string());
        }

        let region = self.region.ok_or("region is required")?;
        if region.is_empty() {
            return Err("region cannot be empty".to_string());
        }

        Ok(S3Config {
            bucket,
            region,
            prefix: self.prefix,
            format: self.format.unwrap_or(SerializationFormat::Json),
            compression: self.compression.unwrap_or_default(),
            key_strategy: self
                .key_strategy
                .unwrap_or(KeyGenerationStrategy::DateHourPartitioned),
            max_retries: self.max_retries.unwrap_or(3),
            endpoint_url: self.endpoint_url,
            force_path_style: self.force_path_style.unwrap_or(false),
        })
    }
}

impl S3Config {
    /// Creates a new builder for `S3Config`.
    #[must_use]
    pub fn builder() -> S3ConfigBuilder {
        S3ConfigBuilder::default()
    }
}
