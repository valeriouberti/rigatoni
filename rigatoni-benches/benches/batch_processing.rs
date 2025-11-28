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

//! Batch Processing Benchmarks
//!
//! These benchmarks measure the performance of various batch processing scenarios:
//! - Different batch sizes
//! - Event aggregation
//! - Memory usage patterns
//! - Batching strategies
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo bench --package rigatoni-benches --bench batch_processing
//! ```

use bson::doc;
use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rigatoni_core::event::{ChangeEvent, Namespace, OperationType};
use std::collections::HashMap;
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
                "user_id": format!("user_{}", i % 1000),
                "action": if i % 3 == 0 { "create" } else if i % 3 == 1 { "update" } else { "delete" },
                "timestamp": Utc::now().to_rfc3339(),
                "value": i as i32,
                "metadata": {
                    "source": "benchmark",
                    "version": "1.0.0",
                },
            }),
            document_key: Some(doc! { "_id": i as i64 }),
            update_description: None,
            cluster_time: Utc::now(),
        })
        .collect()
}

/// Create events with varying sizes
fn create_variable_size_events(count: usize) -> Vec<ChangeEvent> {
    (0..count)
        .map(|i| {
            // Create varying payload sizes
            let payload_size = match i % 4 {
                0 => 100,  // Small
                1 => 500,  // Medium
                2 => 1000, // Large
                _ => 50,   // Tiny
            };

            let large_data = "x".repeat(payload_size);

            ChangeEvent {
                resume_token: doc! { "_data": format!("token_{}", i) },
                operation: OperationType::Insert,
                namespace: Namespace::new("bench_db", "variable_collection"),
                full_document: Some(doc! {
                    "_id": i as i64,
                    "data": large_data,
                    "timestamp": Utc::now().to_rfc3339(),
                }),
                document_key: Some(doc! { "_id": i as i64 }),
                update_description: None,
                cluster_time: Utc::now(),
            }
        })
        .collect()
}

/// Benchmark: Creating batches of events
fn bench_batch_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_creation");

    for size in [10, 100, 1000, 5000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let _events = black_box(create_sample_events(size, "bench_collection"));
            });
        });
    }

    group.finish();
}

/// Benchmark: Batching events by collection
fn bench_batch_by_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_by_collection");

    for total_events in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*total_events as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(total_events),
            total_events,
            |b, &total_events| {
                b.iter(|| {
                    // Create events for multiple collections
                    let collections = ["users", "orders", "products", "logs"];
                    let mut all_events = Vec::new();

                    for collection in collections.iter().cycle().take(total_events) {
                        let events = create_sample_events(1, collection);
                        all_events.extend(events);
                    }

                    // Group by collection
                    let mut batches: HashMap<String, Vec<ChangeEvent>> = HashMap::new();
                    for event in all_events {
                        batches
                            .entry(event.namespace.collection.clone())
                            .or_default()
                            .push(event);
                    }

                    black_box(batches);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Filtering events by operation type
fn bench_filter_by_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_by_operation");

    for count in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let events = create_sample_events(count, "bench_collection");

            b.iter(|| {
                let inserts: Vec<_> = events
                    .iter()
                    .filter(|e| e.operation == OperationType::Insert)
                    .collect();
                let updates: Vec<_> = events
                    .iter()
                    .filter(|e| e.operation == OperationType::Update)
                    .collect();
                let deletes: Vec<_> = events
                    .iter()
                    .filter(|e| e.operation == OperationType::Delete)
                    .collect();

                black_box((inserts, updates, deletes));
            });
        });
    }

    group.finish();
}

/// Benchmark: Batch size optimization
fn bench_optimal_batch_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimal_batch_size");
    group.sample_size(20);

    let total_events = 10000;
    let batch_sizes = [10, 50, 100, 500, 1000, 2000];

    for batch_size in batch_sizes.iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let events = create_sample_events(total_events, "bench_collection");

                    let batches: Vec<Vec<ChangeEvent>> = events
                        .chunks(batch_size)
                        .map(|chunk| chunk.to_vec())
                        .collect();

                    // Simulate processing each batch
                    for batch in batches {
                        black_box(&batch);
                        // Simulate minimal processing
                        let _count = batch.len();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Variable-size event batching
fn bench_variable_size_batching(c: &mut Criterion) {
    let mut group = c.benchmark_group("variable_size_batching");

    for count in [100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let events = create_variable_size_events(count);

                // Batch by approximate byte size (target: 1MB batches)
                let target_batch_bytes = 1024 * 1024; // 1MB
                let mut batches = Vec::new();
                let mut current_batch = Vec::new();
                let mut current_size = 0;

                for event in events {
                    let event_size = serde_json::to_string(&event).map(|s| s.len()).unwrap_or(0);

                    if current_size + event_size > target_batch_bytes && !current_batch.is_empty() {
                        batches.push(std::mem::take(&mut current_batch));
                        current_size = 0;
                    }

                    current_batch.push(event);
                    current_size += event_size;
                }

                if !current_batch.is_empty() {
                    batches.push(current_batch);
                }

                black_box(batches);
            });
        });
    }

    group.finish();
}

/// Benchmark: Event cloning overhead
fn bench_event_cloning(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_cloning");

    for count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let events = create_sample_events(count, "bench_collection");

            b.iter(|| {
                let cloned: Vec<ChangeEvent> = events.to_vec();
                black_box(cloned);
            });
        });
    }

    group.finish();
}

/// Benchmark: Time-based batching simulation
fn bench_time_based_batching(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("time_based_batching");
    group.sample_size(10);

    let batch_intervals = [
        ("10ms", Duration::from_millis(10)),
        ("50ms", Duration::from_millis(50)),
        ("100ms", Duration::from_millis(100)),
    ];

    for (name, interval) in batch_intervals.iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            interval,
            |b, interval| {
                b.to_async(&runtime).iter(|| async {
                    let events = create_sample_events(1000, "bench_collection");
                    let mut batches = Vec::new();
                    let mut current_batch = Vec::new();

                    for (i, event) in events.into_iter().enumerate() {
                        current_batch.push(event);

                        // Simulate time-based flushing
                        if (i + 1) % 10 == 0 {
                            tokio::time::sleep(*interval).await;
                            batches.push(std::mem::take(&mut current_batch));
                        }
                    }

                    if !current_batch.is_empty() {
                        batches.push(current_batch);
                    }

                    black_box(batches);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Batch deduplication
fn bench_batch_deduplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_deduplication");

    for count in [100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut events = create_sample_events(count, "bench_collection");

                // Add duplicates (same _id)
                let duplicates = events[0..count / 10].to_vec();
                events.extend(duplicates);

                // Deduplicate by document key (using string representation)
                let mut seen = std::collections::HashSet::new();
                let deduplicated: Vec<_> = events
                    .into_iter()
                    .filter(|e| {
                        if let Some(doc_key) = &e.document_key {
                            // Use the string representation as key since Document doesn't implement Hash
                            let key_str = format!("{:?}", doc_key);
                            seen.insert(key_str)
                        } else {
                            true
                        }
                    })
                    .collect();

                black_box(deduplicated);
            });
        });
    }

    group.finish();
}

/// Benchmark: Memory usage of large batches
fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");
    group.sample_size(10);

    for size in [1000, 5000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let events = create_sample_events(size, "bench_collection");

                // Simulate holding batches in memory
                let batch1 = events.clone();
                let batch2 = events.clone();
                let batch3 = events;

                black_box((batch1, batch2, batch3));
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_batch_creation,
    bench_batch_by_collection,
    bench_filter_by_operation,
    bench_optimal_batch_size,
    bench_variable_size_batching,
    bench_event_cloning,
    bench_time_based_batching,
    bench_batch_deduplication,
    bench_memory_patterns,
);

criterion_main!(benches);
