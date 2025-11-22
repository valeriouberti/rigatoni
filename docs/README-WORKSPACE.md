# Rigatoni Workspace Structure

## Overview

Rigatoni is organized as a Cargo workspace with three member crates, each serving a distinct purpose in the ETL pipeline architecture. This document explains the workspace structure, dependency management strategy, and development workflows.

## Workspace Members

### 1. rigatoni-core
**Purpose**: Core traits, pipeline orchestration, and MongoDB integration

**Key Dependencies**:
- `tokio`: Async runtime for non-blocking I/O
- `mongodb`: Official MongoDB driver with change stream support
- `async-trait`: Trait definitions for async operations
- `serde`/`serde_json`: Data serialization
- `tracing`: Structured logging and observability

**Features**:
- `mongodb-source` (default): MongoDB change stream source
- `metrics-export`: Prometheus metrics export

### 2. rigatoni-destinations
**Purpose**: Destination implementations (currently S3, with BigQuery and Kafka planned)

**Key Dependencies** (feature-gated):
- `aws-sdk-s3`: S3 destination (enabled by `s3` feature)
- `csv`, `parquet`, `apache-avro`: Format support (feature-gated)
- `flate2`, `zstd`: Compression support (feature-gated)

**Features**:
- `default = ["s3", "json", "csv"]`: Common destinations
- `s3`: AWS S3 destination (available in 0.1.1)
- `json`, `csv`, `parquet`, `avro`: Serialization formats
- `all-formats`: Enable all serialization formats
- `compression`: gzip and zstd compression support

**Feature Strategy**: Only compile destination dependencies when explicitly enabled, reducing binary size and compile times.

### 3. rigatoni-stores
**Purpose**: State store implementations for checkpoint/resume

**Key Dependencies**:
- `dashmap`: Concurrent in-memory store
- `redis` (optional): Distributed state management

**Features**:
- `default = ["memory", "file"]`: Local state stores
- `redis-store`: Redis-based distributed state
- `all-stores`: Enable all store implementations

## Workspace Benefits

### 1. Dependency Deduplication
All shared dependencies are defined once in `workspace.dependencies` and inherited by member crates:

```toml
[workspace.dependencies]
tokio = { version = "1.40", features = ["full", "rt-multi-thread"] }
serde = { version = "1.0", features = ["derive"] }
# ... etc
```

Member crates reference them without version numbers:
```toml
[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
```

**Benefits**:
- Single source of truth for versions
- Guaranteed version consistency across crates
- Easier dependency updates (one place to change)
- Reduced `Cargo.lock` complexity

### 2. Shared Metadata
Workspace-level package metadata (`version`, `authors`, `license`, etc.) is inherited by all members:

```toml
[workspace.package]
version = "0.1.1"
edition = "2021"
authors = ["..."]
# ... etc
```

**Benefits**:
- Consistent versioning across releases
- Reduced boilerplate in member crates
- Single location for metadata updates

### 3. Unified Build and Testing
```bash
# Build entire workspace
cargo build

# Test all crates
cargo test

# Check all crates
cargo check

# Build specific crate
cargo build -p rigatoni-core
```

### 4. Resolver v2
```toml
[workspace]
resolver = "2"
```

**Benefits**:
- Better feature unification across dependency graph
- Dev-dependencies don't affect production builds
- More predictable feature resolution

## Dependency Version Strategy

### Caret Requirements (Default)
Most dependencies use caret requirements (e.g., `"1.40"`):
- `1.40.0` ≤ version < `2.0.0`
- Allows automatic patch and minor updates
- Balance between stability and receiving bug fixes

**Used for**: Stable ecosystem crates (tokio, serde, tracing)

### Exact Versions
Not used in this workspace, but would look like `"=1.40.0"`:
- Only exactly `1.40.0`
- Maximum stability, no automatic updates
- Useful for security-critical dependencies

### Tilde Requirements
Not used extensively, but `"~1.40"` means:
- `1.40.0` ≤ version < `1.41.0`
- Only patch updates
- More conservative than caret

### Rationale for Chosen Versions

#### Tokio 1.40
- Mature async runtime (v1.x is stable API)
- Active development and security patches
- Full feature set for comprehensive async support

#### Serde 1.0
- De facto serialization standard
- Stable 1.0 API since 2017
- Derive macros critical for ergonomics

#### MongoDB 3.1
- Latest official driver
- Full async/await support with tokio
- Change stream API for CDC

#### AWS SDK ~1.x
- Recent stable releases
- Breaking changes infrequent in 1.x series
- Active maintenance

## Feature Flags Strategy

### Why Feature Flags?

1. **Compile Time Optimization**: Only compile what you use
   ```bash
   # Only S3 destination
   cargo build -p rigatoni-destinations --features s3 --no-default-features
   ```

2. **Binary Size Reduction**: Exclude unused cloud SDKs
   - Full build with all features: ~50-100MB
   - Minimal build (core + S3): ~15-20MB

3. **Development Flexibility**: Enable all features during development
   ```bash
   cargo build --all-features
   ```

4. **Production Efficiency**: Deploy only required destinations
   ```toml
   [dependencies]
   rigatoni-destinations = { version = "0.1.1", features = ["s3", "json"] }
   ```

### Feature Flag Patterns

#### Additive Features
Most features are additive (don't break existing functionality):
```toml
[features]
s3 = ["aws-config", "aws-sdk-s3"]  # Adds S3 support
json = []  # Adds JSON format support
csv = ["dep:csv"]  # Adds CSV format support
```

#### Optional Dependencies
Dependencies only compiled when feature enabled:
```toml
[dependencies.aws-sdk-s3]
workspace = true
optional = true  # Only compiled if 's3' feature enabled
```

#### Feature Combinations
Meta-features for common combinations:
```toml
[features]
all-destinations = ["s3"]  # Currently only S3 is implemented
all-formats = ["json", "csv", "parquet", "avro"]
all = ["all-destinations", "all-formats", "compression"]
```

## Adding New Crates to Workspace

### Step 1: Create Crate Directory
```bash
mkdir rigatoni-new-crate
cd rigatoni-new-crate
```

### Step 2: Create Minimal Cargo.toml
```toml
[package]
name = "rigatoni-new-crate"
description = "Description of new crate"
readme = "README.md"

# Inherit workspace metadata
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
documentation.workspace = true
keywords.workspace = true
categories.workspace = true

[dependencies]
rigatoni-core = { path = "../rigatoni-core" }
tokio = { workspace = true }
# ... other workspace deps

[lib]
name = "rigatoni_new_crate"
path = "src/lib.rs"
```

### Step 3: Update Root Cargo.toml
```toml
[workspace]
members = [
    "rigatoni-core",
    "rigatoni-destinations",
    "rigatoni-stores",
    "rigatoni-new-crate",  # Add here
]
```

### Step 4: Create src/lib.rs
```bash
mkdir src
echo "// Placeholder" > src/lib.rs
```

### Step 5: Verify
```bash
cargo check
```

## Compile Times and Binary Size

### Compile Time Analysis

#### Cold Build (from scratch)
```bash
cargo clean && cargo build --release
```
- **Expected**: 5-10 minutes (depending on features)
- **Dominated by**: AWS SDK, parquet, rdkafka

#### Incremental Build
```bash
# After changing rigatoni-core
cargo build -p rigatoni-core
```
- **Expected**: 5-30 seconds
- **Only rebuilds**: Changed crate and dependents

#### Parallel Compilation
```bash
# Utilize all CPU cores
cargo build -j $(nproc)
```

### Reducing Compile Times

1. **Feature-gated dependencies**: Disable unused features during development
   ```bash
   cargo build --no-default-features --features core
   ```

2. **Incremental compilation** (enabled by default)
   ```toml
   [profile.dev]
   incremental = true
   ```

3. **Shared build cache**: Workspace shares `target/` directory

4. **Use `cargo check` instead of `cargo build`** for fast feedback
   ```bash
   cargo check  # 2-3x faster than build
   ```

### Binary Size Optimization

#### Default Release Build
```bash
cargo build --release
```
- **Size**: 30-60MB (depends on features)

#### Optimized Release Build
Add to root `Cargo.toml`:
```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization, slower compile
strip = true         # Remove debug symbols
```
- **Size**: 15-25MB (50% reduction)

#### Feature-minimal Build
```bash
cargo build --release -p rigatoni-core --no-default-features
```
- **Size**: 5-10MB

## Development Workflows

### Running Tests
```bash
# All tests
cargo test

# Specific crate
cargo test -p rigatoni-core

# With all features
cargo test --all-features

# Integration tests only
cargo test --test '*'
```

### Checking Code
```bash
# Fast syntax/type checking
cargo check

# With clippy lints
cargo clippy --all-targets --all-features

# Format code
cargo fmt --all
```

### Building Documentation
```bash
# Generate docs for entire workspace
cargo doc --no-deps --all-features

# Open in browser
cargo doc --no-deps --all-features --open
```

### Publishing to crates.io

#### Pre-publish Checklist
1. Ensure all tests pass: `cargo test --all-features`
2. Run clippy: `cargo clippy --all-targets --all-features`
3. Check formatting: `cargo fmt --all -- --check`
4. Build docs: `cargo doc --no-deps --all-features`
5. Update CHANGELOG.md
6. Update version in `workspace.package`

#### Publishing Order (dependency order)
```bash
# 1. Core first (no dependencies)
cd rigatoni-core && cargo publish

# 2. Wait for crates.io to index (2-3 minutes)

# 3. Destinations and stores (depend on core)
cd ../rigatoni-destinations && cargo publish
cd ../rigatoni-stores && cargo publish
```

#### Dry Run
```bash
cargo publish --dry-run
```

## Dependency Update Strategy

### Checking for Updates
```bash
cargo install cargo-outdated
cargo outdated --workspace
```

### Updating Dependencies
```bash
# Update to latest compatible versions (within semver range)
cargo update

# Update specific dependency
cargo update -p tokio

# Update to latest versions (may break semver)
cargo upgrade  # requires cargo-edit
```

### Testing After Updates
```bash
# Full test suite
cargo test --all-features

# Check for breaking changes
cargo check --all-targets --all-features
```

## Troubleshooting

### "Feature X not found"
Ensure feature is enabled:
```bash
cargo build --features X
```

### "Cyclic dependency detected"
Check for circular dependencies between member crates. Use:
```bash
cargo tree
```

### "Version conflict"
Use `resolver = "2"` and check for conflicting version requirements:
```bash
cargo tree -d  # Show duplicate dependencies
```

### Slow Compile Times
1. Use `cargo check` instead of `cargo build`
2. Disable unused features
3. Use `sccache` or `mold` linker
4. Increase `codegen-units` in dev profile

## Summary

The Rigatoni workspace provides:
- **Modularity**: Separate crates for core, destinations, and stores
- **Efficiency**: Shared dependencies and unified build
- **Flexibility**: Feature flags for conditional compilation
- **Maintainability**: Single source of truth for versions and metadata
- **Performance**: Optimized compile times and binary size

This structure supports both rapid development and production deployments while maintaining clear separation of concerns.
