#!/bin/bash
# Rigatoni Benchmarks Runner
#
# This script helps you run benchmarks with LocalStack

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Print colored messages
error() {
    echo -e "${RED}Error: $1${NC}" >&2
}

success() {
    echo -e "${GREEN}✓ $1${NC}"
}

info() {
    echo -e "${YELLOW}→ $1${NC}"
}

# Check if docker is running
if ! docker info > /dev/null 2>&1; then
    error "Docker is not running. Please start Docker and try again."
    exit 1
fi

success "Docker is running"

# Check if LocalStack is running
if ! docker ps | grep -q rigatoni-localstack; then
    info "Starting LocalStack..."
    cd "$(dirname "$0")/.."
    docker compose -f docker/docker-compose.localstack.yml up -d

    # Wait for LocalStack to be healthy
    info "Waiting for LocalStack to be ready..."
    timeout=30
    elapsed=0
    while ! docker exec rigatoni-localstack curl -sf http://localhost:4566/_localstack/health > /dev/null 2>&1; do
        if [ $elapsed -ge $timeout ]; then
            error "LocalStack failed to start within ${timeout}s"
            exit 1
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done

    success "LocalStack is ready"
else
    success "LocalStack is already running"
fi

# Run benchmarks
BENCH_TYPE="${1:-all}"

case "$BENCH_TYPE" in
    all)
        info "Running all benchmarks..."
        cargo bench --package rigatoni-benches
        ;;
    s3)
        info "Running S3 destination benchmarks..."
        cargo bench --package rigatoni-benches --bench s3_destination
        ;;
    pipeline)
        info "Running pipeline throughput benchmarks..."
        cargo bench --package rigatoni-benches --bench pipeline_throughput
        ;;
    batch)
        info "Running batch processing benchmarks..."
        cargo bench --package rigatoni-benches --bench batch_processing
        ;;
    *)
        error "Unknown benchmark type: $BENCH_TYPE"
        echo "Usage: $0 [all|s3|pipeline|batch]"
        exit 1
        ;;
esac

success "Benchmarks completed!"
info "View results in target/criterion/report/index.html"

# Optionally open the report
if command -v open > /dev/null 2>&1; then
    read -p "Open benchmark report? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        open target/criterion/report/index.html
    fi
elif command -v xdg-open > /dev/null 2>&1; then
    read -p "Open benchmark report? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        xdg-open target/criterion/report/index.html
    fi
fi
