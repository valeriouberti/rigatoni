# Rigatoni Tools

This directory contains development tools and utilities for the Rigatoni project.

## Available Tools

### license-header

**Purpose:** Automated license header management for all source files.

**Location:** [`license-header/`](license-header/)

**Usage:**

```bash
cd tools/license-header
cargo run
```

This tool ensures all Rust source files have the correct Apache 2.0 license header.

**When to use:**

- After adding new source files
- When updating license information
- Before releases to ensure compliance

See [license-header/README.md](license-header/README.md) for details.

---

## Adding New Tools

When adding development tools to this directory:

1. Create a subdirectory for the tool: `tools/my-tool/`
2. Include a README.md explaining purpose and usage
3. Update this file with a description
4. Consider whether it should be a workspace member (add to root Cargo.toml if needed)

---

## Tool Guidelines

Development tools in this directory should:

- **Be self-contained** - Include all dependencies
- **Be documented** - Clear README with usage examples
- **Be optional** - Not required for normal development workflow
- **Be tested** - Include tests where appropriate
- **Follow conventions** - Use same code style as main codebase

---

## Related Resources

- **[Contributing Guide](../CONTRIBUTING.md)** - General development guidelines
- **[CI/CD Guide](../.github/CI_GUIDE.md)** - Automation and testing
- **[Local Development](../docs/guides/local-development.md)** - Environment setup
- **[Docker Setup](../docker/README.md)** - Docker Compose reference
