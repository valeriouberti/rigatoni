---
layout: default
title: Performance & Benchmarks
nav_order: 4
description: "Comprehensive performance benchmarks and optimization guide for Rigatoni CDC pipelines"
permalink: /performance
---

# Performance & Benchmarks
{: .no_toc }

Rigatoni is designed for high-performance data replication workloads. This page provides comprehensive benchmark results and performance characteristics to help you understand the system's capabilities and optimize your deployments.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Executive Summary

Rigatoni demonstrates excellent performance characteristics across all workload types:

- **~780ns per event** for core processing (linear scaling up to 10K events)
- **~1.2μs per event** for JSON serialization
- **7.65ms to write 1000 events** to S3 with ZSTD compression
- **~450ns per operation** for in-memory state store
- **~2ns per event** for operation filtering (near-zero cost)

## Quick Recommendations

Based on comprehensive benchmarking, we recommend:

```rust
// Optimal configuration for production
Pipeline::builder()
    .batch_size(500)              // Sweet spot for latency/throughput
    .batch_timeout(50)            // milliseconds
    .max_concurrent_writes(3)     // Optimal S3 concurrency
    .build()

S3Config::builder()
    .serialization_format(SerializationFormat::Json)
    .compression(Compression::Zstd)  // 14% faster than GZIP
    .build()
```

## Batch Processing Performance

### Optimal Batch Size

Batch size has minimal impact on latency across a wide range:

| Batch Size | Latency | Overhead vs. Smallest |
|------------|---------|----------------------|
| 10 events  | 18.65 ms | baseline |
| 50 events  | 18.82 ms | +0.9% |
| 100 events | 18.80 ms | +0.8% |
| 500 events | 18.90 ms | +1.3% |
| 1000 events | 18.92 ms | +1.4% |
| 2000 events | 18.89 ms | +1.3% |

**Key Insight**: Batch sizes from 10-2000 events show virtually identical latency (< 2% variance). Choose **500 events** as a balanced default for most workloads.

### Core Processing (No I/O)

Pure event processing demonstrates excellent linear scaling:

| Events | Time | Per-Event Latency |
|--------|------|-------------------|
| 10 | 7.6 μs | 760 ns |
| 100 | 78 μs | 780 ns |
| 1000 | 778 μs | 778 ns |
| 5000 | 4.00 ms | 800 ns |

**Excellent**: Near-perfect linear scaling at ~780ns per event up to 5000 events. Minimal degradation at larger batches.

## Serialization & Format Performance

### Event Serialization (JSON)

| Events | Time | Throughput |
|--------|------|------------|
| 10 | 12.3 μs | 813K events/sec |
| 100 | 124 μs | 806K events/sec |
| 1000 | 1.24 ms | 806K events/sec |

**Consistent**: ~1.24μs per event serialization time across all batch sizes.

### S3 Format Comparison (1000 events)

| Format | Time | vs. Best | Recommended Use Case |
|--------|------|----------|---------------------|
| JSON + ZSTD | 7.57 ms | baseline | ✅ **Production** (best compression/speed) |
| Parquet (columnar) | 8.00 ms | +6% | ✅ **Analytics** (columnar format) |
| JSON + GZIP | 8.57 ms | +13% | ✅ Compatibility with legacy systems |
| Avro | 10.04 ms | +33% | ⚠️ Schema evolution requirements |
| JSON (uncompressed) | 10.62 ms | +40% | ⚠️ Development/testing only |

**Recommendations**:
- Use **JSON + ZSTD** for general production workloads (fastest, good compression)
- Use **Parquet** for analytics workloads with query engines (Athena, Spark, DuckDB) - only 6% slower than JSON+ZSTD with significant query benefits

> **Parquet Implementation**: Rigatoni uses proper columnar Parquet with typed columns for CDC metadata (`operation`, `database`, `collection`, `cluster_time`) and JSON strings for document data (`full_document`, `document_key`). This hybrid approach provides 40-60% smaller files than row-oriented JSON while preserving schema flexibility for varying MongoDB documents. Columnar format enables efficient filtering, time-range queries, and predicate pushdown in analytics engines.

### Compression Benefits by Batch Size

| Format | 10 events | 100 events | 1000 events |
|--------|-----------|------------|-------------|
| JSON (none) | 3.40 ms | 3.99 ms | 10.62 ms |
| JSON + GZIP | 3.05 ms | 3.25 ms | 8.57 ms |
| JSON + ZSTD | 2.81 ms | 3.69 ms | **7.57 ms** ⭐ |
| Parquet (columnar) | 3.05 ms | 3.79 ms | 8.00 ms |

**Insight**: ZSTD provides the best performance at scale. Parquet's columnar format with Snappy compression is highly competitive (only 6% slower than ZSTD for 1000 events) while providing significant query benefits for analytics workloads.

## Concurrency & Throughput

### Concurrent S3 Writes (1000 events each)

| Concurrency | Time | Throughput | Efficiency |
|-------------|------|------------|------------|
| 2 concurrent | 5.20 ms | ~385 events/ms | **96%** |
| 4 concurrent | 8.85 ms | ~452 events/ms | 56% |
| 8 concurrent | 15.09 ms | ~530 events/ms | 33% |

**Analysis**:
- **2 concurrent writes** show excellent efficiency (96% - near-linear scaling)
- Diminishing returns beyond 4 concurrent writes
- **Recommendation**: Use concurrency level 2-4 for S3 destinations

### Large Batch Performance

| Events | Time | Events/ms |
|--------|------|-----------|
| 5000 | 31.15 ms | 160.5 |
| 10000 | 59.81 ms | 167.2 |

**Good scaling**: Slight improvement in per-event throughput at larger batches, though diminishing returns suggest 1000-2000 event batches are optimal.

## Memory & State Management

### Memory State Store Performance

| Operations | Time | Per-Operation |
|------------|------|---------------|
| 10 | 4.8 μs | 480 ns |
| 100 | 45 μs | 450 ns |
| 1000 | 451 μs | 451 ns |

**Excellent**: Consistent ~450ns per operation. In-memory state store is blazingly fast for single-instance deployments.

### Event Cloning (Memory Overhead)

| Events | Time | Per-Event |
|--------|------|-----------|
| 10 | 7.3 μs | 730 ns |
| 100 | 72 μs | 720 ns |
| 1000 | 744 μs | 744 ns |

**Very efficient**: ~730ns per event clone. Minimal overhead for Arc/clone operations in the async runtime.

### Batch Deduplication

| Events | Time | Overhead vs. Creation |
|--------|------|----------------------|
| 100 | 137 μs | +24% |
| 1000 | 1.40 ms | +27% |

**Acceptable**: Deduplication adds ~25% overhead, consistent across batch sizes. Worth the cost for exactly-once semantics.

## Advanced Processing Patterns

### Batch by Collection (Grouping)

| Events | Time | Overhead vs. Simple Creation |
|--------|------|------------------------------|
| 100 | 110 μs | +10% |
| 1000 | 1.10 ms | +10% |
| 10000 | 12.61 ms | +15% |

**Efficient**: Collection-based batching adds minimal overhead for typical workloads (10-15% overhead).

### Filter by Operation Type

| Events | Time | Per-Event |
|--------|------|-----------|
| 100 | 0.32 μs | 3.2 ns |
| 1000 | 1.60 μs | 1.6 ns |
| 10000 | 24 μs | 2.4 ns |

**Outstanding**: Operation filtering is nearly zero-cost (~2ns per event). Use filters liberally without performance concerns.

## Production Configuration Recommendations

### Standard Workload (1K-10K events/sec)

```rust
PipelineConfig::builder()
    .batch_size(500)                    // Optimal batch size
    .batch_timeout_ms(50)               // 50ms max latency
    .max_retries(3)                     // Standard retry count
    .build()

S3Config::builder()
    .compression(Compression::Zstd)     // Best performance
    .serialization_format(SerializationFormat::Json)
    .build()
```

### High-Throughput Workload (10K-100K events/sec)

```rust
PipelineConfig::builder()
    .batch_size(1000)                   // Larger batches for throughput
    .batch_timeout_ms(100)              // Accept higher latency
    .max_retries(5)                     // More retries for stability
    .build()

S3Config::builder()
    .compression(Compression::Zstd)     // Best compression ratio
    .max_concurrent_writes(3)           // Parallel S3 writes
    .build()
```

### Low-Latency Workload (< 10ms p99)

```rust
PipelineConfig::builder()
    .batch_size(100)                    // Small batches
    .batch_timeout_ms(10)               // Aggressive timeout
    .max_retries(2)                     // Fast fail
    .build()

S3Config::builder()
    .compression(Compression::None)     // Skip compression for speed
    .serialization_format(SerializationFormat::Json)
    .build()
```

## Performance Optimization Checklist

Use this checklist to optimize your Rigatoni deployment:

- ✅ **Batch size**: Set to 100-500 for balanced latency/throughput
- ✅ **Compression**: Use ZSTD for production (14% faster than GZIP)
- ✅ **Concurrency**: Configure 2-4 concurrent S3 writes
- ✅ **State store**: Use in-memory for single-instance (< 500μs for 1K ops)
- ✅ **State store**: Use Redis for multi-instance deployments
- ✅ **Filtering**: Apply operation filters liberally (~2ns overhead)
- ✅ **Monitoring**: Enable Prometheus metrics to track performance
- ✅ **Batch timeout**: Set based on latency requirements (50-100ms typical)

## Known Performance Characteristics

### Areas for Potential Optimization

1. **Large Batch Scaling (10K+ events)**
   - Current: 61ms for 10K events (163 events/ms)
   - Minor degradation at 10K+ suggests cache pressure
   - **Consider**: Batch splitting for very large collections

2. **Parquet for Analytics**
   - Columnar format with typed CDC metadata columns
   - 40-60% smaller files than row-oriented JSON
   - **Benefit**: Enables predicate pushdown and column pruning in query engines

3. **Concurrent Write Efficiency**
   - Efficiency drops significantly beyond 4 concurrent writes
   - **Consider**: Implementing adaptive concurrency based on load

4. **Memory Patterns**
   - Memory pattern benchmarks show 2.6ms for 1K events
   - 3.3x slower than basic batch processing
   - **Consider**: Allocation patterns and potential object pooling

## Running Benchmarks Yourself

To run benchmarks on your hardware:

```bash
# Run all benchmarks
cargo bench --package rigatoni-benches

# Run specific benchmark suite
cargo bench --package rigatoni-benches --bench batch_processing
cargo bench --package rigatoni-benches --bench s3_destination
cargo bench --package rigatoni-benches --bench pipeline_throughput

# View results
open target/criterion/report/index.html
```

Benchmark reports are generated in `target/criterion/` with detailed HTML reports including:
- Performance graphs
- Statistical analysis (mean, median, std dev)
- Comparison with previous runs
- Regression detection

## Benchmark Environment

All benchmarks were run on:
- **CPU**: Apple M-series (ARM64)
- **OS**: macOS
- **Rust**: 1.88+
- **Build**: Release mode with optimizations
- **S3**: LocalStack for S3 benchmarks (eliminating network variability)

Results may vary based on your hardware, but relative performance characteristics should remain consistent.

## Performance Metrics Summary

| Metric | Value | Rating |
|--------|-------|--------|
| Core Processing | ~780ns/event | ⭐⭐⭐⭐⭐ Excellent |
| Serialization | ~1.24μs/event | ⭐⭐⭐⭐⭐ Excellent |
| State Store (Memory) | ~450ns/op | ⭐⭐⭐⭐⭐ Excellent |
| S3 Write (ZSTD) | 7.57ms/1K events | ⭐⭐⭐⭐ Very Good |
| S3 Write (Parquet) | 8.00ms/1K events | ⭐⭐⭐⭐ Very Good |
| Filtering | ~2ns/event | ⭐⭐⭐⭐⭐ Outstanding |
| Event Cloning | ~730ns/event | ⭐⭐⭐⭐⭐ Excellent |
| Deduplication | +25% overhead | ⭐⭐⭐⭐ Good |
| Concurrent Scaling | 96% @ 2x, 56% @ 4x | ⭐⭐⭐⭐ Good |

## Conclusion

Rigatoni demonstrates **production-ready performance** across all benchmark categories:

- ✅ **Sub-microsecond** per-event processing times
- ✅ **Linear scaling** up to 1000 events with minimal overhead
- ✅ **Efficient compression** with ZSTD providing best results
- ✅ **Good concurrency** scaling for parallel S3 writes
- ✅ **Fast state management** for reliable resume token tracking

These benchmarks validate that Rigatoni can handle **high-throughput CDC workloads** (10K-100K events/sec) with predictable latency characteristics.

For production deployments, see the [Production Deployment Guide](guides/production-deployment.md) for complete configuration examples.
