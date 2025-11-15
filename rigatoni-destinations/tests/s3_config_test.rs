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

//! Unit tests for S3 configuration.

#![cfg(feature = "s3")]

use rigatoni_destinations::s3::{Compression, S3Config, SerializationFormat};

#[test]
fn test_config_builder_minimal() {
    let config = S3Config::builder()
        .bucket("my-bucket")
        .region("us-east-1")
        .build()
        .unwrap();

    assert_eq!(config.bucket, "my-bucket");
    assert_eq!(config.region, "us-east-1");
    assert_eq!(config.prefix, None);
    assert_eq!(config.format, SerializationFormat::Json);
    assert_eq!(config.compression, Compression::None);
    assert_eq!(config.max_retries, 3);
}

#[test]
fn test_config_builder_full() {
    let config = S3Config::builder()
        .bucket("analytics-bucket")
        .region("eu-west-1")
        .prefix("mongodb/events")
        .format(SerializationFormat::Json)
        .compression(Compression::None)
        .max_retries(5)
        .endpoint_url("http://localhost:4566")
        .force_path_style(true)
        .build()
        .unwrap();

    assert_eq!(config.bucket, "analytics-bucket");
    assert_eq!(config.region, "eu-west-1");
    assert_eq!(config.prefix, Some("mongodb/events".to_string()));
    assert_eq!(config.format, SerializationFormat::Json);
    assert_eq!(config.compression, Compression::None);
    assert_eq!(config.max_retries, 5);
    assert_eq!(
        config.endpoint_url,
        Some("http://localhost:4566".to_string())
    );
}

#[test]
fn test_config_builder_missing_bucket() {
    let result = S3Config::builder().region("us-east-1").build();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("bucket"));
}

#[test]
fn test_config_builder_empty_bucket() {
    let result = S3Config::builder().bucket("").region("us-east-1").build();

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty"));
}

#[test]
fn test_serialization_format_extensions() {
    assert_eq!(SerializationFormat::Json.extension(), "jsonl");

    #[cfg(feature = "csv")]
    assert_eq!(SerializationFormat::Csv.extension(), "csv");

    #[cfg(feature = "parquet")]
    assert_eq!(SerializationFormat::Parquet.extension(), "parquet");

    #[cfg(feature = "avro")]
    assert_eq!(SerializationFormat::Avro.extension(), "avro");
}

#[test]
fn test_compression_extensions() {
    assert_eq!(Compression::None.extension(), "");

    #[cfg(feature = "gzip")]
    assert_eq!(Compression::Gzip.extension(), ".gz");

    #[cfg(feature = "zstandard")]
    assert_eq!(Compression::Zstd.extension(), ".zst");
}
