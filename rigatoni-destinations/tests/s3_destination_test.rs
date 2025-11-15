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

//! Unit tests for S3 destination.

#![cfg(feature = "s3")]

use chrono::Utc;
use rigatoni_destinations::s3::{
    Compression, KeyGenerationStrategy, S3Config, SerializationFormat,
};

#[test]
fn test_serialize_json() {
    let config = S3Config::builder()
        .bucket("test")
        .region("us-east-1")
        .build()
        .unwrap();

    // We can't create S3Destination in tests without AWS credentials,
    // but we can test the config
    assert_eq!(config.format, SerializationFormat::Json);
    assert_eq!(config.compression, Compression::None);
}

#[test]
fn test_key_generation() {
    let config = S3Config::builder()
        .bucket("test-bucket")
        .region("us-east-1")
        .prefix("events")
        .key_strategy(KeyGenerationStrategy::CollectionBased)
        .build()
        .unwrap();

    let key = config.key_strategy.generate_key(
        config.prefix.as_deref(),
        "users",
        &Utc::now(),
        "jsonl",
        "",
    );

    assert!(key.starts_with("events/users/"));
    assert!(key.contains(".jsonl"));
}

#[test]
fn test_metadata() {
    // Test metadata without creating actual S3Destination
    let config = S3Config::builder()
        .bucket("test-bucket")
        .region("us-west-2")
        .prefix("data/mongodb")
        .build()
        .unwrap();

    assert_eq!(config.bucket, "test-bucket");
    assert_eq!(config.region, "us-west-2");
    assert_eq!(config.prefix, Some("data/mongodb".to_string()));
}

#[test]
fn test_compression_extensions() {
    assert_eq!(Compression::None.extension(), "");

    #[cfg(feature = "gzip")]
    assert_eq!(Compression::Gzip.extension(), ".gz");

    #[cfg(feature = "zstandard")]
    assert_eq!(Compression::Zstd.extension(), ".zst");
}
