# Rigatoni 🍝

[![CI](https://github.com/valeriouberti/rigatoni/actions/workflows/ci.yml/badge.svg)](https://github.com/valeriouberti/rigatoni/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)

> A high-performance, type-safe ETL framework for Rust, focused on real-time data pipelines.

## 🎯 Overview

Rigatoni is a modern ETL framework built for speed, reliability, and developer experience. Currently supporting:

- **S3 destination** for data export
- **Async-first design** powered by Tokio
- **Type-safe transformations** with compile-time guarantees
- **Modular architecture** for extensibility

## ✨ Features

- 🚀 **High Performance**: Async/await architecture
- 🔒 **Type Safety**: Leverage Rust's type system for data transformation
- 📦 **S3 Integration**: Export data to AWS S3
- 🎨 **Composable Pipelines**: Build ETL workflows from simple components
- 🧪 **Testable**: Comprehensive test utilities

## 🏗️ Architecture

Rigatoni is organized as a workspace with three main crates:

```
rigatoni/
├── rigatoni-core/           # Core traits and pipeline orchestration
├── rigatoni-destinations/   # Destination implementations
└── rigatoni-stores/         # State store implementations
```

### Core Concepts

- **Source**: Extract data from systems
- **Transform**: Process and enrich data with type-safe transformations
- **Destination**: Load data into target systems (currently S3)
- **Store**: Manage pipeline state for reliability
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
rigatoni-destinations = "0.1"
```

### Basic Example

```rust
use rigatoni_destinations::S3Destination;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an S3 destination
    let destination = S3Destination::new("my-bucket").await?;

    // Export data to S3
    destination.write(data).await?;

    Ok(())
}
```

## 📚 Documentation

- [Contributing Guide](CONTRIBUTING.md) - How to contribute
- [CI/CD Guide](.github/CI_GUIDE.md) - Development workflow

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
