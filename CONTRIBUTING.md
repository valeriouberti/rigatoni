# Contributing to Rigatoni

Thank you for your interest in contributing to Rigatoni!

## Quick Start

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run checks: `./scripts/pre-push.sh`
5. Submit a pull request

## Comprehensive Guide

For complete contributing guidelines, please see our detailed documentation:

**[Contributing Guide](https://valeriouberti.github.io/rigatoni/contributing)** or [docs/contributing.md](docs/contributing.md)

This comprehensive guide includes:

- **Development Setup** - Local environment, dependencies, and tools
- **Code Standards** - Style guides, linting, and formatting
- **Testing Requirements** - Unit tests, integration tests, and coverage
- **Pull Request Process** - PR guidelines, review process, and best practices
- **CI/CD Workflow** - Understanding the automated checks
- **Feature Flags** - Working with optional features
- **Release Process** - How releases are created and published
- **Documentation** - Writing and updating docs
- **Troubleshooting** - Common issues and solutions

## Essential Requirements

Before submitting a PR:

- ✅ All tests pass: `cargo test --workspace --all-features`
- ✅ Code is formatted: `cargo fmt --all`
- ✅ Lints pass: `cargo clippy --workspace --all-features -- -D warnings`
- ✅ License headers included (Apache 2.0)
- ✅ Documentation updated if needed

Run all checks at once:
```bash
./scripts/pre-push.sh
```

## License Header

All Rust source files must include the Apache 2.0 license header:

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

## Code of Conduct

We are committed to providing a welcoming and inclusive experience for everyone. Please:

- Be respectful and considerate
- Welcome newcomers and help them get started
- Focus on what is best for the community
- Show empathy towards other community members

## Contributor License Agreement

By contributing to Rigatoni, you agree that your contributions will be licensed under the Apache License 2.0.

## Questions?

- **Documentation:** [Contributing Guide](https://valeriouberti.github.io/rigatoni/contributing)
- **Discussions:** [GitHub Discussions](https://github.com/valeriouberti/rigatoni/discussions)
- **Issues:** [GitHub Issues](https://github.com/valeriouberti/rigatoni/issues)

Thank you for contributing to Rigatoni! 🍝
