# Contributing to Rigatoni

Thank you for your interest in contributing to Rigatoni! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [License Headers](#license-headers)
- [Development Workflow](#development-workflow)
- [Submitting Changes](#submitting-changes)

## Code of Conduct

We are committed to providing a welcoming and inclusive experience for everyone. We expect all contributors to:

- Be respectful and considerate
- Welcome newcomers and help them get started
- Focus on what is best for the community
- Show empathy towards other community members

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/rigatoni.git
   cd rigatoni
   ```
3. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/my-new-feature
   ```

## License Headers

All Rust source files in Rigatoni must include the Apache 2.0 license header.

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

## Development Workflow

### Building the Project

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running Lints

```bash
cargo clippy -- -D warnings
```

### Formatting Code

```bash
cargo fmt
```

## Submitting Changes

### Before Submitting

1. **Ensure all tests pass**: `cargo test`
2. **Format your code**: `cargo fmt`
3. **Run clippy**: `cargo clippy`
4. **Check license headers**: `./tools/license-header/target/release/license-header check`

### Pull Request Process

1. **Push your changes** to your fork
2. **Create a Pull Request** on GitHub
3. **Describe your changes** clearly in the PR description
4. **Link any related issues** using keywords like "Fixes #123"
5. **Wait for review** - maintainers will review your PR and may request changes

### Commit Message Guidelines

Write clear, descriptive commit messages:

```
Add change stream resume token persistence

- Implement token storage in MongoDB
- Add recovery logic on restart
- Include tests for token persistence

Fixes #42
```

## Contributor License Agreement

By submitting a contribution to Rigatoni, you agree that:

1. Your contribution is licensed under Apache 2.0
2. You have the right to submit the contribution
3. You grant the project a perpetual, worldwide, non-exclusive, royalty-free license to use your contribution

This is automatically covered by the Apache 2.0 license (Section 5: Submission of Contributions).

## Questions?

If you have questions about contributing:

- Open a GitHub Discussion
- Ask in an existing issue
- Reach out to maintainers

Thank you for contributing to Rigatoni!
