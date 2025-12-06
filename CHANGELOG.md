# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Proper Columnar Parquet Serialization** - Rewrote Parquet serialization to use proper Arrow columnar format
  - CDC metadata (`operation`, `database`, `collection`, `cluster_time`) stored as typed columns
  - Document data (`full_document`, `document_key`) stored as JSON strings for schema flexibility
  - Hybrid approach enables efficient filtering and predicate pushdown in query engines (Athena, Spark, DuckDB)
  - 40-60% smaller file sizes compared to row-oriented JSON storage
  - Pre-allocated Arrow builders for optimal memory performance
  - Snappy compression enabled by default with dictionary encoding
  - Page-level statistics for predicate pushdown optimization

### Changed

- **Performance Documentation** - Updated benchmark data with latest results
  - Parquet now benchmarks at 8.00ms for 1000 events (only 6% slower than JSON+ZSTD)
  - Updated all benchmark tables with fresh data
  - Added Parquet to performance metrics summary
- **S3 Configuration Guide** - Added detailed Parquet format documentation
  - Explains hybrid columnar approach and its benefits
  - Documents query optimization capabilities

### Fixed

- **Security Advisory Ignore** - Added `RUSTSEC-2025-0134` to `deny.toml` ignore list
  - `rustls-pemfile` is unmaintained but is a transitive dependency from AWS SDK and testcontainers
  - No safe upgrade available; waiting for upstream dependencies to migrate to `rustls-pki-types`

---

## [0.1.4] - 2025-11-28

### Added

- **Comprehensive Benchmarks** - Added `rigatoni-benches` package with extensive performance benchmarks
  - Batch processing benchmarks (optimal batch size, creation, serialization)
  - S3 destination benchmarks (format comparison, compression, concurrent writes)
  - Pipeline throughput benchmarks (state store performance, latency simulation)
  - Benchmark results available at: https://valeriouberti.github.io/rigatoni/performance
- **Performance Documentation** - New comprehensive performance guide at `docs/performance.md`
  - Detailed benchmark analysis and results
  - Production configuration recommendations
  - Performance optimization checklist
  - Troubleshooting and tuning guide
- **Benchmark Workflow** - GitHub Actions workflow for automated benchmarking
  - Manual trigger only (workflow_dispatch)
  - LocalStack integration for S3 benchmarks
  - Criterion HTML reports as artifacts
  - Results publishing to GitHub Pages
- **Performance Highlights** - Added performance metrics section to README
  - Prominently displays key performance characteristics
  - ~780ns per event processing
  - 10K-100K events/sec throughput

### Changed

- **Documentation** - Enhanced main documentation with performance section
  - Added "Performance & Benchmarks" to documentation navigation
  - Performance badge in README linking to benchmark results
  - Updated rigatoni-benches README with documentation link

### Fixed

- **Benchmark Code Quality** - Fixed Clippy warnings in benchmark code
  - Removed unused enumerate in batch processing
  - Replaced `iter().cloned().collect()` with `.to_vec()`
  - Removed unused methods in mock destinations

---

## [0.1.3] - 2025-01-22

### Fixed

- **docs.rs Build** - Moved examples to separate unpublished workspace crate to eliminate circular dev-dependency issues that prevented docs.rs from building documentation
- **MSRV Declaration** - Updated `rust-version` to 1.88 to match AWS SDK S3 requirements (was incorrectly set to 1.85)

### Changed

- **Minimum Rust Version** - Updated MSRV from 1.85 to 1.88 due to AWS SDK S3 dependency requirements
- **Examples Architecture** - Moved integration examples to `rigatoni-examples/` workspace crate (unpublished) for cleaner dependency management
  - Examples can now use all rigatoni crates together without circular dependencies
  - Run examples with: `cargo run --example <name> -p rigatoni-examples`
  - Published crates no longer include examples, reducing package size

---

## [0.1.2] - 2025-01-22

### Fixed

- **GitHub Release Permissions** - Added `permissions: contents: write` to release workflow to allow creating GitHub releases
- **docs.rs Build** - Added version requirements to dev-dependencies in rigatoni-core for proper docs.rs documentation builds

---

## [0.1.1] - 2025-01-22

### Fixed

- **Documentation Configuration** - Added `[package.metadata.docs.rs]` configuration to all crates for proper docs.rs builds
- **Documentation URL** - Fixed workspace documentation URL to point to `rigatoni-core` instead of non-existent `rigatoni` crate
- **Dependency Versions** - Updated all inter-crate dependencies to use explicit version numbers (0.1.1) for crates.io compatibility

### Changed

- All crates now build documentation with all features enabled on docs.rs
- Updated installation examples in README files to reference version 0.1.1

---

## [0.1.0] - 2025-11-22

### Added

- **MongoDB CDC Source** - Real-time change stream integration with resume token support
- **S3 Destination** - AWS S3 integration with multiple formats (JSON, CSV, Parquet, Avro) and compression (Gzip, Zstandard)
- **Pipeline Orchestration** - Multi-worker architecture with batching, retry logic, and graceful shutdown
- **State Stores** - In-memory and Redis-based stores for resume token persistence
- **Metrics** - Comprehensive Prometheus metrics for monitoring throughput, latency, and errors
- **Docker Compose** - Complete local development environment with MongoDB, Redis, LocalStack, Prometheus, and Grafana
- **Examples** - Runnable examples for basic usage, compression, analytics patterns, and metrics
- **Documentation** - Getting started guide, architecture docs, S3 configuration guide, and local development guide

### Security

- Security audits with `cargo-audit`
- License compliance with `cargo-deny`
- No hardcoded credentials

---

[Unreleased]: https://github.com/valeriouberti/rigatoni/compare/v0.1.4...HEAD
[0.1.4]: https://github.com/valeriouberti/rigatoni/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/valeriouberti/rigatoni/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/valeriouberti/rigatoni/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/valeriouberti/rigatoni/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/valeriouberti/rigatoni/releases/tag/v0.1.0
