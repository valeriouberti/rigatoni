#!/bin/bash
# Source this file to set up environment variables for local development
# Usage: source ./scripts/setup-local-env.sh

echo "🔧 Setting up local development environment variables..."

# MongoDB Configuration
export MONGODB_URI="mongodb://localhost:27017/?replicaSet=rs0&directConnection=true"
export MONGODB_DATABASE="testdb"
export MONGODB_COLLECTIONS="users,orders,products"

# Redis Configuration
export REDIS_URL="redis://:redispassword@localhost:6379"

# LocalStack S3 Configuration - REQUIRED for S3 writes
export AWS_ACCESS_KEY_ID="test"
export AWS_SECRET_ACCESS_KEY="test"
export AWS_REGION="us-east-1"
export AWS_ENDPOINT_URL="http://localhost:4566"
export S3_BUCKET="rigatoni-test-bucket"

# Pipeline Configuration
export BATCH_SIZE="100"
export BATCH_TIMEOUT_MS="5000"
export NUM_WORKERS="2"
export MAX_RETRIES="3"

# Logging Configuration
export RUST_LOG="info,rigatoni_core=debug,rigatoni_destinations=debug,rigatoni_stores=debug"

echo "✅ Environment variables set:"
echo "   AWS_ACCESS_KEY_ID=$AWS_ACCESS_KEY_ID"
echo "   AWS_SECRET_ACCESS_KEY=$AWS_SECRET_ACCESS_KEY"
echo "   AWS_REGION=$AWS_REGION"
echo "   MONGODB_URI=$MONGODB_URI"
echo "   REDIS_URL=$REDIS_URL"
echo ""
echo "You can now run:"
echo "   cargo run -p rigatoni-core --example local_development"
