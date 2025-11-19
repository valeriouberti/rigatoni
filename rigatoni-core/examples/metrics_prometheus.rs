//! Prometheus Metrics Exporter Example
//!
//! This example demonstrates how to expose Rigatoni pipeline metrics
//! to Prometheus for monitoring and alerting.
//!
//! # Prerequisites
//!
//! Start the local development stack:
//! ```bash
//! docker-compose -f docker-compose.local.yml up -d
//! ```
//!
//! # Environment Setup
//!
//! ```bash
//! source ./scripts/setup-local-env.sh
//! ```
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example metrics_prometheus --features metrics-export
//! ```
//!
//! # Viewing Metrics
//!
//! While the pipeline is running, metrics are exposed at:
//! - http://localhost:9000/metrics
//!
//! You can query them with curl:
//! ```bash
//! curl http://localhost:9000/metrics
//! ```
//!
//! # Prometheus Configuration
//!
//! Add this to your `prometheus.yml`:
//! ```yaml
//! scrape_configs:
//!   - job_name: 'rigatoni'
//!     static_configs:
//!       - targets: ['localhost:9000']
//! ```

use metrics_exporter_prometheus::PrometheusBuilder;
use rigatoni_core::metrics;
use rigatoni_core::pipeline::{Pipeline, PipelineConfig};
use rigatoni_destinations::s3::{
    Compression, KeyGenerationStrategy, S3Config, S3Destination, SerializationFormat,
};
use rigatoni_stores::redis::{RedisConfig, RedisStore};
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    init_logging();

    info!("🚀 Starting Rigatoni with Prometheus Metrics Exporter");

    // Initialize metrics descriptions
    metrics::init_metrics();

    // Start Prometheus exporter on port 9000
    let prometheus_addr: SocketAddr = ([0, 0, 0, 0], 9000).into();

    info!(
        "📊 Starting Prometheus exporter on http://{}",
        prometheus_addr
    );

    PrometheusBuilder::new()
        .with_http_listener(prometheus_addr)
        .install()
        .expect("Failed to install Prometheus exporter");

    info!("✅ Prometheus metrics available at http://localhost:9000/metrics");
    info!("");

    // Configure Redis state store
    info!("🔧 Configuring Redis state store...");
    let redis_config = RedisConfig::builder()
        .url(
            std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://:redispassword@localhost:6379".to_string()),
        )
        .pool_size(10)
        .ttl(Duration::from_secs(24 * 60 * 60))
        .max_retries(3)
        .build()?;

    let store = match RedisStore::new(redis_config).await {
        Ok(s) => {
            info!("✅ Redis connection established");
            s
        }
        Err(e) => {
            error!("❌ Failed to connect to Redis: {}", e);
            return Err(e.into());
        }
    };

    // Configure S3 destination
    info!("🔧 Configuring S3 destination...");
    let s3_config = S3Config::builder()
        .bucket("rigatoni-test-bucket")
        .region("us-east-1")
        .prefix("mongodb-cdc/metrics-demo")
        .endpoint_url("http://localhost:4566")
        .force_path_style(true)
        .format(SerializationFormat::Json)
        .compression(Compression::Gzip)
        .key_strategy(KeyGenerationStrategy::DatePartitioned)
        .build()?;

    let destination = match S3Destination::new(s3_config).await {
        Ok(d) => {
            info!("✅ S3 destination configured");
            d
        }
        Err(e) => {
            error!("❌ Failed to configure S3: {}", e);
            return Err(e.into());
        }
    };

    // Configure pipeline
    info!("🔧 Configuring Rigatoni pipeline...");
    let pipeline_config = PipelineConfig::builder()
        .mongodb_uri(std::env::var("MONGODB_URI").unwrap_or_else(|_| {
            "mongodb://localhost:27017/?replicaSet=rs0&directConnection=true".to_string()
        }))
        .database("testdb")
        .collections(vec![
            "users".to_string(),
            "orders".to_string(),
            "products".to_string(),
        ])
        .batch_size(50)
        .batch_timeout(Duration::from_secs(10))
        .max_retries(3)
        .build()?;

    // Create and start pipeline
    info!("🎯 Creating pipeline...");
    let mut pipeline = Pipeline::new(pipeline_config, store, destination).await?;

    info!("✅ Pipeline created successfully");
    info!("");
    info!("📊 Metrics Information:");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("");
    info!("🔗 Prometheus Endpoint:");
    info!("   http://localhost:9000/metrics");
    info!("");
    info!("📈 Available Metrics:");
    info!("   Counters:");
    info!("   - rigatoni_events_processed_total");
    info!("   - rigatoni_events_failed_total");
    info!("   - rigatoni_retries_total");
    info!("   - rigatoni_batches_written_total");
    info!("");
    info!("   Histograms:");
    info!("   - rigatoni_batch_size");
    info!("   - rigatoni_batch_duration_seconds");
    info!("   - rigatoni_destination_write_duration_seconds");
    info!("   - rigatoni_destination_write_bytes");
    info!("");
    info!("   Gauges:");
    info!("   - rigatoni_active_collections");
    info!("   - rigatoni_pipeline_status");
    info!("   - rigatoni_batch_queue_size");
    info!("");
    info!("💡 Query Examples:");
    info!("   View all metrics:");
    info!("   curl http://localhost:9000/metrics");
    info!("");
    info!("   Events processed per second (PromQL):");
    info!("   rate(rigatoni_events_processed_total[5m])");
    info!("");
    info!("   Average batch size (PromQL):");
    info!("   avg(rigatoni_batch_size)");
    info!("");
    info!("   99th percentile write latency (PromQL):");
    info!("   histogram_quantile(0.99, rigatoni_destination_write_duration_seconds_bucket)");
    info!("");
    info!("🔍 Generate Test Data:");
    info!("   In another terminal:");
    info!("   ./scripts/generate-test-data.sh");
    info!("");
    info!("🛑 Press Ctrl+C to stop gracefully...");
    info!("");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("");

    // Start the pipeline
    pipeline.start().await?;

    // Wait for Ctrl+C
    signal::ctrl_c().await?;

    info!("");
    info!("🛑 Received shutdown signal, stopping pipeline...");
    pipeline.stop().await?;
    info!("✅ Pipeline stopped gracefully");

    // Keep the metrics endpoint alive for a few seconds to allow final scrape
    info!("⏳ Keeping metrics endpoint alive for final scrape (5 seconds)...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    info!("👋 Example completed!");

    Ok(())
}

/// Initialize structured logging
fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,rigatoni_core=info,rigatoni_destinations=info,rigatoni_stores=info")
    });

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_level(true)
        .with_line_number(true)
        .init();
}
