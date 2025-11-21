#!/bin/bash
# LocalStack initialization script for local S3 development

set -e

echo "Initializing LocalStack S3..."

# Wait for LocalStack to be ready
sleep 2

# Create S3 bucket for testing
awslocal s3 mb s3://rigatoni-test-bucket

# Set bucket to allow public access (for testing only)
awslocal s3api put-bucket-acl \
  --bucket rigatoni-test-bucket \
  --acl private

# Enable versioning (optional)
awslocal s3api put-bucket-versioning \
  --bucket rigatoni-test-bucket \
  --versioning-configuration Status=Enabled

# Create lifecycle policy for automatic cleanup (optional)
awslocal s3api put-bucket-lifecycle-configuration \
  --bucket rigatoni-test-bucket \
  --lifecycle-configuration '{
    "Rules": [
      {
        "Id": "DeleteOldFiles",
        "Status": "Enabled",
        "Prefix": "",
        "Expiration": {
          "Days": 30
        }
      }
    ]
  }'

echo "S3 bucket created: rigatoni-test-bucket"
echo "LocalStack S3 endpoint: http://localhost:4566"
echo "Access Key: test"
echo "Secret Key: test"
echo "Region: us-east-1"

# List buckets to verify
awslocal s3 ls

echo "LocalStack initialization complete!"
