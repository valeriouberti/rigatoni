// Copyright 2025 Rigatoni Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Multi-Instance Pipeline Example with Distributed Locking
//!
//! This example demonstrates how to run multiple Rigatoni instances watching
//! the same MongoDB collections with Redis-based distributed locking to prevent
//! duplicate event processing.
//!
//! # How It Works
//!
//! 1. Each instance generates a unique owner ID (hostname + UUID)
//! 2. Before starting to process a collection, the instance acquires a lock in Redis
//! 3. Only the lock holder processes events for that collection
//! 4. If an instance crashes, its lock expires after TTL (default 30s)
//! 5. Other instances automatically take over orphaned collections
//!
//! # Prerequisites
//!
//! Start MongoDB (replica set required for change streams):
//! ```bash
//! docker run -d --name mongodb -p 27017:27017 \
//!   mongo:7.0 --replSet rs0
//!
//! docker exec mongodb mongosh --eval "rs.initiate()"
//! ```
//!
//! Start Redis:
//! ```bash
//! docker run -d --name redis -p 6379:6379 redis:7.0
//! ```
//!
//! # Running Multiple Instances
//!
//! Terminal 1:
//! ```bash
//! INSTANCE_NAME=instance-1 cargo run --example multi_instance_redis
//! ```
//!
//! Terminal 2:
//! ```bash
//! INSTANCE_NAME=instance-2 cargo run --example multi_instance_redis
//! ```
//!
//! Terminal 3:
//! ```bash
//! INSTANCE_NAME=instance-3 cargo run --example multi_instance_redis
//! ```
//!
//! # Observing Lock Distribution
//!
//! When you start multiple instances, you'll see output like:
//! - Instance 1: "Acquired lock for collection 'users', 'orders'"
//! - Instance 2: "Acquired lock for collection 'products'"
//! - Instance 3: "Collection 'users' is locked by another instance, skipping"
//!
//! # Testing Failover
//!
//! 1. Start 2-3 instances
//! 2. Observe which instance owns which collections
//! 3. Kill one instance (Ctrl+C)
//! 4. Wait for TTL (30s) - other instances will acquire the orphaned locks
//!
//! # Generate Test Data
//!
//! ```bash
//! docker exec mongodb mongosh testdb --eval '
//!   db.users.insertOne({name: "Alice", timestamp: new Date()});
//!   db.orders.insertOne({product: "Widget", quantity: 5, timestamp: new Date()});
//!   db.products.insertOne({name: "Gadget", price: 29.99, timestamp: new Date()});
//! '
//! ```

use rigatoni_core::destination::{Destination, DestinationError, DestinationMetadata};
use rigatoni_core::event::ChangeEvent;
use rigatoni_core::pipeline::{DistributedLockConfig, Pipeline, PipelineConfig};
use rigatoni_stores::redis::{RedisConfig, RedisStore};
use std::error::Error;
use std::time::Duration;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

/// Simple console destination that logs events with instance information
#[derive(Debug)]
struct ConsoleDestination {
    instance_name: String,
    event_count: usize,
}

impl ConsoleDestination {
    fn new(instance_name: &str) -> Self {
        Self {
            instance_name: instance_name.to_string(),
            event_count: 0,
        }
    }
}

#[async_trait::async_trait]
impl Destination for ConsoleDestination {
    async fn write_batch(&mut self, events: &[ChangeEvent]) -> Result<(), DestinationError> {
        for event in events {
            self.event_count += 1;
            info!(
                instance = %self.instance_name,
                count = self.event_count,
                collection = %event.namespace.collection,
                operation = ?event.operation,
                "Event received"
            );

            if let Some(doc) = &event.full_document {
                info!(instance = %self.instance_name, document = ?doc, "   Document");
            }
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), DestinationError> {
        info!(
            instance = %self.instance_name,
            total_events = self.event_count,
            "Batch flushed"
        );
        Ok(())
    }

    async fn close(&mut self) -> Result<(), DestinationError> {
        info!(
            instance = %self.instance_name,
            total_events = self.event_count,
            "Destination closed"
        );
        Ok(())
    }

    fn supports_transactions(&self) -> bool {
        false
    }

    fn metadata(&self) -> DestinationMetadata {
        DestinationMetadata::new("Console", "console")
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    init_logging();

    // Get instance name from environment (for demonstration)
    let instance_name =
        std::env::var("INSTANCE_NAME").unwrap_or_else(|_| "instance-default".to_string());

    info!("");
    info!("====================================================================");
    info!("    Rigatoni Multi-Instance Example with Distributed Locking");
    info!("====================================================================");
    info!("");
    info!("Instance: {}", instance_name);
    info!("");

    // Create Redis state store
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    info!("Connecting to Redis at: {}", redis_url);
    let redis_config = RedisConfig::builder()
        .url(&redis_url)
        .pool_size(5)
        .ttl(Duration::from_secs(7 * 24 * 60 * 60)) // 7 days for resume tokens
        .build()?;

    let store = RedisStore::new(redis_config).await?;
    info!("Redis connection established");

    // Create destination
    let destination = ConsoleDestination::new(&instance_name);

    // Configure pipeline with distributed locking
    let mongodb_uri = std::env::var("MONGODB_URI")
        .unwrap_or_else(|_| "mongodb://localhost:27017/?replicaSet=rs0".to_string());

    info!("");
    info!("Configuring pipeline with distributed locking...");

    let config = PipelineConfig::builder()
        .mongodb_uri(&mongodb_uri)
        .database("testdb")
        // Watch multiple collections - each protected by its own lock
        .watch_collections(vec![
            "users".to_string(),
            "orders".to_string(),
            "products".to_string(),
        ])
        .batch_size(10)
        .batch_timeout(Duration::from_secs(5))
        // Configure distributed locking
        .distributed_lock(DistributedLockConfig {
            enabled: true,
            ttl: Duration::from_secs(30), // Lock expires after 30s if not refreshed
            refresh_interval: Duration::from_secs(10), // Refresh lock every 10s
            retry_interval: Duration::from_secs(5), // Retry acquiring locks every 5s
        })
        .build()?;

    info!("");
    info!("Configuration:");
    info!("  MongoDB URI: {}", mongodb_uri);
    info!("  Database: testdb");
    info!("  Collections: users, orders, products");
    info!("  Distributed Lock: ENABLED");
    info!("    - TTL: 30s (lock expires if holder crashes)");
    info!("    - Refresh: 10s (heartbeat interval)");
    info!("    - Retry: 5s (attempt to acquire unowned locks)");
    info!("");

    // Create pipeline
    info!("Creating pipeline...");
    let mut pipeline = Pipeline::new(config, store, destination).await?;

    info!("Pipeline owner ID: {}", pipeline.owner_id());
    info!("");

    // Display usage information
    display_usage(&instance_name);

    // Start the pipeline
    // This will:
    // 1. Try to acquire locks for each collection
    // 2. Start workers only for collections we successfully locked
    // 3. Start lock refresh background tasks
    info!("Starting pipeline (acquiring locks)...");
    pipeline.start().await?;

    info!("");
    info!("Pipeline is running!");
    info!("This instance is processing events for locked collections.");
    info!("");

    // Wait for Ctrl+C
    info!("Press Ctrl+C to stop gracefully (locks will be released)");
    info!("");
    info!("============================================================");
    info!("");

    signal::ctrl_c().await?;

    info!("");
    info!("============================================================");
    info!("Received shutdown signal");
    info!("");
    info!("Stopping pipeline and releasing locks...");

    // Stop the pipeline
    // This will:
    // 1. Stop all workers gracefully
    // 2. Flush pending batches
    // 3. Release all held locks
    // 4. Close connections
    pipeline.stop().await?;

    info!("Pipeline stopped. Locks released.");
    info!("Other instances can now acquire the released collections.");
    info!("");
    info!("Goodbye from {}!", instance_name);

    Ok(())
}

fn display_usage(instance_name: &str) {
    info!("============================================================");
    info!("MULTI-INSTANCE DEMONSTRATION");
    info!("============================================================");
    info!("");
    info!("To run multiple instances:");
    info!("");
    info!("  Terminal 1: INSTANCE_NAME=instance-1 cargo run --example multi_instance_redis");
    info!("  Terminal 2: INSTANCE_NAME=instance-2 cargo run --example multi_instance_redis");
    info!("  Terminal 3: INSTANCE_NAME=instance-3 cargo run --example multi_instance_redis");
    info!("");
    info!("Watch how collections are distributed:");
    info!("  - Each collection is owned by exactly ONE instance");
    info!("  - Other instances skip collections they can't lock");
    info!("  - If an instance dies, others take over after 30s TTL");
    info!("");
    info!("Generate test data:");
    info!("");
    info!("  docker exec mongodb mongosh testdb --eval '");
    info!("    db.users.insertOne({{name: \"Alice\", timestamp: new Date()}});");
    info!("    db.orders.insertOne({{product: \"Widget\", timestamp: new Date()}});");
    info!("    db.products.insertOne({{name: \"Gadget\", timestamp: new Date()}});");
    info!("  '");
    info!("");
    info!("Current instance: {}", instance_name);
    info!("============================================================");
    info!("");
}

/// Initialize structured logging
fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,rigatoni_core=info,rigatoni_stores=info,rigatoni_core::pipeline=debug")
    });

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();
}
