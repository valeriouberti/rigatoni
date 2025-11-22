# License Header Tool

A Rust CLI tool to manage Apache 2.0 license headers in Rust source files for the Rigatoni project.

## Features

- **Apply** headers to Rust files automatically
- **Check** if files have correct headers (CI/CD integration)
- **Remove** headers when needed
- Dry-run mode for safe testing
- Recursive directory processing
- Skips `target/` directories automatically

## Installation

### From source (in the Rigatoni repository):

```bash
cd tools/license-header
cargo build --release
```

The binary will be at `target/release/license-header`.

### Add to PATH (optional):

```bash
# From the tools/license-header directory
cargo install --path .
```

## Usage

### Apply Headers

Apply headers to all Rust files in the current directory:

```bash
license-header apply
```

Apply to a specific directory:

```bash
license-header apply /path/to/src
```

Apply to a specific file:

```bash
license-header apply src/main.rs
```

Dry run (see what would change without modifying files):

```bash
license-header apply --dry-run
```

Force overwrite existing headers:

```bash
license-header apply --force
```

Use a custom header template:

```bash
license-header apply --template my_custom_header.txt
```

### Check Headers

Check if all files have headers (useful for CI/CD):

```bash
license-header check
```

This command exits with code 1 if any files are missing headers, making it perfect for pre-commit hooks or CI pipelines.

### Remove Headers

Remove headers from all Rust files:

```bash
license-header remove
```

Dry run:

```bash
license-header remove --dry-run
```

## Examples

### Example 1: Apply headers to the entire project

```bash
# From the Rigatoni root directory
cd /Users/valeriouberti/personal/side-projects/rigatoni
./tools/license-header/target/release/license-header apply
```

### Example 2: CI/CD Integration

Add to your `.github/workflows/ci.yml`:

```yaml
- name: Check license headers
  run: |
    cd tools/license-header
    cargo build --release
    cd ../..
    ./tools/license-header/target/release/license-header check
```

### Example 3: Pre-commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
./tools/license-header/target/release/license-header check
if [ $? -ne 0 ]; then
    echo "Error: Some files are missing license headers"
    echo "Run: ./tools/license-header/target/release/license-header apply"
    exit 1
fi
```

## Header Template

The default header template is read from the `HEADER` file in the repository root:

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

## How It Works

1. **Detection**: The tool looks for files with `// Copyright` at the start
2. **Application**: Adds the header template before the first line of code
3. **Preservation**: Maintains proper spacing and formatting
4. **Safety**: Dry-run mode lets you preview changes

## Command Reference

```
license-header 0.1.0
Rigatoni Contributors
Apply, check, or remove Apache 2.0 license headers from Rust source files

USAGE:
    license-header <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    apply     Apply license headers to Rust files
    check     Check if files have correct license headers
    remove    Remove license headers from Rust files
    help      Print this message or the help of the given subcommand(s)
```

## Development

Run tests:

```bash
cargo test
```

Run with logging:

```bash
RUST_LOG=debug cargo run -- apply --dry-run
```

## License

This tool is part of the Rigatoni project and is licensed under Apache 2.0.
