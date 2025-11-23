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

//! Basic S3 destination example.
//!
//! This example demonstrates the simplest usage of the S3 destination
//! with JSON serialization and no compression.
//!
//! # Prerequisites
//!
//! - AWS credentials configured (via environment variables, AWS CLI, or IAM role)
//! - An S3 bucket you have write access to
//!
//! # Running
//!
//! ```bash
//! # Set your S3 bucket name
//! export S3_BUCKET="your-bucket-name"
//!
//! # Run the example
//! cargo run --example s3_basic --features s3,json
//! ```

use chrono::Utc;
use rigatoni_core::destination::Destination;
use rigatoni_core::event::{ChangeEvent, Namespace, OperationType};
use rigatoni_destinations::s3::{S3Config, S3Destination};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Basic S3 Destination Example ===\n");

    // Get bucket name from environment
    let bucket = env::var("S3_BUCKET").unwrap_or_else(|_| "rigatoni-test-bucket".to_string());

    println!("Using S3 bucket: {}\n", bucket);

    // Create S3 configuration with minimal settings
    let config = S3Config::builder()
        .bucket(&bucket)
        .region("us-east-1")
        .prefix("rigatoni/examples/basic")
        .build()?;

    println!("Configuration:");
    println!("  Bucket: {}", config.bucket);
    println!("  Region: {}", config.region);
    println!("  Prefix: {:?}", config.prefix);
    println!("  Format: {:?}", config.format);
    println!("  Compression: {:?}\n", config.compression);

    // Create S3 destination
    let mut destination = S3Destination::new(config).await?;

    // Print metadata
    let metadata = destination.metadata();
    println!("Destination Metadata:");
    println!("  Name: {}", metadata.name);
    println!("  Type: {}", metadata.destination_type);
    println!(
        "  Supports Transactions: {}",
        metadata.supports_transactions
    );
    println!(
        "  Supports Concurrent Writes: {}\n",
        metadata.supports_concurrent_writes
    );

    // Create sample events
    println!("Creating sample change events...\n");
    let events = vec![
        create_sample_event("users", 1, "Alice"),
        create_sample_event("users", 2, "Bob"),
        create_sample_event("products", 101, "Laptop"),
        create_sample_event("products", 102, "Mouse"),
    ];

    println!("Writing {} events to S3...", events.len());

    // Write batch to S3
    destination.write_batch(&events).await?;

    // Flush to ensure data is written
    println!("Flushing to S3...");
    destination.flush().await?;

    println!("\n✓ Successfully wrote events to S3!");
    println!("\nYou can now check your S3 bucket for the uploaded files.");
    println!("Files will be organized by collection and partitioned by date/hour.");

    // Clean shutdown
    destination.close().await?;

    println!("\n✓ Destination closed successfully!");

    Ok(())
}

/// Helper function to create sample change events
fn create_sample_event(collection: &str, id: i32, name: &str) -> ChangeEvent {
    use bson::doc;

    ChangeEvent {
        resume_token: doc! { "_data": format!("token_{}", id) },
        operation: OperationType::Insert,
        namespace: Namespace::new("example_db", collection),
        full_document: Some(doc! {
            "_id": id,
            "name": name,
            "created_at": Utc::now().to_rfc3339(),
        }),
        document_key: Some(doc! { "_id": id }),
        update_description: None,
        cluster_time: Utc::now(),
    }
}
