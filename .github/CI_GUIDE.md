# CI/CD Guide for Rigatoni

## Running CI Checks Locally

Before pushing code, run these commands locally to catch issues early:

### 1. Run All Tests

```bash
# Test with default features
cargo test --workspace

# Test with all features
cargo test --workspace --all-features

# Test with no default features
cargo test --workspace --no-default-features

# Run doc tests
cargo test --workspace --doc --all-features
```

### 2. Run Clippy (Linting)

```bash
# Standard clippy check with strict lints
cargo clippy --workspace --all-features --all-targets -- \
  -D warnings \
  -D clippy::all \
  -D clippy::pedantic \
  -W clippy::nursery \
  -A clippy::module_name_repetitions \
  -A clippy::missing_errors_doc \
  -A clippy::missing_panics_doc

# Check with no default features
cargo clippy --workspace --no-default-features --all-targets -- -D warnings
```

### 3. Check Formatting

```bash
# Check if code is formatted correctly
cargo fmt --all -- --check

# Auto-fix formatting issues
cargo fmt --all
```

### 4. Build Documentation

```bash
# Build docs with warnings as errors
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --document-private-items
```

### 5. Run Cargo Check

```bash
# Quick compile check
cargo check --workspace --all-features

# Check without default features
cargo check --workspace --no-default-features
```

## Complete Pre-Push Script

Create a script to run all checks before pushing:

**`scripts/pre-push.sh`** (Linux/macOS):

```bash
#!/bin/bash
set -e

echo "🧪 Running tests..."
cargo test --workspace --all-features

echo "📎 Running Clippy..."
cargo clippy --workspace --all-features --all-targets -- -D warnings

echo "🎨 Checking formatting..."
cargo fmt --all -- --check

echo "📚 Building documentation..."
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps

echo "✅ All checks passed!"
```

**`scripts/pre-push.ps1`** (Windows PowerShell):

```powershell
$ErrorActionPreference = "Stop"

Write-Host "🧪 Running tests..." -ForegroundColor Cyan
cargo test --workspace --all-features

Write-Host "📎 Running Clippy..." -ForegroundColor Cyan
cargo clippy --workspace --all-features --all-targets -- -D warnings

Write-Host "🎨 Checking formatting..." -ForegroundColor Cyan
cargo fmt --all -- --check

Write-Host "📚 Building documentation..." -ForegroundColor Cyan
$env:RUSTDOCFLAGS="-D warnings"
cargo doc --workspace --all-features --no-deps

Write-Host "✅ All checks passed!" -ForegroundColor Green
```

Make the script executable:

```bash
chmod +x scripts/pre-push.sh
```

## CI Optimization Tips

### 1. Rust Cache Configuration

The workflow uses `Swatinem/rust-cache` which automatically:

- Caches compiled dependencies in `target/`
- Caches cargo registry and git checkouts
- Invalidates cache when `Cargo.lock` changes
- Separates caches by job and OS

**Why rust-cache over actions/cache?**

- Automatically handles Rust-specific cache keys
- Better cache hit rates
- Simpler configuration
- Actively maintained for Rust workflows

### 2. Concurrency Controls

The workflow cancels in-progress runs when new commits are pushed to the same branch:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

This saves CI minutes and provides faster feedback.

### 3. Fail-Fast Strategy

`fail-fast: false` ensures all platform tests run even if one fails, giving you complete information about platform-specific issues.

## Debugging CI Failures

### 1. Check the Logs

Click on the failed job in GitHub Actions to see detailed logs.

### 2. Reproduce Locally

Use the same commands as CI:

```bash
# Match the CI environment
cargo clean
cargo test --workspace --all-features --verbose
```

### 3. Platform-Specific Issues

If a test fails only on specific platforms:

**For Windows issues:**

- Use WSL2 or a Windows VM to reproduce
- Check for path separator issues (`/` vs `\`)
- Look for case-sensitivity problems

**For macOS issues:**

- Check for Unix-specific assumptions
- Verify file system case sensitivity

### 4. Enable Debug Logging

Add to your workflow temporarily:

```yaml
env:
  RUST_LOG: debug
  RUST_BACKTRACE: full
```

## Adding More Checks

### Security Audit with cargo-audit

✅ **Enabled by default** - The `audit` job in [ci.yml](../workflows/ci.yml) checks for security vulnerabilities in dependencies.

To run locally:

```bash
cargo install cargo-audit
cargo audit
```

### License and Dependency Checks with cargo-deny

✅ **Enabled by default** - The `deny` job enforces:

- License compliance (only allow approved licenses)
- Ban specific dependencies
- Check for security advisories
- Detect duplicate dependencies

Configuration file: [deny.toml](../../deny.toml)

```bash
cargo install cargo-deny
cargo deny init  # Creates deny.toml
cargo deny check
```

### Code Coverage

Add code coverage with `cargo-tarpaulin` or `cargo-llvm-cov`:

```yaml
coverage:
  name: Code Coverage
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - name: Install tarpaulin
      run: cargo install cargo-tarpaulin
    - name: Generate coverage
      run: cargo tarpaulin --workspace --all-features --out Xml
    - name: Upload to codecov
      uses: codecov/codecov-action@v3
```

## Branch Protection Rules

Configure branch protection in GitHub Settings → Branches:

1. Require status checks to pass:
   - ✅ CI Success
2. Require branches to be up to date
3. Include administrators (recommended)

This ensures all code is tested before merging.

## Troubleshooting Common Issues

### "Clippy is not installed"

```bash
rustup component add clippy
```

### "rustfmt is not installed"

```bash
rustup component add rustfmt
```

### Cache Not Working

- Ensure `Cargo.lock` is committed
- Check if cache key is unique per job
- Verify rust-cache action version is latest

### Tests Pass Locally but Fail in CI

- Check environment variables
- Verify test isolation (no shared state)
- Look for timing-dependent tests
- Check for absolute path assumptions

## Continuous Integration Best Practices

1. **Keep tests fast**: Slow tests discourage running them locally
2. **Make tests deterministic**: No random failures
3. **Test in isolation**: Each test should be independent
4. **Use meaningful test names**: Easy to identify failures
5. **Don't commit failing tests**: Fix or mark as `#[ignore]`
6. **Run CI checks before pushing**: Use the pre-push script
7. **Monitor CI trends**: Track build times and flaky tests

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [rust-cache Action](https://github.com/Swatinem/rust-cache)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/index.html)
- [cargo-deny Documentation](https://embarkstudios.github.io/cargo-deny/)
- [cargo-audit Documentation](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
