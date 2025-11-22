# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/valeriouberti/rigatoni/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/valeriouberti/rigatoni/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/valeriouberti/rigatoni/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/valeriouberti/rigatoni/releases/tag/v0.1.0
