# Rigatoni

<p align="center">
    <img src="assets/logo.webp" alt="Rigatoni Logo" width="180" />
</p>

[![CI](https://github.com/valeriouberti/rigatoni/actions/workflows/ci.yml/badge.svg)](https://github.com/valeriouberti/rigatoni/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)

> A high-performance, type-safe ETL framework for Rust, focused on real-time data pipelines.

## 🎯 Overview

Rigatoni is a modern ETL framework built for speed, reliability, and developer experience. Built with Rust's type system and async/await, it provides production-ready data pipelines for real-time streaming workloads.

**Currently supporting:**

- **MongoDB Change Streams** - Real-time CDC (Change Data Capture) from MongoDB
- **S3 Destination** - Export to AWS S3 with multiple formats (JSON, CSV, Parquet, Avro)
- **Redis State Store** - Distributed state management for multi-instance deployments
- **Pipeline Orchestration** - Multi-worker architecture with retry logic and state management
- **Metrics & Observability** - Prometheus metrics with Grafana dashboards
- **Async-first design** - Powered by Tokio for high throughput
- **Type-safe transformations** - Compile-time guarantees with Rust's type system
- **Modular architecture** - Extensible with feature flags

## ✨ Features

- 🚀 **High Performance**: Async/await architecture with Tokio for concurrent processing
- 🔒 **Type Safety**: Leverage Rust's type system for data transformation guarantees
- 📊 **MongoDB CDC**: Real-time change stream listening with resume token support
- 📦 **S3 Integration**: Multiple formats (JSON, CSV, Parquet, Avro) with compression (gzip, zstd)
- 🗄️ **Distributed State**: Redis-backed state store for multi-instance deployments
- 🔄 **Retry Logic**: Exponential backoff with configurable limits
- 🎯 **Batching**: Automatic batching based on size and time windows
- 🎨 **Composable Pipelines**: Build ETL workflows from simple, testable components
- 📊 **Metrics**: Prometheus metrics for throughput, latency, errors, and health
- 📝 **Observability**: Comprehensive tracing, metrics, and Grafana dashboards
- 🧪 **Testable**: Mock destinations and comprehensive test utilities

## 🏗️ Architecture

Rigatoni is organized as a workspace with three main crates:

```
rigatoni/
├── rigatoni-core/           # Core traits and pipeline orchestration
├── rigatoni-destinations/   # Destination implementations
└── rigatoni-stores/         # State store implementations
```

### Core Concepts

- **Source**: Extract data from systems (MongoDB change streams)
- **Transform**: Process and enrich data with type-safe transformations
- **Destination**: Load data into target systems (S3 with multiple formats)
- **Store**: Manage pipeline state for reliability (in-memory, file, Redis)
- **Pipeline**: Orchestrate the entire ETL workflow with error handling

## 🚀 Quick Start

### Prerequisites

- Rust 1.85 or later
- AWS credentials configured for S3 access

### Installation

Add Rigatoni to your `Cargo.toml`:

```toml
[dependencies]
rigatoni-core = "0.1"
rigatoni-destinations = { version = "0.1", features = ["s3"] }
rigatoni-stores = { version = "0.1", features = ["redis-store"] }
```

### Basic Example: MongoDB to S3 Pipeline

```rust
use rigatoni_core::pipeline::{Pipeline, PipelineConfig};
use rigatoni_destinations::s3::{S3Config, S3Destination};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure S3 destination
    let s3_config = S3Config::builder()
        .bucket("my-data-lake")
        .region("us-east-1")
        .prefix("mongodb-cdc")
        .build()?;

    let destination = S3Destination::new(s3_config).await?;

    // Configure pipeline
    let config = PipelineConfig::builder()
        .mongodb_uri("mongodb://localhost:27017")
        .database("mydb")
        .collections(vec!["users", "orders"])
        .batch_size(1000)
        .build()?;

    // Run pipeline
    let mut pipeline = Pipeline::new(config, destination).await?;
    pipeline.run().await?;

    Ok(())
}
```

### Distributed State with Redis

For multi-instance deployments, use Redis to share state across pipeline instances:

```rust
use rigatoni_core::pipeline::{Pipeline, PipelineConfig};
use rigatoni_destinations::s3::{S3Config, S3Destination};
use rigatoni_stores::redis::{RedisStore, RedisConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure Redis state store
    let redis_config = RedisConfig::builder()
        .url("redis://localhost:6379")
        .pool_size(10)
        .ttl(Duration::from_secs(7 * 24 * 60 * 60))  // 7 days
        .build()?;

    let store = RedisStore::new(redis_config).await?;

    // Configure S3 destination
    let s3_config = S3Config::builder()
        .bucket("my-data-lake")
        .region("us-east-1")
        .build()?;

    let destination = S3Destination::new(s3_config).await?;

    // Configure pipeline with Redis store
    let config = PipelineConfig::builder()
        .mongodb_uri("mongodb://localhost:27017")
        .database("mydb")
        .collections(vec!["users", "orders"])
        .build()?;

    // Run pipeline with distributed state
    let mut pipeline = Pipeline::with_store(config, destination, store).await?;
    pipeline.run().await?;

    Ok(())
}
```

See [Getting Started](https://valeriouberti.github.io/rigatoni/getting-started) for detailed tutorials and [Redis Configuration Guide](https://valeriouberti.github.io/rigatoni/guides/redis-configuration) for production deployment.

### Metrics and Monitoring

Rigatoni includes comprehensive metrics for production observability:

```rust
use metrics_exporter_prometheus::PrometheusBuilder;
use rigatoni_core::metrics;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize metrics
    metrics::init_metrics();

    // Start Prometheus exporter
    let addr = ([0, 0, 0, 0], 9000).into();
    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()?;

    // Metrics now available at http://localhost:9000/metrics

    // ... configure and run pipeline ...

    Ok(())
}
```

**Available Metrics:**
- **Counters**: events processed, events failed, retries, batches written
- **Histograms**: batch size, batch duration, write latency, write bytes
- **Gauges**: active collections, pipeline status, queue size

See [Observability Guide](docs/OBSERVABILITY.md) for Prometheus setup, Grafana dashboards, and alerting.

## 📚 Documentation

**User Documentation:**
- [Getting Started](https://valeriouberti.github.io/rigatoni/getting-started) - Quick start guide and tutorials
- [Architecture Guide](https://valeriouberti.github.io/rigatoni/architecture) - System design and concepts
- [Observability Guide](docs/OBSERVABILITY.md) - Metrics, monitoring, and alerting
- [API Reference](https://docs.rs/rigatoni) - Complete API documentation
- [User Guides](https://valeriouberti.github.io/rigatoni/guides/) - Task-specific guides

**Developer Documentation:**
- [Contributing Guide](CONTRIBUTING.md) - How to contribute
- [CI/CD Guide](.github/CI_GUIDE.md) - Development workflow
- [Workspace Guide](docs/README-WORKSPACE.md) - Workspace structure and dependencies

## 🛠️ Development

### Building

```bash
# Build all workspace members
cargo build --workspace

# Build with all features
cargo build --workspace --all-features

# Run tests
cargo test --workspace --all-features
```

### Running Checks

We provide a pre-push script to run all CI checks locally:

```bash
# Linux/macOS
./scripts/pre-push.sh

# Windows PowerShell
.\scripts\pre-push.ps1
```

This runs:

- All tests (default features, all features, no default features)
- Clippy linting with strict rules
- Rustfmt formatting checks
- Documentation builds

### Code Quality

Rigatoni maintains high code quality standards:

- ✅ **Automated CI**: All code must pass comprehensive checks
- ✅ **Security Scanning**: Vulnerability detection with `cargo-audit`
- ✅ **License Compliance**: Enforced with `cargo-deny`
- ✅ **Strict Linting**: Clippy pedantic mode

## 🔒 Security

- **Automated Security Audits**: Every commit is scanned for known vulnerabilities
- **Dependency Review**: All dependencies are vetted for license compliance

To report security vulnerabilities, please email: valeriouberti@icloud.com

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

Quick checklist:

1. Fork the repository
2. Create a feature branch
3. Write tests for your changes
4. Run `./scripts/pre-push.sh` to validate
5. Submit a PR

## 📝 License

Rigatoni is dual-licensed under MIT OR Apache-2.0.

## 📧 Contact

- **Author**: Valerio Uberti
- **Email**: valeriouberti@icloud.com
- **Repository**: [github.com/valeriouberti/rigatoni](https://github.com/valeriouberti/rigatoni)
