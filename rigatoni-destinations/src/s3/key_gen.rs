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

//! S3 key generation strategies.
//!
//! This module provides different strategies for generating S3 object keys.
//! The key generation strategy affects how data is partitioned in S3, which
//! impacts query performance, data organization, and costs.
//!
//! # Partitioning Trade-offs
//!
//! - **Date-based partitioning**: Enables efficient time-range queries, reduces scan costs
//! - **Collection-based partitioning**: Groups related data, simplifies access patterns
//! - **Hive-style partitioning**: Compatible with Athena, Presto, Spark for SQL queries
//!
//! # Examples
//!
//! ```rust,ignore
//! use rigatoni_destinations::s3::KeyGenerationStrategy;
//! use chrono::Utc;
//!
//! let strategy = KeyGenerationStrategy::HivePartitioned;
//! let key = strategy.generate_key(
//!     Some("events/mongodb"),  // prefix
//!     "users",                  // collection
//!     &Utc::now(),             // timestamp
//!     "jsonl",                 // extension
//!     "",                      // compression extension
//! );
//! // Result: "events/mongodb/collection=users/year=2025/month=01/day=15/hour=10/1705318800000.jsonl"
//! ```

use chrono::{DateTime, Utc};

/// Key generation strategy for S3 objects.
///
/// Different strategies organize data in S3 using different partitioning schemes.
/// Choose based on your query patterns and analytics requirements.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum KeyGenerationStrategy {
    /// Date and hour partitioned with Hive-style naming.
    ///
    /// Pattern: `{prefix}/collection={name}/year={YYYY}/month={MM}/day={DD}/hour={HH}/{timestamp}.{ext}`
    ///
    /// **Best for**:
    /// - SQL analytics with Athena, Presto, Spark
    /// - Automatic partition discovery
    /// - Time-range queries with hour granularity
    ///
    /// **Example**: `events/collection=users/year=2025/month=01/day=15/hour=10/1705318800000.jsonl`
    HivePartitioned,

    /// Date and hour partitioned with simple naming.
    #[default]
    ///
    /// Pattern: `{prefix}/{collection}/{YYYY}/{MM}/{DD}/{HH}/{timestamp}.{ext}`
    ///
    /// **Best for**:
    /// - Time-range queries
    /// - S3 lifecycle policies by date
    /// - Human-readable structure
    ///
    /// **Example**: `events/users/2025/01/15/10/1705318800000.jsonl`
    DateHourPartitioned,

    /// Date partitioned (no hour) with simple naming.
    ///
    /// Pattern: `{prefix}/{collection}/{YYYY}/{MM}/{DD}/{timestamp}.{ext}`
    ///
    /// **Best for**:
    /// - Daily aggregations
    /// - Lower partition count
    /// - Reduced S3 LIST overhead
    ///
    /// **Example**: `events/users/2025/01/15/1705318800000.jsonl`
    DatePartitioned,

    /// Collection-based partitioning only.
    ///
    /// Pattern: `{prefix}/{collection}/{timestamp}.{ext}`
    ///
    /// **Best for**:
    /// - Collection-specific access patterns
    /// - Simple organization
    /// - Minimal partition overhead
    ///
    /// **Example**: `events/users/1705318800000.jsonl`
    CollectionBased,

    /// Flat structure with timestamp only.
    ///
    /// Pattern: `{prefix}/{collection}_{timestamp}.{ext}`
    ///
    /// **Best for**:
    /// - Simple backups
    /// - Single-file exports
    /// - Testing
    ///
    /// **Example**: `events/users_1705318800000.jsonl`
    Flat,
}

impl KeyGenerationStrategy {
    /// Generates an S3 key based on the strategy.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Optional prefix for all keys (e.g., "events/mongodb")
    /// * `collection` - Collection or table name
    /// * `timestamp` - Timestamp for partitioning and file naming
    /// * `extension` - File extension (e.g., "jsonl", "parquet")
    /// * `compression_ext` - Compression extension (e.g., ".gz", ".zst", or empty)
    ///
    /// # Returns
    ///
    /// A complete S3 key ready for use with PutObject.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use chrono::Utc;
    /// use rigatoni_destinations::s3::KeyGenerationStrategy;
    ///
    /// let strategy = KeyGenerationStrategy::DateHourPartitioned;
    /// let key = strategy.generate_key(
    ///     Some("data"),
    ///     "orders",
    ///     &Utc::now(),
    ///     "jsonl",
    ///     ".gz",
    /// );
    /// // Result: "data/orders/2025/01/15/10/1705318800000.jsonl.gz"
    /// ```
    #[must_use]
    pub fn generate_key(
        &self,
        prefix: Option<&str>,
        collection: &str,
        timestamp: &DateTime<Utc>,
        extension: &str,
        compression_ext: &str,
    ) -> String {
        let timestamp_millis = timestamp.timestamp_millis();
        let year = timestamp.format("%Y");
        let month = timestamp.format("%m");
        let day = timestamp.format("%d");
        let hour = timestamp.format("%H");

        let base_prefix = prefix.unwrap_or("");
        let prefix_part = if base_prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", base_prefix)
        };

        match self {
            Self::HivePartitioned => {
                format!(
                    "{}collection={}/year={}/month={}/day={}/hour={}/{}.{}{}",
                    prefix_part,
                    collection,
                    year,
                    month,
                    day,
                    hour,
                    timestamp_millis,
                    extension,
                    compression_ext
                )
            }
            Self::DateHourPartitioned => {
                format!(
                    "{}{}/{}/{}/{}/{}/{}.{}{}",
                    prefix_part,
                    collection,
                    year,
                    month,
                    day,
                    hour,
                    timestamp_millis,
                    extension,
                    compression_ext
                )
            }
            Self::DatePartitioned => {
                format!(
                    "{}{}/{}/{}/{}/{}.{}{}",
                    prefix_part,
                    collection,
                    year,
                    month,
                    day,
                    timestamp_millis,
                    extension,
                    compression_ext
                )
            }
            Self::CollectionBased => {
                format!(
                    "{}{}/{}.{}{}",
                    prefix_part, collection, timestamp_millis, extension, compression_ext
                )
            }
            Self::Flat => {
                format!(
                    "{}{}_{}.{}{}",
                    prefix_part, collection, timestamp_millis, extension, compression_ext
                )
            }
        }
    }

    /// Returns a human-readable description of the key pattern.
    #[must_use]
    pub const fn pattern_description(&self) -> &'static str {
        match self {
            Self::HivePartitioned => "{prefix}/collection={name}/year={YYYY}/month={MM}/day={DD}/hour={HH}/{timestamp}.{ext}",
            Self::DateHourPartitioned => "{prefix}/{collection}/{YYYY}/{MM}/{DD}/{HH}/{timestamp}.{ext}",
            Self::DatePartitioned => "{prefix}/{collection}/{YYYY}/{MM}/{DD}/{timestamp}.{ext}",
            Self::CollectionBased => "{prefix}/{collection}/{timestamp}.{ext}",
            Self::Flat => "{prefix}/{collection}_{timestamp}.{ext}",
        }
    }
}
