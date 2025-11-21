---
layout: default
title: Guides
nav_order: 4
has_children: true
permalink: /guides
description: "Task-specific guides for common Rigatoni use cases."
---

# User Guides
{: .no_toc }

Task-specific guides for common Rigatoni use cases.
{: .fs-6 .fw-300 }

---

## Available Guides

### S3 Destinations

- **[S3 Configuration](s3-configuration)** - Complete guide to S3 destination configuration
- **[Formats and Compression](formats-compression)** - Choosing the right format and compression
- **[Partitioning Strategies](partitioning)** - Optimize for analytics workloads

### State Stores

- **[Redis Configuration](redis-configuration)** - Complete guide to Redis state store configuration
- **[State Store Comparison](state-stores)** - Choosing the right state store for your deployment

### Pipeline Configuration

- **[Batching and Performance](batching-performance)** - Tuning for throughput and latency
- **[Error Handling and Retries](error-handling)** - Handling failures gracefully
- **[Observability](../OBSERVABILITY)** - Metrics, Prometheus, Grafana dashboards, and alerting

### Development and Testing

- **[Local Development with Docker Compose](local-development)** - Complete local setup with MongoDB, Redis, LocalStack, Prometheus, and Grafana
- **[LocalStack Development](localstack)** - Local development with LocalStack
- **[Testing Strategies](testing)** - Testing your pipelines

### Advanced Topics

- **[Production Deployment](production-deployment)** - Best practices for production

---

## Quick Links

- [Getting Started](../getting-started) - Build your first pipeline
- [Architecture](../architecture) - Understand the system design
- [API Reference](https://docs.rs/rigatoni) - Complete API documentation
