---
layout: default
title: Home
nav_order: 1
description: "Rigatoni is a high-performance, type-safe ETL framework for Rust, focused on real-time data pipelines."
permalink: /
---

# Rigatoni
{: .fs-9 }

A high-performance, type-safe ETL framework for Rust, focused on real-time data pipelines.
{: .fs-6 .fw-300 }

[Get Started](getting-started){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View on GitHub](https://github.com/valeriouberti/rigatoni){: .btn .fs-5 .mb-4 .mb-md-0 }

---

## Overview

Rigatoni is a modern ETL (Extract, Transform, Load) framework built with Rust, designed for production-ready real-time data pipelines. It combines the performance and safety of Rust with an intuitive API for building reliable data pipelines.

### Key Features

- **🚀 High Performance** - Async/await architecture with Tokio for concurrent processing
- **🔒 Type Safety** - Leverage Rust's type system for compile-time guarantees
- **📊 MongoDB CDC** - Real-time change stream listening with resume token support
- **📦 S3 Integration** - Multiple formats (JSON, CSV, Parquet, Avro) with compression
- **🔄 Retry Logic** - Exponential backoff with configurable limits
- **🎯 Batching** - Automatic batching based on size and time windows
- **🎨 Composable** - Build ETL workflows from simple, testable components
- **📝 Observable** - Comprehensive tracing and metrics support

## Quick Start

### Installation

Add Rigatoni to your `Cargo.toml`:

```toml
[dependencies]
rigatoni-core = "0.1"
rigatoni-destinations = { version = "0.1", features = ["s3"] }
```

### Your First Pipeline

Create a simple MongoDB to S3 pipeline:

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

## Architecture

Rigatoni is organized as a workspace with three main crates:

### Core Components

- **rigatoni-core** - Core traits, pipeline orchestration, and MongoDB integration
- **rigatoni-destinations** - Destination implementations (S3, BigQuery, Kafka, etc.)
- **rigatoni-stores** - State store implementations for checkpoint/resume

### Pipeline Flow

```
┌─────────────┐      ┌───────────┐      ┌──────────────┐
│  MongoDB    │─────▶│  Pipeline │─────▶│  Destination │
│ Change      │      │           │      │   (S3)       │
│ Stream      │      │ (batching,│      │              │
└─────────────┘      │  retry)   │      └──────────────┘
                     └───────────┘
```

[Learn more about architecture →](architecture)

## Use Cases

### Real-time CDC to Data Lake

Stream MongoDB changes to S3 for analytics:

- **Format**: Parquet for efficient columnar storage
- **Partitioning**: Hive-style for query performance
- **Compression**: Zstandard for optimal ratio and speed

### Backup and Archive

Continuous backup of MongoDB collections:

- **Format**: JSON for flexibility
- **Partitioning**: Date-based for lifecycle policies
- **Compression**: Gzip for compatibility

### Event Sourcing

Capture all database changes for audit and replay:

- **Format**: Avro for schema evolution
- **Partitioning**: Collection-based for isolation
- **State Management**: Resume tokens for exactly-once semantics

## Documentation

- **[Getting Started](getting-started)** - Installation, setup, and your first pipeline
- **[Architecture](architecture)** - System design and core concepts
- **[User Guides](guides/)** - Task-specific guides and examples
- **[API Reference](https://docs.rs/rigatoni)** - Complete API documentation
- **[Contributing](contributing)** - Contribution guidelines

## Community

- **GitHub**: [valeriouberti/rigatoni](https://github.com/valeriouberti/rigatoni)
- **Issues**: [Report bugs or request features](https://github.com/valeriouberti/rigatoni/issues)
- **Discussions**: [Ask questions and share ideas](https://github.com/valeriouberti/rigatoni/discussions)

## License

Rigatoni is dual-licensed under **MIT OR Apache-2.0**.

You may choose either license for your use of this software.

---

## Next Steps

Ready to build your first pipeline?

[Get Started with Rigatoni →](getting-started){: .btn .btn-blue }
