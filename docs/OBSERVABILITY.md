# Rigatoni Observability Guide

This guide explains how to monitor and troubleshoot Rigatoni pipelines in production using metrics, logging, and distributed tracing.

## Table of Contents

- [Overview](#overview)
- [Metrics](#metrics)
- [Prometheus Integration](#prometheus-integration)
- [Grafana Dashboards](#grafana-dashboards)
- [Alerting](#alerting)
- [Performance Tuning](#performance-tuning)
- [Troubleshooting](#troubleshooting)

## Overview

Rigatoni provides comprehensive observability through three pillars:

1. **Metrics**: Quantitative measurements using the `metrics` crate and Prometheus
2. **Logging**: Structured logging with `tracing`
3. **Health Checks**: Pipeline status and health endpoints

## Metrics

### Metric Types

Rigatoni uses three types of metrics:

#### Counters (Monotonically Increasing)

- `rigatoni_events_processed_total` - Total events successfully processed
- `rigatoni_events_failed_total` - Total events that failed processing
- `rigatoni_retries_total` - Total retry attempts
- `rigatoni_batches_written_total` - Total batches written to destinations
- `rigatoni_destination_write_errors_total` - Total destination write errors

#### Histograms (Value Distributions)

- `rigatoni_batch_size` - Distribution of batch sizes
- `rigatoni_batch_duration_seconds` - Time to process batches
- `rigatoni_destination_write_duration_seconds` - Destination write latency
- `rigatoni_destination_write_bytes` - Size of data written
- `rigatoni_change_stream_lag_seconds` - Change stream lag

#### Gauges (Point-in-Time Values)

- `rigatoni_active_collections` - Number of monitored collections
- `rigatoni_pipeline_status` - Pipeline status (0=stopped, 1=running, 2=error)
- `rigatoni_batch_queue_size` - Events buffered awaiting write

### Metric Labels

All metrics include relevant labels for filtering and aggregation:

- `collection`: MongoDB collection name
- `destination_type`: Destination type (s3, bigquery, kafka)
- `operation`: Operation type (insert, update, delete)
- `error_type`: Error category (timeout_error, connection_error, etc.)

### Cardinality Considerations

⚠️ **Important**: Labels increase cardinality, which affects Prometheus performance and storage.

**Safe Labels** (low cardinality, <100 unique values):
- collection names
- destination types
- operation types
- error categories

**Dangerous Labels** (avoid these):
- Document IDs
- Timestamps
- User IDs
- Full error messages
- IP addresses

## Prometheus Integration

### Setup

1. **Enable metrics in your pipeline**:

```rust
use rigatoni_core::metrics;
use metrics_exporter_prometheus::PrometheusBuilder;

// Initialize metrics
metrics::init_metrics();

// Start Prometheus exporter
let addr = ([0, 0, 0, 0], 9000).into();
PrometheusBuilder::new()
    .with_http_listener(addr)
    .install()
    .expect("Failed to install Prometheus exporter");
```

2. **Configure Prometheus** (`prometheus.yml`):

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'rigatoni'
    static_configs:
      - targets: ['localhost:9000']
    metric_relabel_configs:
      # Drop high-cardinality labels if needed
      - source_labels: [__name__]
        regex: 'rigatoni_.*'
        action: keep
```

3. **Run the metrics example**:

```bash
cargo run --example metrics_prometheus --features metrics-export
```

4. **Verify metrics**:

```bash
curl http://localhost:9000/metrics | grep rigatoni_
```

### Useful PromQL Queries

#### Throughput

```promql
# Events processed per second
rate(rigatoni_events_processed_total[5m])

# Events processed per second by collection
sum by (collection) (rate(rigatoni_events_processed_total[5m]))

# Total throughput across all collections
sum(rate(rigatoni_events_processed_total[5m]))
```

#### Latency

```promql
# 50th percentile write latency
histogram_quantile(0.50, rate(rigatoni_destination_write_duration_seconds_bucket[5m]))

# 95th percentile write latency
histogram_quantile(0.95, rate(rigatoni_destination_write_duration_seconds_bucket[5m]))

# 99th percentile write latency
histogram_quantile(0.99, rate(rigatoni_destination_write_duration_seconds_bucket[5m]))

# Average batch processing time
rate(rigatoni_batch_duration_seconds_sum[5m]) / rate(rigatoni_batch_duration_seconds_count[5m])
```

#### Error Rates

```promql
# Error rate (events/sec)
rate(rigatoni_events_failed_total[5m])

# Error rate by type
sum by (error_type) (rate(rigatoni_events_failed_total[5m]))

# Error percentage
100 * (
  rate(rigatoni_events_failed_total[5m]) /
  (rate(rigatoni_events_processed_total[5m]) + rate(rigatoni_events_failed_total[5m]))
)
```

#### Queue Depth

```promql
# Current queue size
rigatoni_batch_queue_size

# Queue size by collection
sum by (collection) (rigatoni_batch_queue_size)

# Queue growth rate (positive = filling up, negative = draining)
deriv(rigatoni_batch_queue_size[5m])
```

#### Data Volume

```promql
# Bytes written per second
rate(rigatoni_destination_write_bytes_sum[5m])

# Megabytes written per hour
rate(rigatoni_destination_write_bytes_sum[1h]) / 1024 / 1024

# Average batch size
rigatoni_batch_size_sum / rigatoni_batch_size_count
```

## Grafana Dashboards

### Importing the Dashboard

1. Navigate to Grafana → Dashboards → Import
2. Upload `docs/grafana/rigatoni-dashboard.json`
3. Select your Prometheus datasource
4. Click Import

### Dashboard Panels

#### Overview Row
- **Pipeline Status**: Current pipeline state (running/stopped/error)
- **Active Collections**: Number of collections being monitored
- **Events Processed**: Real-time throughput graph
- **Error Rate**: Failed events over time

#### Performance Row
- **Batch Size Distribution**: Heatmap showing batch size patterns
- **Write Latency Percentiles**: p50, p95, p99 latencies
- **Batch Processing Time**: Time from event receipt to destination write
- **Queue Depth**: Events waiting to be written

#### Health Row
- **Retry Rate**: Retry attempts over time
- **Destination Errors**: Errors by destination type
- **Data Written**: Bytes/sec written to destinations
- **Change Stream Lag**: Delay between MongoDB operation and event receipt

### Custom Queries

Add custom panels with these queries:

**Success Rate**:
```promql
100 * (
  rate(rigatoni_events_processed_total[5m]) /
  (rate(rigatoni_events_processed_total[5m]) + rate(rigatoni_events_failed_total[5m]))
)
```

**Average Batch Size by Collection**:
```promql
avg by (collection) (rigatoni_batch_size)
```

**Destination Write Success Rate**:
```promql
100 * (
  rate(rigatoni_batches_written_total[5m]) /
  (rate(rigatoni_batches_written_total[5m]) + rate(rigatoni_destination_write_errors_total[5m]))
)
```

## Alerting

### Recommended Alerts

#### Critical Alerts

**Pipeline Down**:
```yaml
- alert: RigatoniPipelineDown
  expr: rigatoni_pipeline_status != 1
  for: 1m
  labels:
    severity: critical
  annotations:
    summary: "Rigatoni pipeline is not running"
    description: "Pipeline status is {{ $value }} (expected 1=running)"
```

**High Error Rate**:
```yaml
- alert: RigatoniHighErrorRate
  expr: |
    (
      rate(rigatoni_events_failed_total[5m]) /
      (rate(rigatoni_events_processed_total[5m]) + rate(rigatoni_events_failed_total[5m]))
    ) > 0.05
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "High error rate in Rigatoni pipeline"
    description: "Error rate is {{ $value | humanizePercentage }} (threshold: 5%)"
```

**No Events Processed**:
```yaml
- alert: RigatoniNoEventsProcessed
  expr: rate(rigatoni_events_processed_total[10m]) == 0
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "Rigatoni pipeline not processing events"
    description: "No events processed in the last 10 minutes"
```

#### Warning Alerts

**High Write Latency**:
```yaml
- alert: RigatoniHighWriteLatency
  expr: |
    histogram_quantile(0.99,
      rate(rigatoni_destination_write_duration_seconds_bucket[5m])
    ) > 5
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "High destination write latency"
    description: "P99 write latency is {{ $value }}s (threshold: 5s)"
```

**Queue Growing**:
```yaml
- alert: RigatoniQueueGrowing
  expr: deriv(rigatoni_batch_queue_size[10m]) > 10
  for: 15m
  labels:
    severity: warning
  annotations:
    summary: "Batch queue growing"
    description: "Queue is growing at {{ $value }} events/min for collection {{ $labels.collection }}"
```

**High Retry Rate**:
```yaml
- alert: RigatoniHighRetryRate
  expr: rate(rigatoni_retries_total[5m]) > 1
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "High retry rate"
    description: "Retry rate is {{ $value }} retries/sec for {{ $labels.error_type }}"
```

### Alert Routing

Configure Alertmanager to route alerts based on severity:

```yaml
route:
  group_by: ['alertname', 'collection']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: 'default'
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty'
    - match:
        severity: warning
      receiver: 'slack'

receivers:
  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: '<your-key>'
  - name: 'slack'
    slack_configs:
      - api_url: '<your-webhook>'
        channel: '#rigatoni-alerts'
```

## Performance Tuning

### Identifying Bottlenecks

#### 1. High Queue Depth

**Symptoms**:
- `rigatoni_batch_queue_size` continuously growing
- `deriv(rigatoni_batch_queue_size[10m]) > 0`

**Causes**:
- Destination writes too slow
- Batch size too large
- Network issues

**Solutions**:
- Reduce `batch_size` in config
- Increase `batch_timeout` to flush faster
- Scale destination (e.g., more S3 parallel uploads)
- Check destination performance

#### 2. High Write Latency

**Symptoms**:
- `histogram_quantile(0.99, rigatoni_destination_write_duration_seconds_bucket) > 5`

**Causes**:
- Destination overloaded
- Large batch sizes
- Network latency

**Solutions**:
- Reduce batch size
- Enable compression (if not already)
- Use faster destination (e.g., S3 over HTTP/2)
- Check network connectivity

#### 3. High Error Rate

**Symptoms**:
- `rate(rigatoni_events_failed_total[5m]) > threshold`

**Causes**:
- Serialization errors
- Destination connectivity issues
- Permiss ion errors
- Rate limiting

**Solutions**:
- Check error types: `sum by (error_type) (rate(rigatoni_events_failed_total[5m]))`
- Fix serialization issues
- Verify credentials
- Implement rate limiting backoff
- Increase `max_retries`

#### 4. High Retry Rate

**Symptoms**:
- `rate(rigatoni_retries_total[5m]) > 1`

**Causes**:
- Transient network errors
- Destination throttling
- Timeout too aggressive

**Solutions**:
- Check retry reasons: `sum by (error_type) (rate(rigatoni_retries_total[5m]))`
- Increase retry delay
- Implement exponential backoff
- Contact destination support if persistent

### Optimization Strategies

#### Batch Size Tuning

Monitor `rigatoni_batch_size` histogram:

```promql
# Show batch size distribution
histogram_quantile(0.50, rate(rigatoni_batch_size_bucket[5m]))  # median
histogram_quantile(0.95, rate(rigatoni_batch_size_bucket[5m]))  # p95
```

**Guidelines**:
- **Too small** (<10 events): Overhead from frequent writes
- **Optimal** (50-200 events): Good balance
- **Too large** (>500 events): High latency, memory usage

#### Memory Usage

Monitor queue size to estimate memory:

```promql
# Approximate memory usage (assuming 1KB per event)
sum(rigatoni_batch_queue_size) * 1024
```

**Guidelines**:
- Keep queue size < 10,000 events per collection
- Configure `batch_timeout` to prevent unbounded growth
- Scale horizontally if needed

## Troubleshooting

### Common Issues

#### Pipeline Not Starting

**Check**:
1. Pipeline status: `rigatoni_pipeline_status` (should be 1)
2. Logs for error messages
3. MongoDB connectivity
4. Redis connectivity

**Debug**:
```bash
# Check if metrics endpoint is accessible
curl http://localhost:9000/metrics

# Look for initialization errors in logs
grep "ERROR" pipeline.log | grep -i "start\|init"
```

#### No Events Being Processed

**Check**:
1. MongoDB change stream: Are there actual changes?
2. Collection configuration: Are correct collections monitored?
3. Resume token: Is pipeline stuck on old token?

**Debug**:
```promql
# Should be > 0 if events are flowing
rate(rigatoni_events_processed_total[5m])

# Check if change stream is receiving events
rate(rigatoni_change_stream_lag_seconds_count[5m])
```

#### Destination Writes Failing

**Check**:
1. Destination errors: `rigatoni_destination_write_errors_total`
2. Error types: `sum by (error_type, destination_type) (rate(rigatoni_destination_write_errors_total[5m]))`
3. Credentials and permissions

**Debug**:
```bash
# Test destination connectivity manually
awslocal s3 ls s3://your-bucket/  # for S3
# or
bq ls your-dataset  # for BigQuery
```

#### High Memory Usage

**Check**:
1. Queue sizes: `sum(rigatoni_batch_queue_size)`
2. Batch sizes: `histogram_quantile(0.99, rate(rigatoni_batch_size_bucket[5m]))`

**Solutions**:
- Reduce `batch_size`
- Decrease `batch_timeout`
- Add backpressure limits

### Performance Degradation

If throughput decreases over time:

```promql
# Compare current vs historical throughput
rate(rigatoni_events_processed_total[5m])  # current
rate(rigatoni_events_processed_total[5m] offset 1h)  # 1 hour ago
```

**Possible Causes**:
1. Memory pressure → check queue size
2. Destination throttling → check error rates
3. MongoDB replication lag → check change stream lag
4. Network issues → check write latency

## Best Practices

### 1. Set Retention Policies

Configure Prometheus retention:

```yaml
# In prometheus.yml
storage:
  tsdb:
    retention.time: 30d
    retention.size: 50GB
```

### 2. Use Recording Rules

Pre-compute expensive queries:

```yaml
# In prometheus.yml
groups:
  - name: rigatoni
    interval: 30s
    rules:
      - record: rigatoni:throughput:rate5m
        expr: rate(rigatoni_events_processed_total[5m])

      - record: rigatoni:error_rate:rate5m
        expr: |
          rate(rigatoni_events_failed_total[5m]) /
          (rate(rigatoni_events_processed_total[5m]) + rate(rigatoni_events_failed_total[5m]))

      - record: rigatoni:write_latency:p99
        expr: histogram_quantile(0.99, rate(rigatoni_destination_write_duration_seconds_bucket[5m]))
```

### 3. Implement Dashboards for Each Team

- **Operations**: Pipeline health, errors, throughput
- **Development**: Latency distributions, queue depths
- **Business**: Data volume, collection statistics

### 4. Regular Review

Schedule weekly reviews of:
- Alert frequency and accuracy
- Dashboard usage
- Metric cardinality
- Performance trends

### 5. Documentation

Document your specific:
- Alert thresholds and rationale
- Expected throughput ranges
- Maintenance procedures
- Escalation procedures

## Additional Resources

- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [Grafana Documentation](https://grafana.com/docs/)
- [PromQL Cheat Sheet](https://promlabs.com/promql-cheat-sheet/)
- [Metrics Crate Documentation](https://docs.rs/metrics/)
