#!/bin/bash
# Quick start script for local development with Rigatoni

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored output
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_header() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""
}

# Check prerequisites
check_prerequisites() {
    print_header "Checking Prerequisites"

    # Check Docker
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed"
        echo "Install from: https://docs.docker.com/get-docker/"
        exit 1
    fi
    print_success "Docker found: $(docker --version | cut -d' ' -f3)"

    # Check Docker Compose
    if ! docker compose version &> /dev/null; then
        print_error "Docker Compose is not installed"
        echo "Install from: https://docs.docker.com/compose/install/"
        exit 1
    fi
    print_success "Docker Compose found: $(docker compose version | cut -d' ' -f4)"

    # Check Rust
    if ! command -v cargo &> /dev/null; then
        print_error "Rust is not installed"
        echo "Install from: https://rustup.rs/"
        exit 1
    fi
    print_success "Rust found: $(rustc --version | cut -d' ' -f2)"

    # Check awslocal (optional)
    if command -v awslocal &> /dev/null; then
        print_success "awslocal found (optional)"
    else
        print_warning "awslocal not found (optional, but recommended)"
        echo "Install with: pip install awscli-local"
    fi
}

# Start Docker services
start_services() {
    print_header "Starting Docker Services"

    print_info "Starting MongoDB, Redis, LocalStack, Prometheus, and Grafana..."

    # Determine the correct docker-compose path based on current directory
    if [ -f "docker/docker-compose.yml" ]; then
        # Running from repository root
        docker compose -f docker/docker-compose.yml up -d
    elif [ -f "docker-compose.yml" ]; then
        # Running from docker/ directory
        docker compose up -d
    elif [ -f "../docker-compose.yml" ]; then
        # Running from docker/scripts/ directory
        docker compose -f ../docker-compose.yml up -d
    else
        print_error "Could not find docker-compose.yml"
        echo "Please run this script from the repository root, docker/, or docker/scripts/ directory"
        exit 1
    fi

    print_info "Waiting for services to be healthy..."
    sleep 5

    # Wait for MongoDB
    print_info "Waiting for MongoDB replica set..."
    max_attempts=30
    attempt=0
    while [ $attempt -lt $max_attempts ]; do
        if docker exec rigatoni-mongodb mongosh --quiet --eval "rs.status()" > /dev/null 2>&1; then
            print_success "MongoDB replica set is ready"
            break
        fi
        attempt=$((attempt + 1))
        sleep 2
    done

    if [ $attempt -eq $max_attempts ]; then
        print_error "MongoDB replica set failed to initialize"
        exit 1
    fi

    # Check Redis
    if docker exec rigatoni-redis redis-cli -a redispassword PING > /dev/null 2>&1; then
        print_success "Redis is ready"
    else
        print_error "Redis failed to start"
        exit 1
    fi

    # Check LocalStack
    if curl -s http://localhost:4566/_localstack/health > /dev/null 2>&1; then
        print_success "LocalStack is ready"
    else
        print_error "LocalStack failed to start"
        exit 1
    fi

    # Check Prometheus
    if curl -s http://localhost:9090/-/healthy > /dev/null 2>&1; then
        print_success "Prometheus is ready"
    else
        print_warning "Prometheus may not be ready yet"
    fi

    # Check Grafana
    if curl -s http://localhost:3000/api/health > /dev/null 2>&1; then
        print_success "Grafana is ready"
    else
        print_warning "Grafana may not be ready yet"
    fi
}

# Verify services
verify_services() {
    print_header "Verifying Services"

    echo ""
    # Determine correct path for ps command
    if [ -f "docker/docker-compose.yml" ]; then
        docker compose -f docker/docker-compose.yml ps
    elif [ -f "docker-compose.yml" ]; then
        docker compose ps
    elif [ -f "../docker-compose.yml" ]; then
        docker compose -f ../docker-compose.yml ps
    fi
    echo ""

    print_info "Service endpoints:"
    echo "  • MongoDB:        mongodb://admin:password@localhost:27017/?replicaSet=rs0"
    echo "  • Redis:          redis://:redispassword@localhost:6379"
    echo "  • LocalStack S3:  http://localhost:4566"
    echo ""

    print_info "Web UIs:"
    echo "  • MongoDB Express:  http://localhost:8081"
    echo "  • Redis Commander:  http://localhost:8082"
    echo "  • Prometheus:       http://localhost:9090"
    echo "  • Grafana:          http://localhost:3000 (admin/admin)"
    echo ""

    print_info "Metrics:"
    echo "  • Your app should expose metrics at: http://localhost:9000/metrics"
    echo "  • Prometheus will scrape from:       host.docker.internal:9000"
    echo ""
}

# Show usage instructions
show_usage() {
    print_header "Quick Start Complete! 🎉"

    print_info "Next steps:"
    echo ""
    echo "1️⃣  Run the example pipeline:"
    echo "   ${GREEN}cargo run --example metrics_prometheus --features metrics-export${NC}"
    echo ""
    echo "2️⃣  In a new terminal, generate test data:"
    echo "   ${GREEN}./docker/scripts/generate-test-data.sh${NC}"
    echo ""
    echo "3️⃣  Monitor the pipeline processing changes!"
    echo ""

    print_info "Useful commands:"
    echo ""
    echo "  • View MongoDB data:    ${BLUE}http://localhost:8081${NC}"
    echo "  • View Redis state:     ${BLUE}http://localhost:8082${NC}"
    echo "  • View metrics:         ${BLUE}http://localhost:9090${NC} (Prometheus)"
    echo "  • View dashboards:      ${BLUE}http://localhost:3000${NC} (Grafana: admin/admin)"
    echo "  • List S3 objects:      ${GREEN}awslocal s3 ls s3://rigatoni-test-bucket/mongodb-cdc/ --recursive${NC}"
    echo "  • View pipeline logs:   ${GREEN}RUST_LOG=debug cargo run --example metrics_prometheus --features metrics-export${NC}"
    echo "  • Generate test data:   ${GREEN}./docker/scripts/generate-test-data.sh${NC}"
    echo "  • Stop services:        ${GREEN}cd docker && docker compose down${NC}"
    echo "  • Reset everything:     ${GREEN}cd docker && docker compose down -v${NC}"
    echo ""

    print_info "Documentation:"
    echo "  • Local Dev Guide:    docs/guides/local-development.md"
    echo "  • Docker Compose:     docker/README.md"
    echo "  • Observability:      docs/OBSERVABILITY.md"
    echo ""
}

# Main execution
main() {
    clear
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║  Rigatoni Local Development Quick Start   ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
    echo ""

    check_prerequisites
    start_services
    verify_services
    show_usage

    print_success "All systems ready! Happy coding! 🚀"
    echo ""
}

# Run main function
main
