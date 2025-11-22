# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/valeriouberti/rigatoni/compare/main...HEAD
