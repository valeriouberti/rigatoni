//! Simple Pipeline Example with In-Memory State Store
//!
//! This example demonstrates the simplest possible Rigatoni setup using
//! an in-memory state store. Perfect for local development and testing.
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
//! cargo run --example simple_pipeline_memory --features metrics-export
//! ```
//!
//! # Generate Test Data
//!
//! In another terminal:
//! ```bash
//! docker exec mongodb mongosh testdb --eval '
//!   db.users.insertOne({name: "Alice", email: "alice@example.com", age: 30})
//! '
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
                collection = %event.namespace.collection,
                operation = ?event.operation,
                "📦 Event received"
            );

            if let Some(doc) = &event.full_document {
                info!(document = ?doc, "   Document");
            }
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), DestinationError> {
        info!("✅ Batch flushed ({} events total)", self.event_count);
        Ok(())
    }

    async fn close(&mut self) -> Result<(), DestinationError> {
        info!(
            "👋 Destination closed ({} events processed)",
            self.event_count
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

    info!("🚀 Starting Rigatoni with In-Memory State Store");
    info!("");

    // Create in-memory state store (no configuration needed!)
    let store = MemoryStore::new();
    info!("✅ In-memory state store created (no external dependencies)");

    // Create simple console destination
    let destination = ConsoleDestination::new();
    info!("✅ Console destination created");

    // Configure pipeline
    info!("🔧 Configuring pipeline...");
    let config = PipelineConfig::builder()
        .mongodb_uri(std::env::var("MONGODB_URI").unwrap_or_else(|_| {
            "mongodb://localhost:27017/?replicaSet=rs0&directConnection=true".to_string()
        }))
        .database("testdb")
        .collections(vec!["users".to_string()])
        .batch_size(10)
        .batch_timeout(Duration::from_secs(5))
        .build()?;

    info!("✅ Pipeline configured");
    info!("");
    info!("📊 Configuration:");
    info!("   MongoDB: {}", config.mongodb_uri);
    info!("   Database: {}", config.database);
    info!("   Collections: {:?}", config.collections);
    info!("   Batch size: {}", config.batch_size);
    info!("   Batch timeout: {:?}", config.batch_timeout);
    info!("   State store: In-memory (ephemeral)");
    info!("");

    // Create pipeline
    info!("🎯 Creating pipeline...");
    let mut pipeline = Pipeline::new(config, store, destination).await?;
    info!("✅ Pipeline created successfully");
    info!("");

    // Display usage information
    display_usage();

    // Start the pipeline
    info!("▶️  Starting pipeline...");
    pipeline.start().await?;
    info!("✅ Pipeline running!");
    info!("");

    // Wait for Ctrl+C
    signal::ctrl_c().await?;

    info!("");
    info!("🛑 Received shutdown signal, stopping pipeline...");
    pipeline.stop().await?;
    info!("✅ Pipeline stopped gracefully");

    info!("👋 Example completed!");

    Ok(())
}

fn display_usage() {
    info!("💡 Usage:");
    info!("");
    info!("   1. The pipeline is now watching for MongoDB change events");
    info!("   2. Open another terminal and insert data:");
    info!("");
    info!("      docker exec mongodb mongosh testdb --eval '");
    info!("        db.users.insertOne({{");
    info!("          name: \"Alice\",");
    info!("          email: \"alice@example.com\",");
    info!("          age: 30");
    info!("        }})");
    info!("      '");
    info!("");
    info!("   3. Watch the events appear in this terminal!");
    info!("   4. Press Ctrl+C to stop gracefully");
    info!("");
    warn!("⚠️  Note: Using in-memory state store");
    warn!("   - Resume tokens are NOT persisted");
    warn!("   - Events will be replayed from the beginning on restart");
    warn!("   - Use RedisStore for production deployments");
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
