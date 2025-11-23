---
layout: default
title: Contributing
nav_order: 5
description: "Learn how to contribute to Rigatoni."
permalink: /contributing
---

# Contributing to Rigatoni

{: .no_toc }

Thank you for your interest in contributing to Rigatoni!
{: .fs-6 .fw-300 }

## Table of contents

{: .no_toc .text-delta }

1. TOC
   {:toc}

---

## Code of Conduct

Be respectful, inclusive, and professional in all interactions. We're here to build great software together.

---

## Getting Started

### Prerequisites

- **Rust 1.88+** - [Install Rust](https://www.rust-lang.org/tools/install)
- **Git** - [Install Git](https://git-scm.com/downloads)
- **MongoDB** - For testing (or use Docker)
- **LocalStack** - For S3 testing (optional)

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork:

```bash
git clone https://github.com/YOUR_USERNAME/rigatoni.git
cd rigatoni
```

3. Add upstream remote:

```bash
git remote add upstream https://github.com/valeriouberti/rigatoni.git
```

---

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

**Branch Naming:**

- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation changes
- `refactor/` - Code refactoring
- `test/` - Test additions/improvements

### 2. Make Changes

Write your code following our [coding standards](#coding-standards).

### 3. Run Tests

```bash
# Run all tests
cargo test --workspace --all-features

# Run specific crate tests
cargo test -p rigatoni-core --all-features

# Run with logging
RUST_LOG=debug cargo test
```

### 4. Run Quality Checks

We provide a pre-push script that runs all CI checks:

```bash
# Linux/macOS
./scripts/pre-push.sh

# Windows PowerShell
.\scripts\pre-push.ps1
```

This runs:

- Formatting checks
- Clippy linting
- Tests (default, all features, no features)
- Documentation build
- Security audit

### 5. Commit Changes

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```bash
git add .
git commit -m "feat: add support for Kafka destination"
git commit -m "fix: resolve S3 upload timeout issue"
git commit -m "docs: update getting started guide"
```

**Commit Message Format:**

```
<type>: <description>

[optional body]

[optional footer]
```

**Types:**

- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation only
- `style` - Code style changes (formatting, etc.)
- `refactor` - Code refactoring
- `test` - Adding tests
- `chore` - Maintenance tasks

### 6. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub.

---

## Pull Request Guidelines

### PR Title

Follow the same format as commit messages:

```
feat: add Kafka destination support
fix: resolve S3 upload timeout
docs: improve getting started guide
```

### PR Description

Provide a clear description:

```markdown
## Summary

Brief description of what this PR does.

## Changes

- List of specific changes
- Another change
- Yet another change

## Testing

How you tested the changes:

- [ ] Unit tests added/updated
- [ ] Integration tests passing
- [ ] Manually tested with [describe scenario]

## Related Issues

Closes #123
Relates to #456
```

### PR Checklist

Before submitting, ensure:

- [ ] Code follows project style
- [ ] Tests added for new functionality
- [ ] All tests passing
- [ ] Documentation updated
- [ ] Changelog updated (if applicable)
- [ ] No warnings from Clippy
- [ ] Code formatted with `rustfmt`
- [ ] Pre-push script passes

---

## Coding Standards

### Rust Style

Follow the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/):

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

### Clippy Lints

Fix all Clippy warnings:

```bash
# Run Clippy
cargo clippy --workspace --all-features -- -D warnings

# Strict mode (what CI uses)
cargo clippy --workspace --all-features -- \
    -D warnings \
    -D clippy::all \
    -D clippy::pedantic
```

### Documentation

Document all public APIs:

````rust
/// Writes a batch of events to the destination.
///
/// # Arguments
///
/// * `events` - Slice of change events to write
///
/// # Errors
///
/// Returns `DestinationError` if the write fails.
///
/// # Examples
///
/// ```
/// # use rigatoni_core::destination::Destination;
/// # async fn example(destination: &mut impl Destination, events: &[ChangeEvent]) {
/// destination.write_batch(events).await.unwrap();
/// # }
/// ```
#[async_trait]
async fn write_batch(&mut self, events: &[ChangeEvent]) -> Result<(), DestinationError>;
````

### Error Handling

Use `thiserror` for custom errors:

```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("network error")]
    Network(#[from] std::io::Error),

    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

### Testing

Write comprehensive tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Arrange
        let input = 42;

        // Act
        let result = process(input);

        // Assert
        assert_eq!(result, 84);
    }

    #[tokio::test]
    async fn test_async_functionality() {
        let result = async_process().await;
        assert!(result.is_ok());
    }
}
```

### License Headers

All files must include the Apache-2.0 license header:

```rust
// Copyright 2025 Rigatoni Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0
```

---

## Testing

### Unit Tests

Located in `#[cfg(test)]` modules:

```bash
# Run unit tests
cargo test --lib

# Run specific test
cargo test test_name
```

### Integration Tests

Located in `tests/` directory:

```bash
# Run integration tests
cargo test --test '*'

# Run specific integration test
cargo test --test s3_integration_test
```

### With LocalStack

Start LocalStack:

```bash
cd rigatoni-destinations
docker-compose up -d
```

Run S3 integration tests:

```bash
cargo test --test s3_integration_test --features s3,json,gzip -- --ignored
```

### Test Coverage

We aim for >80% test coverage:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --workspace --all-features --out Html
```

---

## Adding New Features

### New Destination

1. Create module: `rigatoni-destinations/src/newdest/`
2. Implement `Destination` trait
3. Add feature flag in `Cargo.toml`
4. Write tests
5. Add documentation
6. Add example

**Example Structure:**

```
rigatoni-destinations/src/newdest/
├── mod.rs           # Public exports
├── destination.rs   # Destination implementation
├── config.rs        # Configuration
└── error.rs         # Error types
```

### New Source

1. Create module: `rigatoni-core/src/sources/newsource/`
2. Implement source logic
3. Add feature flag
4. Write tests
5. Add documentation

---

## Documentation

### API Documentation

Write comprehensive rustdoc comments:

```bash
# Generate docs
cargo doc --no-deps --all-features --open
```

### User Documentation

Update markdown files in `docs/`:

- `docs/getting-started.md` - Getting started guide
- `docs/architecture.md` - Architecture docs
- `docs/guides/` - Task-specific guides

### Examples

Add examples to `examples/` directory:

```bash
cargo run --example new_example --features required-features
```

---

## Release Process

(For maintainers)

### Version Bump

1. Update version in `Cargo.toml`:

```toml
[workspace.package]
version = "0.2.0"
```

2. Update `CHANGELOG.md`

3. Commit and tag:

```bash
git commit -am "chore: bump version to 0.2.0"
git tag -a v0.2.0 -m "Release v0.2.0"
git push --tags
```

### Publishing

```bash
# Dry run
cargo publish --dry-run -p rigatoni-core

# Publish (in dependency order)
cargo publish -p rigatoni-core
# Wait 2-3 minutes for crates.io to index
cargo publish -p rigatoni-destinations
cargo publish -p rigatoni-stores
```

---

## Getting Help

### Communication Channels

- **Issues** - [GitHub Issues](https://github.com/valeriouberti/rigatoni/issues)
- **Discussions** - [GitHub Discussions](https://github.com/valeriouberti/rigatoni/discussions)
- **Email** - valeriouberti@icloud.com

### Issue Templates

Use the appropriate template when creating issues:

- **Bug Report** - For reporting bugs
- **Feature Request** - For proposing new features
- **Question** - For asking questions

### Where to Start

Good first issues are labeled `good first issue`:

[View Good First Issues](https://github.com/valeriouberti/rigatoni/labels/good%20first%20issue)

---

## Common Tasks

### Add a Dependency

1. Add to `workspace.dependencies` in root `Cargo.toml`:

```toml
[workspace.dependencies]
new-crate = "1.0"
```

2. Reference in member crate:

```toml
[dependencies]
new-crate = { workspace = true }
```

### Add a Feature Flag

1. Update member `Cargo.toml`:

```toml
[features]
new-feature = ["dep:some-crate"]

[dependencies]
some-crate = { workspace = true, optional = true }
```

2. Use conditional compilation:

```rust
#[cfg(feature = "new-feature")]
pub mod new_feature;
```

### Update Dependencies

```bash
# Check outdated dependencies
cargo outdated

# Update within semver
cargo update

# Update to latest (may break)
cargo upgrade
```

---

## Troubleshooting

### Tests Failing

```bash
# Clean and rebuild
cargo clean
cargo test

# Check for outdated dependencies
cargo update
```

### Clippy Errors

```bash
# Auto-fix where possible
cargo clippy --fix --workspace --all-features
```

### Formatting Issues

```bash
# Auto-format
cargo fmt --all
```

---

## Recognition

Contributors will be:

- Listed in `CONTRIBUTORS.md`
- Mentioned in release notes
- Given credit in commit messages with `Co-authored-by:`

---

## License

By contributing to Rigatoni, you agree that your contributions will be licensed under the Apache License 2.0.

---

Thank you for contributing to Rigatoni! 🍝

Questions? Open a [discussion](https://github.com/valeriouberti/rigatoni/discussions) or [issue](https://github.com/valeriouberti/rigatoni/issues).
