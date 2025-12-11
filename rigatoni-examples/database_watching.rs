//! Database-Level Watching Example
//!
//! This example demonstrates database-level watching, where Rigatoni monitors
//! all collections in a database automatically. New collections are included
//! as they are created.
//!
//! # Prerequisites
//!
//! Start MongoDB (replica set required for change streams):
//! ```bash
//! docker run -d --name mongodb -p 27017:27017 \
//!   mongo:7.0 --replSet rs0
//!
//! # Initialize replica set
//! docker exec mongodb mongosh --eval "rs.initiate()"
//! ```
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example database_watching --features metrics-export
//! ```
//!
//! # Generate Test Data
//!
//! In another terminal, insert data into any collection:
//! ```bash
//! # Insert into users collection
//! docker exec mongodb mongosh testdb --eval '
//!   db.users.insertOne({name: "Alice", email: "alice@example.com"})
//! '
//!
//! # Create a new collection and insert data
//! docker exec mongodb mongosh testdb --eval '
//!   db.orders.insertOne({product: "Widget", quantity: 5})
//! '
//!
//! # The pipeline will automatically capture events from all collections!
//! ```

use rigatoni_core::destination::{Destination, DestinationError, DestinationMetadata};
use rigatoni_core::event::ChangeEvent;
use rigatoni_core::pipeline::{Pipeline, PipelineConfig};
use rigatoni_stores::memory::MemoryStore;
use std::error::Error;
use std::time::Duration;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

/// Simple console destination that just prints events
#[derive(Debug)]
struct ConsoleDestination {
    event_count: usize,
}

impl ConsoleDestination {
    fn new() -> Self {
        Self { event_count: 0 }
    }
}

#[async_trait::async_trait]
impl Destination for ConsoleDestination {
    async fn write_batch(&mut self, events: &[ChangeEvent]) -> Result<(), DestinationError> {
        for event in events {
            self.event_count += 1;
            info!(
                count = self.event_count,
                database = %event.namespace.database,
                collection = %event.namespace.collection,
                operation = ?event.operation,
                "Event received from database watch"
            );

            if let Some(doc) = &event.full_document {
                info!(document = ?doc, "   Document");
            }
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), DestinationError> {
        info!("Batch flushed ({} events total)", self.event_count);
        Ok(())
    }

    async fn close(&mut self) -> Result<(), DestinationError> {
        info!("Destination closed ({} events processed)", self.event_count);
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

    info!("Starting Rigatoni with Database-Level Watching");
    info!("");

    // Create in-memory state store
    let store = MemoryStore::new();
    info!("In-memory state store created");

    // Create console destination
    let destination = ConsoleDestination::new();
    info!("Console destination created");

    // Configure pipeline with database-level watching
    info!("Configuring pipeline...");
    let config = PipelineConfig::builder()
        .mongodb_uri(std::env::var("MONGODB_URI").unwrap_or_else(|_| {
            "mongodb://localhost:27017/?replicaSet=rs0&directConnection=true".to_string()
        }))
        .database("testdb")
        .watch_database() // Watch all collections in the database!
        .batch_size(10)
        .batch_timeout(Duration::from_secs(5))
        .build()?;

    info!("Pipeline configured");
    info!("");
    info!("Configuration:");
    info!("   MongoDB: {}", config.mongodb_uri);
    info!("   Database: {}", config.database);
    info!("   Watch level: {} (all collections)", config.watch_level);
    info!("   Batch size: {}", config.batch_size);
    info!("   Batch timeout: {:?}", config.batch_timeout);
    info!("");

    // Create pipeline
    info!("Creating pipeline...");
    let mut pipeline = Pipeline::new(config, store, destination).await?;
    info!("Pipeline created successfully");
    info!("");

    // Display usage information
    display_usage();

    // Start the pipeline
    info!("Starting pipeline...");
    pipeline.start().await?;
    info!("Pipeline running!");
    info!("");

    // Wait for Ctrl+C
    signal::ctrl_c().await?;

    info!("");
    info!("Received shutdown signal, stopping pipeline...");
    pipeline.stop().await?;
    info!("Pipeline stopped gracefully");

    info!("Example completed!");

    Ok(())
}

fn display_usage() {
    info!("Usage:");
    info!("");
    info!("   1. The pipeline is now watching ALL collections in 'testdb'");
    info!("   2. Open another terminal and insert data into ANY collection:");
    info!("");
    info!("      # Insert into users collection");
    info!("      docker exec mongodb mongosh testdb --eval '");
    info!("        db.users.insertOne({{name: \"Alice\"}})");
    info!("      '");
    info!("");
    info!("      # Create a NEW collection and insert (automatically detected!)");
    info!("      docker exec mongodb mongosh testdb --eval '");
    info!("        db.orders.insertOne({{product: \"Widget\"}})");
    info!("      '");
    info!("");
    info!("   3. Watch the events appear in this terminal!");
    info!("   4. Press Ctrl+C to stop gracefully");
    info!("");
    warn!("Note: Using in-memory state store - resume tokens NOT persisted");
    info!("");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("");
}

/// Initialize structured logging
fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,rigatoni_core=info,rigatoni_stores=info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();
}
