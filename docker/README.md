# Rigatoni Docker Compose Files

This directory contains Docker Compose configurations for running Rigatoni in various modes. Choose the appropriate compose file based on your needs.

## Quick Reference

| File | Use Case | Services | Start Command |
|------|----------|----------|---------------|
| **docker-compose.yml** | Complete local development | All services + UI | `docker compose up -d` |
| **docker-compose.minimal.yml** | Essential services only | MongoDB + Redis | `docker compose -f docker-compose.minimal.yml up -d` |
| **docker-compose.localstack.yml** | S3 testing only | LocalStack S3 | `docker compose -f docker-compose.localstack.yml up -d` |
| **docker-compose.observability.yml** | Metrics stack only | Prometheus + Grafana | `docker compose -f docker-compose.observability.yml up -d` |

## Compose Files

### 1. docker-compose.yml (Recommended)

**Complete local development stack with all services.**

**Includes:**
- MongoDB (replica set with change streams)
- Redis (state store)
- LocalStack (S3 emulation)
- Prometheus (metrics collection)
- Grafana (metrics visualization with dashboards)
- MongoDB Express (web UI - optional)
- Redis Commander (web UI - optional)

**When to use:**
- Learning Rigatoni
- Full-featured local development
- Testing complete pipeline flow
- Observability testing

**Start:**
```bash
docker compose up -d
```

**With UI tools:**
```bash
docker compose --profile ui up -d
```

**Access:**
- MongoDB: `mongodb://localhost:27017/?replicaSet=rs0&directConnection=true`
- Redis: `redis://:redispassword@localhost:6379`
- LocalStack S3: `http://localhost:4566`
- Prometheus: `http://localhost:9090`
- Grafana: `http://localhost:3000` (admin/admin)
- MongoDB Express: `http://localhost:8081` (requires `--profile ui`)
- Redis Commander: `http://localhost:8082` (requires `--profile ui`)

**Documentation:** [docs/guides/local-development.md](../docs/guides/local-development.md)

---

### 2. docker-compose.minimal.yml

**Minimal setup with only essential services.**

**Includes:**
- MongoDB (replica set)
- Redis (state store)

**When to use:**
- Testing with real AWS S3 (not LocalStack)
- Don't need local observability
- Fastest startup time
- CI/CD pipelines

**Note:** You can make this even more minimal by using the in-memory StateStore instead of Redis:
```rust
use rigatoni_stores::memory::MemoryStore;
let store = MemoryStore::new();  // No Redis needed!
```

See the `simple_pipeline_memory` example for a Redis-free setup.

**Start:**
```bash
docker compose -f docker-compose.minimal.yml up -d
```

**Access:**
- MongoDB: `mongodb://localhost:27017/?replicaSet=rs0&directConnection=true`
- Redis: `redis://:redispassword@localhost:6379`

**Note:** You'll need to configure real AWS credentials or another S3-compatible service.

---

### 3. docker-compose.localstack.yml

**LocalStack S3 only for testing S3 destinations.**

**Includes:**
- LocalStack (S3 emulation)
- AWS CLI helper (optional)

**When to use:**
- Testing S3 destination functionality in isolation
- Running `rigatoni-destinations` integration tests
- Already have MongoDB and Redis running elsewhere

**Start:**
```bash
docker compose -f docker-compose.localstack.yml up -d
```

**With AWS CLI helper:**
```bash
docker compose -f docker-compose.localstack.yml --profile helpers up -d
```

**Access:**
- LocalStack S3: `http://localhost:4566`

**Create buckets:**
```bash
awslocal s3 mb s3://test-bucket
```

**Documentation:** [rigatoni-destinations/README.md](../rigatoni-destinations/README.md)

---

### 4. docker-compose.observability.yml

**Prometheus and Grafana only for metrics visualization.**

**Includes:**
- Prometheus (metrics collection)
- Grafana (metrics visualization with pre-built dashboards)

**When to use:**
- Adding observability to existing infrastructure
- Testing metrics integration
- Already have MongoDB, Redis, and S3 running

**Start:**
```bash
docker compose -f docker-compose.observability.yml up -d
```

**Access:**
- Prometheus: `http://localhost:9090`
- Grafana: `http://localhost:3000` (admin/admin)

**Note:** Prometheus expects your Rigatoni app to expose metrics at `http://host.docker.internal:9000/metrics`

**Documentation:** [docs/OBSERVABILITY.md](../docs/OBSERVABILITY.md)

---

## Combining Compose Files

You can combine multiple compose files for custom setups:

### Example: Minimal + Observability
```bash
docker compose -f docker-compose.minimal.yml -f docker-compose.observability.yml up -d
```

### Example: Minimal + LocalStack
```bash
docker compose -f docker-compose.minimal.yml -f docker-compose.localstack.yml up -d
```

---

## Common Commands

### Start services
```bash
# Full stack
docker compose up -d

# Specific compose file
docker compose -f docker-compose.minimal.yml up -d

# With UI profile
docker compose --profile ui up -d
```

### View logs
```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f mongodb
docker compose logs -f prometheus
```

### Check status
```bash
docker compose ps
```

### Stop services
```bash
# Stop (keep data)
docker compose down

# Stop and remove data
docker compose down -v
```

### Restart a service
```bash
docker compose restart mongodb
docker compose restart prometheus
```

### Execute commands in containers
```bash
# MongoDB shell
docker exec -it rigatoni-mongodb mongosh

# Redis CLI
docker exec -it rigatoni-redis redis-cli -a redispassword

# Check LocalStack health
curl http://localhost:4566/_localstack/health
```

---

## Directory Structure

```
docker/
├── README.md                          # This file
├── docker-compose.yml                 # Full stack (recommended)
├── docker-compose.minimal.yml         # MongoDB + Redis only
├── docker-compose.localstack.yml      # LocalStack S3 only
├── docker-compose.observability.yml   # Prometheus + Grafana only
├── config/
│   ├── prometheus.yml                 # Prometheus configuration
│   └── grafana/
│       ├── datasources/               # Grafana datasource config
│       │   └── prometheus.yml
│       └── dashboards/                # Grafana dashboard config
│           └── rigatoni.yml
└── scripts/
    ├── init-mongo.sh                  # MongoDB replica set setup
    ├── init-localstack.sh             # LocalStack S3 bucket creation
    ├── setup-local-env.sh             # Environment variable setup
    ├── generate-test-data.sh          # Test data generator
    └── quick-start-local.sh           # One-command setup script
```

---

## Quick Start Scripts

### Complete Setup (Automated)
```bash
./scripts/quick-start-local.sh
```

This script:
1. Checks prerequisites (Docker, Rust, etc.)
2. Starts all services with `docker-compose.yml`
3. Waits for services to be healthy
4. Displays access URLs and next steps

### Manual Setup
```bash
# 1. Start services
docker compose up -d

# 2. Wait for MongoDB replica set (takes ~30 seconds)
docker compose logs -f mongodb-init

# 3. Set environment variables
source ./scripts/setup-local-env.sh

# 4. Run example
cargo run --example metrics_prometheus --features metrics-export

# 5. Generate test data (in another terminal)
./scripts/generate-test-data.sh
```

---

## Service Details

### MongoDB
- **Image:** mongo:7.0
- **Replica Set:** rs0 (required for change streams)
- **Port:** 27017
- **Auth:** No authentication (local development only)
- **Data:** Persisted in `mongodb_data` volume

### Redis
- **Image:** redis:7-alpine
- **Port:** 6379
- **Password:** redispassword
- **Persistence:** AOF enabled
- **Data:** Persisted in `redis_data` volume

### LocalStack
- **Image:** localstack/localstack:3.0
- **Services:** S3 only
- **Port:** 4566
- **Credentials:** test/test (dummy values)
- **Region:** us-east-1
- **Data:** Persisted in `localstack_data` volume

### Prometheus
- **Image:** prom/prometheus:v2.48.0
- **Port:** 9090
- **Retention:** 30 days
- **Scrape Interval:** 15 seconds
- **Targets:** `host.docker.internal:9000` (your Rigatoni app)
- **Data:** Persisted in `prometheus_data` volume

### Grafana
- **Image:** grafana/grafana:10.2.2
- **Port:** 3000
- **Login:** admin/admin
- **Dashboards:** Pre-provisioned Rigatoni dashboard
- **Datasource:** Prometheus (auto-configured)
- **Data:** Persisted in `grafana_data` volume

### MongoDB Express (Optional)
- **Image:** mongo-express:latest
- **Port:** 8081
- **Profile:** ui (requires `--profile ui`)

### Redis Commander (Optional)
- **Image:** rediscommander/redis-commander:latest
- **Port:** 8082
- **Profile:** ui (requires `--profile ui`)

---

## Environment Variables

The following environment variables can customize the setup:

```bash
# MongoDB
export MONGODB_URI="mongodb://localhost:27017/?replicaSet=rs0&directConnection=true"
export MONGODB_DATABASE="testdb"
export MONGODB_COLLECTIONS="users,orders,products"

# Redis
export REDIS_URL="redis://:redispassword@localhost:6379"

# AWS/LocalStack
export AWS_ACCESS_KEY_ID="test"
export AWS_SECRET_ACCESS_KEY="test"
export AWS_REGION="us-east-1"
export AWS_ENDPOINT_URL="http://localhost:4566"

# S3
export S3_BUCKET="rigatoni-test-bucket"
export S3_PREFIX="mongodb-cdc"

# Application
export RUST_LOG="info,rigatoni=debug"
```

Or use the setup script:
```bash
source ./scripts/setup-local-env.sh
```

---

## Troubleshooting

### MongoDB replica set not initialized

**Symptom:** Pipeline fails with "change streams are only supported on replica sets"

**Solution:**
```bash
# Check init logs
docker compose logs mongodb-init

# Manually initialize if needed
docker exec rigatoni-mongodb mongosh --eval "rs.initiate()"
```

### LocalStack not accessible

**Symptom:** S3 writes fail with connection errors

**Solution:**
```bash
# Check LocalStack health
curl http://localhost:4566/_localstack/health

# Check logs
docker compose logs localstack

# Restart if needed
docker compose restart localstack
```

### Prometheus not scraping metrics

**Symptom:** No data in Grafana

**Solution:**
1. Verify your app exposes metrics: `curl http://localhost:9000/metrics`
2. Check Prometheus targets: http://localhost:9090/targets
3. Verify Prometheus can reach host: `docker exec rigatoni-prometheus wget -qO- http://host.docker.internal:9000/metrics`

### Grafana dashboard not loading

**Symptom:** Dashboard shows "No data"

**Solution:**
1. Check Grafana datasource: Configuration → Data Sources → Prometheus → Test
2. Ensure Prometheus is running: http://localhost:9090
3. Check dashboard queries match your metrics

### Port conflicts

**Symptom:** "port is already allocated"

**Solution:** Stop conflicting services or edit port mappings in compose file:
```yaml
ports:
  - "27018:27017"  # Use different host port
```

### Out of disk space

**Symptom:** Services crash, Docker errors

**Solution:**
```bash
# Remove old volumes
docker compose down -v

# Clean Docker system
docker system prune -a
```

---

## Performance Tips

### Reduce Resource Usage
```bash
# Use minimal setup
docker compose -f docker-compose.minimal.yml up -d

# Disable UI tools (don't use --profile ui)
docker compose up -d
```

### Speed Up Startup
```bash
# Reduce health check intervals (edit compose file)
healthcheck:
  interval: 10s  # Increase from 5s
  retries: 3     # Reduce from 10
```

### Optimize for Testing
```bash
# Use tmpfs for faster I/O (lose data on restart)
docker compose -f docker-compose.minimal.yml up -d --force-recreate --renew-anon-volumes
```

---

## Migration from Old Structure

If you were using the old docker-compose files:

### From `tools/local-development/docker-compose.local.yml`
```bash
# Old
docker compose -f tools/local-development/docker-compose.local.yml up -d

# New
docker compose up -d
# or
docker compose -f docker/docker-compose.yml up -d
```

### From `rigatoni-destinations/docker-compose.yml`
```bash
# Old
cd rigatoni-destinations && docker-compose up -d

# New
docker compose -f docker/docker-compose.localstack.yml up -d
```

---

## Additional Resources

- **[Local Development Guide](../docs/guides/local-development.md)** - Complete tutorial
- **[Observability Guide](../docs/OBSERVABILITY.md)** - Metrics and monitoring
- **[Getting Started](../docs/getting-started.md)** - Build your first pipeline
- **[S3 Configuration](../docs/guides/s3-configuration.md)** - S3 destination setup
- **[Redis Configuration](../docs/guides/redis-configuration.md)** - Redis state store setup

---

## Contributing

When adding new services or modifying compose files:

1. Update this README
2. Add appropriate healthchecks
3. Use consistent naming (rigatoni-{service})
4. Use the `rigatoni-network` network
5. Add appropriate profiles for optional services
6. Document environment variables
7. Update related documentation

---

## Need Help?

- **Issues:** https://github.com/valeriouberti/rigatoni/issues
- **Documentation:** [docs/](../docs/)
- **Examples:** [rigatoni-core/examples/](../rigatoni-core/examples/)
