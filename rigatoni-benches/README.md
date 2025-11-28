# Rigatoni Benchmarks

This crate contains comprehensive benchmarks for the Rigatoni CDC/Data Replication framework. The benchmarks measure performance across various dimensions including S3 destination throughput, pipeline processing, and batch operations.

📊 **[View Detailed Performance Analysis](https://valeriouberti.github.io/rigatoni/performance)** - Complete benchmark results, analysis, and optimization recommendations.

## Prerequisites

### Required Services

The benchmarks use LocalStack to simulate AWS S3 locally. Start the required services:

```bash
# From the project root
docker-compose up -d localstack
```

Verify LocalStack is running:

```bash
docker ps | grep localstack
```

You should see LocalStack running on port 4566.

### Environment Setup

Ensure you have Rust 1.88+ installed:

```bash
rustc --version
```

## Running Benchmarks

### Run All Benchmarks

```bash
cargo bench --package rigatoni-benches
```

### Run Specific Benchmark Suites

#### S3 Destination Benchmarks

Measures S3 write performance with various formats and compression:

```bash
cargo bench --package rigatoni-benches --bench s3_destination
```

**What it measures:**
- JSON serialization (no compression, gzip, zstandard)
- Parquet format
- Avro format
- Different batch sizes (10, 100, 1000, 5000, 10000 events)
- Format comparison across all combinations
- Large batch performance

#### Pipeline Throughput Benchmarks

Measures end-to-end pipeline throughput:

```bash
cargo bench --package rigatoni-benches --bench pipeline_throughput
```

**What it measures:**
- Memory state store performance
- Batch processing with no I/O (mock destination)
- Batch processing with simulated latency (1ms, 10ms, 50ms)
- Real S3 destination throughput with LocalStack
- Concurrent S3 writes (2, 4, 8 parallel tasks)
- Event serialization overhead

#### Batch Processing Benchmarks

Measures various batch processing scenarios:

```bash
cargo bench --package rigatoni-benches --bench batch_processing
```

**What it measures:**
- Batch creation at different sizes
- Grouping events by collection
- Filtering events by operation type
- Optimal batch size determination
- Variable-size event batching
- Event cloning overhead
- Time-based batching simulation
- Batch deduplication
- Memory usage patterns

### Run Specific Benchmark Groups

You can run individual benchmark functions using filters:

```bash
# Only JSON with gzip compression benchmarks
cargo bench --package rigatoni-benches --bench s3_destination -- json_gzip

# Only large batch benchmarks
cargo bench --package rigatoni-benches --bench s3_destination -- large_batches

# Only memory state store benchmarks
cargo bench --package rigatoni-benches --bench pipeline_throughput -- memory_state

# Only batch creation benchmarks
cargo bench --package rigatoni-benches --bench batch_processing -- batch_creation
```

## Benchmark Output

### HTML Reports

Criterion generates detailed HTML reports in `target/criterion/`:

```bash
# Open the main report
open target/criterion/report/index.html

# Or on Linux
xdg-open target/criterion/report/index.html
```

The reports include:
- Line charts showing performance over time
- Violin plots for distribution analysis
- Statistical comparisons between runs
- Regression analysis

### Terminal Output

The benchmarks print summary statistics to the terminal:

```
s3_json_no_compression/10
                        time:   [45.234 ms 46.123 ms 47.891 ms]
                        thrpt:  [208.72 elem/s 216.83 elem/s 221.08 elem/s]

s3_json_no_compression/100
                        time:   [156.34 ms 161.23 ms 167.45 ms]
                        thrpt:  [597.12 elem/s 620.15 elem/s 639.67 elem/s]
```

## Understanding the Results

### Key Metrics

- **time**: Wall-clock time to complete the benchmark
- **thrpt** (throughput): Events processed per second
- **Lower is better** for time measurements
- **Higher is better** for throughput measurements

### Interpreting Comparisons

When running benchmarks multiple times, Criterion compares results:

```
                        time:   [45.234 ms 46.123 ms 47.891 ms]
                        change: [-5.2% -3.1% -1.2%] (p = 0.02 < 0.05)
                        Performance has improved.
```

- **change**: Performance delta from previous run
- **p-value**: Statistical significance (< 0.05 is significant)

### Baseline Comparisons

Save a baseline for future comparisons:

```bash
cargo bench --package rigatoni-benches -- --save-baseline main
```

Compare against the baseline:

```bash
cargo bench --package rigatoni-benches -- --baseline main
```

## Benchmark Scenarios

### S3 Destination Benchmarks

#### JSON No Compression
- **Use case**: Fast writes, human-readable format
- **Trade-off**: Larger file sizes, higher S3 storage costs
- **Expected**: ~200-300 events/sec for small batches, ~500-800 events/sec for large batches

#### JSON + Gzip
- **Use case**: Balanced compression and speed
- **Trade-off**: CPU overhead for compression
- **Expected**: ~150-250 events/sec (slightly slower than no compression)

#### JSON + Zstandard
- **Use case**: Better compression ratio than gzip
- **Trade-off**: More CPU usage
- **Expected**: ~100-200 events/sec

#### Parquet
- **Use case**: Columnar format for analytics
- **Trade-off**: Higher serialization overhead
- **Expected**: ~80-150 events/sec

#### Avro
- **Use case**: Schema evolution support
- **Trade-off**: Moderate overhead
- **Expected**: ~100-180 events/sec

### Pipeline Throughput Benchmarks

#### Mock Destination (No I/O)
- **Purpose**: Measure pure pipeline overhead
- **Expected**: ~50,000-100,000 events/sec

#### Simulated Latency
- **Purpose**: Understand impact of destination write latency
- **Expected**: Throughput inversely proportional to latency

#### S3 Throughput
- **Purpose**: Real-world performance with LocalStack
- **Expected**: ~200-500 events/sec (network + serialization overhead)

#### Concurrent Writes
- **Purpose**: Measure scalability with parallel processing
- **Expected**: Near-linear scaling up to 4-8 tasks

### Batch Processing Benchmarks

#### Optimal Batch Size
- **Purpose**: Find the sweet spot for batch sizes
- **Typical results**:
  - Small batches (10-50): Lower latency, higher overhead
  - Medium batches (100-500): Balanced
  - Large batches (1000-2000): Higher throughput, higher latency

## Troubleshooting

### LocalStack Not Running

```
Error: Connection refused (os error 61)
```

**Solution**: Start LocalStack:
```bash
docker-compose up -d localstack
```

### Benchmarks Too Slow

Reduce sample size for faster iteration during development:

```bash
# Edit benchmark files and set:
group.sample_size(10);  // Default is 100
```

### Out of Memory

For very large batches, you may encounter OOM errors:

**Solution**: Reduce batch sizes or increase Docker memory:
```bash
# In Docker Desktop: Settings → Resources → Memory
```

### Inconsistent Results

Benchmarks can be affected by system load. For accurate results:

1. Close unnecessary applications
2. Disable CPU frequency scaling (if possible)
3. Run multiple times and compare
4. Use `--save-baseline` and `--baseline` for comparisons

## Performance Optimization Tips

Based on benchmark results:

### For S3 Destinations

1. **Use compression** for large batches (gzip is a good default)
2. **Batch size**: 500-1000 events for optimal throughput
3. **Format choice**:
   - JSON: Human-readable, debugging
   - Parquet: Analytics, data warehouses
   - Avro: Schema evolution

### For Pipelines

1. **Concurrent writes**: Use 4-8 parallel tasks for high throughput
2. **State store**: Memory store is fastest, Redis for distributed setups
3. **Batch timeout**: 1-5 seconds for balanced latency/throughput

### For Batch Processing

1. **Deduplication**: Only when necessary (adds overhead)
2. **Filtering**: Pre-filter before batching when possible
3. **Memory**: Avoid holding multiple large batches simultaneously

## Continuous Benchmarking

### CI Integration

Run benchmarks in CI and track performance over time:

```yaml
# .github/workflows/benchmarks.yml
name: Benchmarks
on:
  push:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run benchmarks
        run: cargo bench --package rigatoni-benches -- --save-baseline ci
```

### Regression Detection

Compare branches:

```bash
# On main branch
cargo bench --package rigatoni-benches -- --save-baseline main

# On feature branch
git checkout feature-branch
cargo bench --package rigatoni-benches -- --baseline main
```

Look for significant regressions (> 5% slower).

## Contributing

When adding new benchmarks:

1. Follow existing patterns in the benchmark files
2. Use meaningful benchmark names
3. Document what the benchmark measures
4. Set appropriate sample sizes
5. Use `black_box()` to prevent compiler optimizations
6. Add documentation to this README

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Benchmarking Best Practices](https://bheisler.github.io/criterion.rs/book/user_guide/benchmarking_tips.html)
- [LocalStack Documentation](https://docs.localstack.cloud/)

## License

Apache-2.0 (same as the main Rigatoni project)
