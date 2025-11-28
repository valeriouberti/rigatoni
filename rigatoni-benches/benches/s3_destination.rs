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

//! S3 Destination Benchmarks using LocalStack
//!
//! These benchmarks measure the performance of the S3 destination
//! with various configurations (formats, compression, batch sizes).
//!
//! # Prerequisites
//!
//! Start LocalStack with S3 service:
//! ```bash
//! docker-compose up -d localstack
//! ```
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo bench --package rigatoni-benches --bench s3_destination
//! ```

use bson::doc;
use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rigatoni_core::destination::Destination;
use rigatoni_core::event::{ChangeEvent, Namespace, OperationType};
use rigatoni_destinations::s3::config::{Compression, SerializationFormat};
use rigatoni_destinations::s3::{AwsCredentials, S3Config, S3Destination};
use std::time::Duration;

/// Helper function to create sample change events
fn create_sample_events(count: usize) -> Vec<ChangeEvent> {
    (0..count)
        .map(|i| ChangeEvent {
            resume_token: doc! { "_data": format!("token_{}", i) },
            operation: OperationType::Insert,
            namespace: Namespace::new("bench_db", "bench_collection"),
            full_document: Some(doc! {
                "_id": i as i64,
                "user_id": format!("user_{}", i),
                "email": format!("user_{}@example.com", i),
                "name": format!("User {}", i),
                "age": (20 + (i % 50)) as i32,
                "created_at": Utc::now().to_rfc3339(),
                "metadata": {
                    "source": "benchmark",
                    "version": "1.0.0",
                    "tags": ["benchmark", "test", "data"],
                },
                "balance": (i as f64) * 123.45,
                "active": i % 2 == 0,
            }),
            document_key: Some(doc! { "_id": i as i64 }),
            update_description: None,
            cluster_time: Utc::now(),
        })
        .collect()
}

/// Create S3 configuration for LocalStack
async fn create_localstack_config(
    format: SerializationFormat,
    compression: Compression,
) -> S3Config {
    S3Config::builder()
        .bucket("rigatoni-bench-bucket")
        .region("us-east-1")
        .prefix("benchmarks/s3_destination")
        .format(format)
        .compression(compression)
        .endpoint_url("http://localhost:4566") // LocalStack endpoint
        .force_path_style(true)
        .credentials(AwsCredentials::new("test", "test"))
        .build()
        .expect("Failed to create S3 config")
}

/// Setup LocalStack S3 bucket
async fn setup_localstack_bucket() {
    use aws_config::BehaviorVersion;
    use aws_sdk_s3::config::{Credentials, Region};
    use aws_sdk_s3::Client;

    let creds = Credentials::new("test", "test", None, None, "localstack");
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .credentials_provider(creds)
        .endpoint_url("http://localhost:4566")
        .load()
        .await;

    let s3_config = aws_sdk_s3::config::Builder::from(&config)
        .force_path_style(true)
        .build();

    let client = Client::from_conf(s3_config);

    // Create bucket if it doesn't exist
    let _ = client
        .create_bucket()
        .bucket("rigatoni-bench-bucket")
        .send()
        .await;
}

/// Benchmark: Write batch to S3 with JSON format (no compression)
fn bench_json_no_compression(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Setup LocalStack bucket once
    runtime.block_on(setup_localstack_bucket());

    let mut group = c.benchmark_group("s3_json_no_compression");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let config =
                    create_localstack_config(SerializationFormat::Json, Compression::None).await;
                let mut destination = S3Destination::new(config)
                    .await
                    .expect("Failed to create S3 destination");

                let events = create_sample_events(size);

                destination
                    .write_batch(black_box(&events))
                    .await
                    .expect("Failed to write batch");

                destination.flush().await.expect("Failed to flush");
                destination.close().await.expect("Failed to close");
            });
        });
    }

    group.finish();
}

/// Benchmark: Write batch to S3 with JSON format + Gzip compression
fn bench_json_gzip(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(setup_localstack_bucket());

    let mut group = c.benchmark_group("s3_json_gzip");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let config =
                    create_localstack_config(SerializationFormat::Json, Compression::Gzip).await;
                let mut destination = S3Destination::new(config)
                    .await
                    .expect("Failed to create S3 destination");

                let events = create_sample_events(size);

                destination
                    .write_batch(black_box(&events))
                    .await
                    .expect("Failed to write batch");

                destination.flush().await.expect("Failed to flush");
                destination.close().await.expect("Failed to close");
            });
        });
    }

    group.finish();
}

/// Benchmark: Write batch to S3 with JSON format + Zstandard compression
fn bench_json_zstd(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(setup_localstack_bucket());

    let mut group = c.benchmark_group("s3_json_zstd");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let config =
                    create_localstack_config(SerializationFormat::Json, Compression::Zstd).await;
                let mut destination = S3Destination::new(config)
                    .await
                    .expect("Failed to create S3 destination");

                let events = create_sample_events(size);

                destination
                    .write_batch(black_box(&events))
                    .await
                    .expect("Failed to write batch");

                destination.flush().await.expect("Failed to flush");
                destination.close().await.expect("Failed to close");
            });
        });
    }

    group.finish();
}

/// Benchmark: Write batch to S3 with Parquet format
fn bench_parquet_no_compression(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(setup_localstack_bucket());

    let mut group = c.benchmark_group("s3_parquet_no_compression");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let config =
                    create_localstack_config(SerializationFormat::Parquet, Compression::None).await;
                let mut destination = S3Destination::new(config)
                    .await
                    .expect("Failed to create S3 destination");

                let events = create_sample_events(size);

                destination
                    .write_batch(black_box(&events))
                    .await
                    .expect("Failed to write batch");

                destination.flush().await.expect("Failed to flush");
                destination.close().await.expect("Failed to close");
            });
        });
    }

    group.finish();
}

/// Benchmark: Comparison of all formats and compression combinations
fn bench_format_comparison(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(setup_localstack_bucket());

    let mut group = c.benchmark_group("format_comparison");
    group.sample_size(10); // Smaller sample size for larger batches

    let configs = vec![
        ("json", SerializationFormat::Json, Compression::None),
        ("json_gzip", SerializationFormat::Json, Compression::Gzip),
        ("json_zstd", SerializationFormat::Json, Compression::Zstd),
        ("parquet", SerializationFormat::Parquet, Compression::None),
        ("avro", SerializationFormat::Avro, Compression::None),
    ];

    let batch_size = 1000;
    group.throughput(Throughput::Elements(batch_size as u64));

    for (name, format, compression) in configs {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(format, compression),
            |b, &(format, compression)| {
                b.to_async(&runtime).iter(|| async {
                    let config = create_localstack_config(format, compression).await;
                    let mut destination = S3Destination::new(config)
                        .await
                        .expect("Failed to create S3 destination");

                    let events = create_sample_events(batch_size);

                    destination
                        .write_batch(black_box(&events))
                        .await
                        .expect("Failed to write batch");

                    destination.flush().await.expect("Failed to flush");
                    destination.close().await.expect("Failed to close");
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Large batch performance
fn bench_large_batches(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(setup_localstack_bucket());

    let mut group = c.benchmark_group("large_batches");
    group.sample_size(10); // Smaller sample size for very large batches
    group.measurement_time(Duration::from_secs(30)); // Longer measurement time

    for size in [5000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&runtime).iter(|| async {
                let config =
                    create_localstack_config(SerializationFormat::Json, Compression::Gzip).await;
                let mut destination = S3Destination::new(config)
                    .await
                    .expect("Failed to create S3 destination");

                let events = create_sample_events(size);

                destination
                    .write_batch(black_box(&events))
                    .await
                    .expect("Failed to write batch");

                destination.flush().await.expect("Failed to flush");
                destination.close().await.expect("Failed to close");
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_json_no_compression,
    bench_json_gzip,
    bench_json_zstd,
    bench_parquet_no_compression,
    bench_format_comparison,
    bench_large_batches,
);

criterion_main!(benches);
