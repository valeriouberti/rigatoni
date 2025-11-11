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

echo "🔒 Running security audit..."
cargo audit --deny unsound --deny yanked

echo "📋 Running cargo deny..."
cargo deny check

echo "✅ All checks passed!"
