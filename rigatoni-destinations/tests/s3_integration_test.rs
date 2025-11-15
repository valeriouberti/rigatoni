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

//! Integration tests for S3 destination using LocalStack.
//!
//! These tests require LocalStack to be running. Start it with:
//!
//! ```bash
//! docker-compose up -d
//! ```
//!
//! Then run the tests:
//!
//! ```bash
//! cargo test --package rigatoni-destinations --test s3_integration_test --features s3,json,gzip,zstandard
//! ```

#![cfg(feature = "s3")]

use bson::doc;
use chrono::Utc;
use rigatoni_core::destination::Destination;
use rigatoni_core::event::{ChangeEvent, Namespace, OperationType};
use rigatoni_destinations::s3::{
    KeyGenerationStrategy, S3Config, S3Destination, SerializationFormat,
};
use std::env;

#[cfg(any(feature = "gzip", feature = "zstandard"))]
use rigatoni_destinations::s3::Compression;

/// Helper to check if LocalStack is available
fn is_localstack_available() -> bool {
    env::var("LOCALSTACK_ENDPOINT").is_ok() || env::var("CI").is_err()
}

/// Helper to get LocalStack endpoint
fn get_localstack_endpoint() -> String {
    env::var("LOCALSTACK_ENDPOINT").unwrap_or_else(|_| "http://localhost:4566".to_string())
}

/// Create a test event
fn create_test_event(collection: &str, id: i32) -> ChangeEvent {
    ChangeEvent {
        resume_token: doc! { "_data": format!("test_{}", id) },
        operation: OperationType::Insert,
        namespace: Namespace::new("test_db", collection),
        full_document: Some(doc! {
            "_id": id,
            "name": format!("test_doc_{}", id),
            "value": id * 10,
        }),
        document_key: Some(doc! { "_id": id }),
        update_description: None,
        cluster_time: Utc::now(),
    }
}

#[tokio::test]
#[ignore] // Requires LocalStack running
async fn test_s3_basic_write() {
    if !is_localstack_available() {
        eprintln!("Skipping test: LocalStack not available");
        return;
    }

    // Create S3 config for LocalStack
    let config = S3Config::builder()
        .bucket("test-bucket")
        .region("us-east-1")
        .prefix("integration-tests/basic")
        .endpoint_url(get_localstack_endpoint())
        .force_path_style(true) // Required for LocalStack
        .build()
        .expect("Failed to build config");

    let mut destination = S3Destination::new(config)
        .await
        .expect("Failed to create S3 destination");

    // Create test events
    let events = vec![
        create_test_event("users", 1),
        create_test_event("users", 2),
        create_test_event("products", 101),
    ];

    // Write events
    destination
        .write_batch(events)
        .await
        .expect("Failed to write batch");

    // Flush
    destination.flush().await.expect("Failed to flush");

    // Verify buffered count is 0 after flush
    assert_eq!(destination.buffered_count(), 0);

    // Close
    destination.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore] // Requires LocalStack running
#[cfg(feature = "gzip")]
async fn test_s3_with_gzip_compression() {
    if !is_localstack_available() {
        return;
    }

    let config = S3Config::builder()
        .bucket("test-bucket")
        .region("us-east-1")
        .prefix("integration-tests/compressed")
        .compression(Compression::Gzip)
        .endpoint_url(get_localstack_endpoint())
        .force_path_style(true)
        .build()
        .expect("Failed to build config");

    let mut destination = S3Destination::new(config)
        .await
        .expect("Failed to create S3 destination");

    // Create multiple events to test compression
    let events: Vec<_> = (1..=20)
        .map(|i| create_test_event("analytics", i))
        .collect();

    destination
        .write_batch(events)
        .await
        .expect("Failed to write batch");

    destination.flush().await.expect("Failed to flush");
    destination.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore] // Requires LocalStack running
#[cfg(feature = "zstandard")]
async fn test_s3_with_zstd_compression() {
    if !is_localstack_available() {
        return;
    }

    let config = S3Config::builder()
        .bucket("test-bucket")
        .region("us-east-1")
        .prefix("integration-tests/zstd")
        .compression(Compression::Zstd)
        .endpoint_url(get_localstack_endpoint())
        .force_path_style(true)
        .build()
        .expect("Failed to build config");

    let mut destination = S3Destination::new(config)
        .await
        .expect("Failed to create S3 destination");

    let events: Vec<_> = (1..=20).map(|i| create_test_event("metrics", i)).collect();

    destination
        .write_batch(events)
        .await
        .expect("Failed to write batch");

    destination.flush().await.expect("Failed to flush");
    destination.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore] // Requires LocalStack running
async fn test_s3_hive_partitioning() {
    if !is_localstack_available() {
        return;
    }

    let config = S3Config::builder()
        .bucket("test-bucket")
        .region("us-east-1")
        .prefix("data-lake")
        .key_strategy(KeyGenerationStrategy::HivePartitioned)
        .endpoint_url(get_localstack_endpoint())
        .force_path_style(true)
        .build()
        .expect("Failed to build config");

    let mut destination = S3Destination::new(config)
        .await
        .expect("Failed to create S3 destination");

    // Create events from different collections
    let events = vec![
        create_test_event("users", 1),
        create_test_event("orders", 1),
        create_test_event("products", 1),
    ];

    destination
        .write_batch(events)
        .await
        .expect("Failed to write batch");

    destination.flush().await.expect("Failed to flush");
    destination.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore] // Requires LocalStack running
#[cfg(feature = "csv")]
async fn test_s3_csv_format() {
    if !is_localstack_available() {
        return;
    }

    let config = S3Config::builder()
        .bucket("test-bucket")
        .region("us-east-1")
        .prefix("exports/csv")
        .format(SerializationFormat::Csv)
        .endpoint_url(get_localstack_endpoint())
        .force_path_style(true)
        .build()
        .expect("Failed to build config");

    let mut destination = S3Destination::new(config)
        .await
        .expect("Failed to create S3 destination");

    let events = vec![
        create_test_event("exports", 1),
        create_test_event("exports", 2),
    ];

    destination
        .write_batch(events)
        .await
        .expect("Failed to write batch");

    destination.flush().await.expect("Failed to flush");
    destination.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore] // Requires LocalStack running
#[cfg(feature = "parquet")]
async fn test_s3_parquet_format() {
    if !is_localstack_available() {
        return;
    }

    let config = S3Config::builder()
        .bucket("test-bucket")
        .region("us-east-1")
        .prefix("analytics/parquet")
        .format(SerializationFormat::Parquet)
        .endpoint_url(get_localstack_endpoint())
        .force_path_style(true)
        .build()
        .expect("Failed to build config");

    let mut destination = S3Destination::new(config)
        .await
        .expect("Failed to create S3 destination");

    let events = vec![
        create_test_event("events", 1),
        create_test_event("events", 2),
    ];

    destination
        .write_batch(events)
        .await
        .expect("Failed to write batch");

    destination.flush().await.expect("Failed to flush");
    destination.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore] // Requires LocalStack running
#[cfg(feature = "avro")]
async fn test_s3_avro_format() {
    if !is_localstack_available() {
        return;
    }

    let config = S3Config::builder()
        .bucket("test-bucket")
        .region("us-east-1")
        .prefix("streaming/avro")
        .format(SerializationFormat::Avro)
        .endpoint_url(get_localstack_endpoint())
        .force_path_style(true)
        .build()
        .expect("Failed to build config");

    let mut destination = S3Destination::new(config)
        .await
        .expect("Failed to create S3 destination");

    let events = vec![
        create_test_event("stream", 1),
        create_test_event("stream", 2),
    ];

    destination
        .write_batch(events)
        .await
        .expect("Failed to write batch");

    destination.flush().await.expect("Failed to flush");
    destination.close().await.expect("Failed to close");
}

#[tokio::test]
#[ignore] // Requires LocalStack running
async fn test_s3_metadata() {
    if !is_localstack_available() {
        return;
    }

    let config = S3Config::builder()
        .bucket("test-bucket")
        .region("us-east-1")
        .endpoint_url(get_localstack_endpoint())
        .force_path_style(true)
        .build()
        .expect("Failed to build config");

    let destination = S3Destination::new(config)
        .await
        .expect("Failed to create S3 destination");

    let metadata = destination.metadata();

    assert_eq!(metadata.name, "AWS S3");
    assert_eq!(metadata.destination_type, "s3");
    assert!(!metadata.supports_transactions);
    assert!(metadata.supports_concurrent_writes);
    assert_eq!(
        metadata.properties.get("bucket"),
        Some(&"test-bucket".to_string())
    );
}
