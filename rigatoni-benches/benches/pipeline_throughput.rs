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

//! Pipeline Throughput Benchmarks
//!
//! These benchmarks measure the end-to-end throughput of the Rigatoni pipeline
//! with various configurations, destinations, and state stores.
//!
//! # Prerequisites
//!
//! Start required services:
//! ```bash
//! docker-compose up -d localstack
//! ```
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo bench --package rigatoni-benches --bench pipeline_throughput
//! ```

use async_trait::async_trait;
use bson::doc;
use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rigatoni_core::destination::{Destination, DestinationError, DestinationMetadata};
use rigatoni_core::event::{ChangeEvent, Namespace, OperationType};
use rigatoni_core::state::StateStore;
use rigatoni_destinations::s3::config::{Compression, SerializationFormat};
use rigatoni_destinations::s3::{AwsCredentials, S3Config, S3Destination};
use rigatoni_stores::memory::MemoryStore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Helper function to create sample change events
fn create_sample_events(count: usize, collection: &str) -> Vec<ChangeEvent> {
    (0..count)
        .map(|i| ChangeEvent {
            resume_token: doc! { "_data": format!("token_{}_{}", collection, i) },
            operation: OperationType::Insert,
            namespace: Namespace::new("bench_db", collection),
            full_document: Some(doc! {
                "_id": i as i64,
                "data": format!("event_{}", i),
                "timestamp": Utc::now().to_rfc3339(),
                "value": i as i32,
            }),
            document_key: Some(doc! { "_id": i as i64 }),
            update_description: None,
            cluster_time: Utc::now(),
        })
        .collect()
}

/// Mock destination for measuring pure pipeline overhead
#[derive(Clone)]
struct MockDestination {
    events_written: Arc<AtomicUsize>,
    write_delay: Duration,
}

impl MockDestination {
    fn new(write_delay: Duration) -> Self {
        Self {
            events_written: Arc::new(AtomicUsize::new(0)),
            write_delay,
        }
    }
}

#[async_trait]
impl Destination for MockDestination {
    async fn write_batch(&mut self, events: &[ChangeEvent]) -> Result<(), DestinationError> {
        if self.write_delay > Duration::ZERO {
            tokio::time::sleep(self.write_delay).await;
        }
        self.events_written
            .fetch_add(events.len(), Ordering::SeqCst);
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), DestinationError> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), DestinationError> {
        Ok(())
    }

    fn supports_transactions(&self) -> bool {
        false
    }

    fn metadata(&self) -> DestinationMetadata {
        DestinationMetadata {
            name: "MockDestination".to_string(),
            destination_type: "mock".to_string(),
            supports_transactions: false,
            max_batch_size: None,
            supports_concurrent_writes: true,
            properties: Default::default(),
        }
    }
}

/// Benchmark: Memory state store performance
fn bench_memory_state_store(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_state_store");

    for count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.to_async(&runtime).iter(|| async {
                let store = MemoryStore::new();
                let collection = "bench_collection";

                for i in 0..count {
                    let token = doc! { "_data": format!("token_{}", i) };
                    store
                        .save_resume_token(collection, &token)
                        .await
                        .expect("Failed to save token");
                }

                for _ in 0..count {
                    let _ = black_box(store.get_resume_token(collection).await);
                }
            });
        });
    }

    group.finish();
}

/// Benchmark: Batch processing with mock destination (no I/O)
fn bench_batch_processing_no_io(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("batch_processing_no_io");

    for batch_size in [10, 100, 1000, 5000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&runtime).iter(|| async {
                    let mut destination = MockDestination::new(Duration::ZERO);
                    let events = create_sample_events(batch_size, "bench_collection");

                    destination
                        .write_batch(black_box(&events))
                        .await
                        .expect("Failed to write batch");

                    destination.flush().await.expect("Failed to flush");
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Batch processing with simulated write latency
fn bench_batch_processing_with_latency(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("batch_processing_with_latency");
    group.sample_size(10);

    let latencies = vec![
        ("1ms", Duration::from_millis(1)),
        ("10ms", Duration::from_millis(10)),
        ("50ms", Duration::from_millis(50)),
    ];

    for (name, latency) in latencies {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &latency,
            |b, &latency| {
                b.to_async(&runtime).iter(|| async {
                    let mut destination = MockDestination::new(latency);
                    let events = create_sample_events(100, "bench_collection");

                    destination
                        .write_batch(black_box(&events))
                        .await
                        .expect("Failed to write batch");

                    destination.flush().await.expect("Failed to flush");
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: S3 destination with LocalStack (real I/O)
fn bench_s3_destination_throughput(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Setup LocalStack bucket
    runtime.block_on(async {
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
        let _ = client
            .create_bucket()
            .bucket("rigatoni-bench-bucket")
            .send()
            .await;
    });

    let mut group = c.benchmark_group("s3_throughput");
    group.sample_size(10);

    for batch_size in [100, 1000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&runtime).iter(|| async {
                    let config = S3Config::builder()
                        .bucket("rigatoni-bench-bucket")
                        .region("us-east-1")
                        .prefix("benchmarks/throughput")
                        .format(SerializationFormat::Json)
                        .compression(Compression::Gzip)
                        .endpoint_url("http://localhost:4566")
                        .force_path_style(true)
                        .credentials(AwsCredentials::new("test", "test"))
                        .build()
                        .expect("Failed to create S3 config");

                    let mut destination = S3Destination::new(config)
                        .await
                        .expect("Failed to create S3 destination");

                    let events = create_sample_events(batch_size, "bench_collection");

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

/// Benchmark: Concurrent writes to S3
fn bench_concurrent_s3_writes(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Setup LocalStack bucket
    runtime.block_on(async {
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
        let _ = client
            .create_bucket()
            .bucket("rigatoni-bench-bucket")
            .send()
            .await;
    });

    let mut group = c.benchmark_group("concurrent_s3_writes");
    group.sample_size(10);

    for num_tasks in [2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_tasks),
            num_tasks,
            |b, &num_tasks| {
                b.to_async(&runtime).iter(|| async {
                    let mut handles = Vec::new();

                    for i in 0..num_tasks {
                        let handle = tokio::spawn(async move {
                            let config = S3Config::builder()
                                .bucket("rigatoni-bench-bucket")
                                .region("us-east-1")
                                .prefix(format!("benchmarks/concurrent/task_{}", i))
                                .format(SerializationFormat::Json)
                                .compression(Compression::Gzip)
                                .endpoint_url("http://localhost:4566")
                                .force_path_style(true)
                                .credentials(AwsCredentials::new("test", "test"))
                                .build()
                                .expect("Failed to create S3 config");

                            let mut destination = S3Destination::new(config)
                                .await
                                .expect("Failed to create S3 destination");

                            let events = create_sample_events(100, &format!("collection_{}", i));

                            destination
                                .write_batch(&events)
                                .await
                                .expect("Failed to write batch");

                            destination.flush().await.expect("Failed to flush");
                            destination.close().await.expect("Failed to close");
                        });

                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.expect("Task failed");
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Event serialization overhead
fn bench_event_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_serialization");

    for count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let events = create_sample_events(count, "bench_collection");
                for event in events {
                    let _ = black_box(serde_json::to_string(&event).unwrap());
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_memory_state_store,
    bench_batch_processing_no_io,
    bench_batch_processing_with_latency,
    bench_s3_destination_throughput,
    bench_concurrent_s3_writes,
    bench_event_serialization,
);

criterion_main!(benches);
