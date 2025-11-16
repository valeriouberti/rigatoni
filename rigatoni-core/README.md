# rigatoni-core

Core traits, pipeline orchestration, and MongoDB integration for the Rigatoni ETL framework.

[![Crates.io](https://img.shields.io/crates/v/rigatoni-core.svg)](https://crates.io/crates/rigatoni-core)
[![Documentation](https://docs.rs/rigatoni-core/badge.svg)](https://docs.rs/rigatoni-core)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE)

## Overview

`rigatoni-core` provides the foundational components for building ETL pipelines with Rigatoni:

- **Pipeline Orchestration** - Multi-worker architecture with retry logic and graceful shutdown
- **MongoDB Source** - Real-time change stream integration with resume token support
- **Destination Trait** - Generic interface for pluggable output destinations
- **Event Model** - Type-safe change event representation
- **State Management** - Resume token persistence for fault tolerance

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rigatoni-core = "0.1"
```

## Quick Start

```rust
use rigatoni_core::pipeline::{Pipeline, PipelineConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = PipelineConfig::builder()
        .mongodb_uri("mongodb://localhost:27017")
        .database("mydb")
        .collections(vec!["users".to_string()])
        .batch_size(1000)
        .build()?;

    let destination = /* your destination */;
    let mut pipeline = Pipeline::new(config, destination).await?;
    pipeline.run().await?;

    Ok(())
}
```

## Features

This crate includes:

- MongoDB change stream source (default)
- Pipeline orchestration with batching and retry
- Event model and destination trait
- State store trait for resume tokens

## Documentation

- [Getting Started Guide](https://valeriouberti.github.io/rigatoni/getting-started)
- [Architecture Documentation](https://valeriouberti.github.io/rigatoni/architecture)
- [API Documentation](https://docs.rs/rigatoni-core)
- [Main Repository](https://github.com/valeriouberti/rigatoni)

## License

Licensed under either of:

- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
