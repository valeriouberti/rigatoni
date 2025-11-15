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

//! Unit tests for S3 key generation strategies.

#![cfg(feature = "s3")]

use chrono::{TimeZone, Utc};
use rigatoni_destinations::s3::KeyGenerationStrategy;

/// Helper function to create a fixed timestamp for testing
/// Fixed timestamp: 2025-01-15 10:30:45 UTC
fn test_timestamp() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 1, 15, 10, 30, 45).unwrap()
}

#[test]
fn test_hive_partitioned() {
    let strategy = KeyGenerationStrategy::HivePartitioned;
    let timestamp = test_timestamp();

    let key = strategy.generate_key(Some("data"), "users", &timestamp, "jsonl", "");

    assert_eq!(
        key,
        format!(
            "data/collection=users/year=2025/month=01/day=15/hour=10/{}.jsonl",
            timestamp.timestamp_millis()
        )
    );
}

#[test]
fn test_hive_partitioned_with_compression() {
    let strategy = KeyGenerationStrategy::HivePartitioned;
    let timestamp = test_timestamp();

    let key = strategy.generate_key(Some("data"), "orders", &timestamp, "jsonl", ".gz");

    assert_eq!(
        key,
        format!(
            "data/collection=orders/year=2025/month=01/day=15/hour=10/{}.jsonl.gz",
            timestamp.timestamp_millis()
        )
    );
}

#[test]
fn test_date_hour_partitioned() {
    let strategy = KeyGenerationStrategy::DateHourPartitioned;
    let timestamp = test_timestamp();

    let key = strategy.generate_key(Some("events"), "products", &timestamp, "csv", "");

    assert_eq!(
        key,
        format!(
            "events/products/2025/01/15/10/{}.csv",
            timestamp.timestamp_millis()
        )
    );
}

#[test]
fn test_date_partitioned() {
    let strategy = KeyGenerationStrategy::DatePartitioned;
    let timestamp = test_timestamp();

    let key = strategy.generate_key(Some("daily"), "analytics", &timestamp, "parquet", ".zst");

    assert_eq!(
        key,
        format!(
            "daily/analytics/2025/01/15/{}.parquet.zst",
            timestamp.timestamp_millis()
        )
    );
}

#[test]
fn test_collection_based() {
    let strategy = KeyGenerationStrategy::CollectionBased;
    let timestamp = test_timestamp();

    let key = strategy.generate_key(Some("collections"), "metrics", &timestamp, "avro", "");

    assert_eq!(
        key,
        format!("collections/metrics/{}.avro", timestamp.timestamp_millis())
    );
}

#[test]
fn test_flat() {
    let strategy = KeyGenerationStrategy::Flat;
    let timestamp = test_timestamp();

    let key = strategy.generate_key(Some("backup"), "users", &timestamp, "jsonl", ".gz");

    assert_eq!(
        key,
        format!("backup/users_{}.jsonl.gz", timestamp.timestamp_millis())
    );
}

#[test]
fn test_no_prefix() {
    let strategy = KeyGenerationStrategy::CollectionBased;
    let timestamp = test_timestamp();

    let key = strategy.generate_key(None, "events", &timestamp, "jsonl", "");

    assert_eq!(
        key,
        format!("events/{}.jsonl", timestamp.timestamp_millis())
    );
}

#[test]
fn test_empty_prefix() {
    let strategy = KeyGenerationStrategy::DateHourPartitioned;
    let timestamp = test_timestamp();

    let key = strategy.generate_key(Some(""), "users", &timestamp, "jsonl", "");

    // Empty prefix should be treated like no prefix
    assert_eq!(
        key,
        format!("users/2025/01/15/10/{}.jsonl", timestamp.timestamp_millis())
    );
}

#[test]
fn test_pattern_descriptions() {
    assert!(KeyGenerationStrategy::HivePartitioned
        .pattern_description()
        .contains("collection="));
    assert!(KeyGenerationStrategy::DateHourPartitioned
        .pattern_description()
        .contains("{HH}"));
    assert!(KeyGenerationStrategy::DatePartitioned
        .pattern_description()
        .contains("{DD}"));
    assert!(KeyGenerationStrategy::CollectionBased
        .pattern_description()
        .contains("{collection}"));
    assert!(KeyGenerationStrategy::Flat
        .pattern_description()
        .contains('_'));
}

#[test]
fn test_default_strategy() {
    // Default should be DateHourPartitioned
    let default_strategy = KeyGenerationStrategy::default();
    assert!(matches!(
        default_strategy,
        KeyGenerationStrategy::DateHourPartitioned
    ));
}

#[test]
fn test_different_timestamps() {
    let strategy = KeyGenerationStrategy::HivePartitioned;

    let ts1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let ts2 = Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap();

    let key1 = strategy.generate_key(None, "test", &ts1, "jsonl", "");
    let key2 = strategy.generate_key(None, "test", &ts2, "jsonl", "");

    assert!(key1.contains("year=2025/month=01/day=01/hour=00"));
    assert!(key2.contains("year=2025/month=12/day=31/hour=23"));
}
