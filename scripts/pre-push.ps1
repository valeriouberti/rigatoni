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
