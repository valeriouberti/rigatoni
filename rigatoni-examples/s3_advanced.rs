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

//! Advanced S3 destination example.
//!
//! This example demonstrates advanced features including:
//! - Different serialization formats (JSON, CSV, Parquet, Avro)
//! - Hive-style partitioning for analytics
//! - Compression options
//! - Custom retry configuration
//!
//! # Prerequisites
//!
//! - AWS credentials configured
//! - An S3 bucket you have write access to
//!
//! # Running
//!
//! ## With AWS S3
//!
//! ```bash
//! # With all features
//! cargo run --example s3_advanced --all-features
//!
//! # With specific format
//! cargo run --example s3_advanced --features s3,csv,gzip
//! cargo run --example s3_advanced --features s3,parquet,zstandard
//! cargo run --example s3_advanced --features s3,avro
//! ```
//!
//! ## With LocalStack (for testing)
//!
//! ```bash
//! docker-compose up -d localstack
//! USE_LOCALSTACK=1 cargo run --example s3_advanced --all-features
//! ```

use chrono::Utc;
use rigatoni_core::destination::Destination;
use rigatoni_core::event::{ChangeEvent, Namespace, OperationType};
use rigatoni_destinations::s3::{
    AwsCredentials, Compression, KeyGenerationStrategy, S3Config, S3Destination,
    SerializationFormat,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Advanced S3 Destination Example ===\n");

    let use_localstack = env::var("USE_LOCALSTACK").is_ok();
    let bucket = env::var("S3_BUCKET").unwrap_or_else(|_| "rigatoni-test-bucket".to_string());

    // Use Parquet format for best analytics performance
    let format = SerializationFormat::Parquet;

    // Use Zstandard compression for best compression ratio and speed
    let compression = Compression::Zstd;

    println!("Configuration:");
    println!("  Bucket: {}", bucket);
    println!("  Format: {:?}", format);
    println!("  Compression: {:?}", compression);
    println!("  Partitioning: Hive-style (for analytics)");
    if use_localstack {
        println!("  Mode: LocalStack (testing)");
    }
    println!();

    // Create advanced configuration with Hive partitioning
    let mut config_builder = S3Config::builder()
        .bucket(bucket)
        .region("us-east-1")
        .prefix("analytics/mongodb-cdc")
        .format(format)
        .compression(compression)
        .key_strategy(KeyGenerationStrategy::HivePartitioned)
        .max_retries(5); // Increased retries for production

    if use_localstack {
        config_builder = config_builder
            .endpoint_url("http://localhost:4566")
            .force_path_style(true)
            .credentials(AwsCredentials::new("test", "test"));
    }

    let config = config_builder.build()?;

    println!("📁 Key Generation Strategy:");
    println!("  Pattern: {}", config.key_strategy.pattern_description());
    println!("\n  Benefits:");
    println!("  ✓ Automatic partition discovery in Athena/Presto/Spark");
    println!("  ✓ Efficient time-range queries");
    println!("  ✓ Organized data lake structure\n");

    let mut destination = S3Destination::new(config).await?;

    // Create diverse events across multiple collections
    println!("Creating sample events across collections...");
    let mut events = Vec::new();

    // User events
    for i in 1..=10 {
        events.push(create_event(
            "users",
            i,
            "user_event",
            format!("user_{}", i).as_str(),
        ));
    }

    // Order events
    for i in 1..=10 {
        events.push(create_event(
            "orders",
            i + 100,
            "order_event",
            format!("order_{}", i).as_str(),
        ));
    }

    // Product events
    for i in 1..=10 {
        events.push(create_event(
            "products",
            i + 200,
            "product_event",
            format!("product_{}", i).as_str(),
        ));
    }

    println!("  Total events: {}", events.len());
    println!("  Collections: users, orders, products\n");

    println!("Writing to S3...");
    let start = std::time::Instant::now();

    destination.write_batch(&events).await?;
    destination.flush().await?;

    let elapsed = start.elapsed();

    println!("\n✓ Successfully wrote events!");
    println!("  Time taken: {:?}", elapsed);
    println!("\n📊 Data Organization:");
    println!("  Each collection will have separate partitioned files:");
    println!("  - analytics/mongodb-cdc/collection=users/year=2025/month=01/day=15/...");
    println!("  - analytics/mongodb-cdc/collection=orders/year=2025/month=01/day=15/...");
    println!("  - analytics/mongodb-cdc/collection=products/year=2025/month=01/day=15/...");

    println!("\n💡 Analytics Usage:");
    println!("  You can now query this data using:");
    println!("  - AWS Athena: CREATE EXTERNAL TABLE with PARTITIONED BY");
    println!("  - Presto/Trino: Automatic partition discovery");
    println!("  - Apache Spark: Read with partition pruning");

    destination.close().await?;
    println!("\n✓ Example completed!");

    Ok(())
}

/// Create a sample event with metadata
fn create_event(collection: &str, id: i32, event_type: &str, name: &str) -> ChangeEvent {
    use bson::doc;

    ChangeEvent {
        resume_token: doc! { "_data": format!("{}_{}", collection, id) },
        operation: OperationType::Insert,
        namespace: Namespace::new("production_db", collection),
        full_document: Some(doc! {
            "_id": id,
            "name": name,
            "event_type": event_type,
            "status": "active",
            "created_at": Utc::now().to_rfc3339(),
            "metadata": doc! {
                "version": "1.0",
                "source": "mongodb-cdc",
                "environment": "production",
            },
        }),
        document_key: Some(doc! { "_id": id }),
        update_description: None,
        cluster_time: Utc::now(),
    }
}
