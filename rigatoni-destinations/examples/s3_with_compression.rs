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

//! S3 destination example with compression.
//!
//! This example demonstrates using different compression algorithms
//! to reduce storage costs and bandwidth usage.
//!
//! # Prerequisites
//!
//! - AWS credentials configured
//! - An S3 bucket you have write access to
//!
//! # Running
//!
//! ```bash
//! # With gzip compression
//! cargo run --example s3_with_compression --features s3,json,gzip
//!
//! # With zstd compression (better compression ratio)
//! cargo run --example s3_with_compression --features s3,json,zstandard
//! ```

use chrono::Utc;
use rigatoni_core::destination::Destination;
use rigatoni_core::event::{ChangeEvent, Namespace, OperationType};
use rigatoni_destinations::s3::{Compression, S3Config, S3Destination};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== S3 Destination with Compression Example ===\n");

    let bucket = env::var("S3_BUCKET").unwrap_or_else(|_| "rigatoni-test-bucket".to_string());

    // Determine compression based on features
    #[cfg(feature = "zstandard")]
    let compression = Compression::Zstd;
    #[cfg(all(feature = "gzip", not(feature = "zstandard")))]
    let compression = Compression::Gzip;
    #[cfg(not(any(feature = "gzip", feature = "zstandard")))]
    let compression = Compression::None;

    println!("Configuration:");
    println!("  Bucket: {}", bucket);
    println!("  Compression: {:?}\n", compression);

    // Create configuration with compression
    let config = S3Config::builder()
        .bucket(bucket)
        .region("us-east-1")
        .prefix("rigatoni/examples/compressed")
        .compression(compression)
        .build()?;

    let mut destination = S3Destination::new(config).await?;

    // Create a larger batch to demonstrate compression benefits
    println!("Creating 100 sample events...");
    let mut events = Vec::new();
    for i in 1..=100 {
        events.push(create_large_event("analytics", i));
    }

    println!("Events created: {}", events.len());
    println!("\nWriting to S3 with {:?} compression...", compression);

    let start = std::time::Instant::now();
    destination.write_batch(&events).await?;
    destination.flush().await?;
    let elapsed = start.elapsed();

    println!("\n✓ Successfully wrote 100 events");
    println!("  Time taken: {:?}", elapsed);
    println!("\nCompression benefits:");
    println!("  - Reduced storage costs");
    println!("  - Lower bandwidth usage");
    println!("  - Faster uploads for large batches");

    match compression {
        Compression::None => {
            println!("\n💡 Tip: Enable gzip or zstd features for compression!");
        }
        #[cfg(feature = "gzip")]
        Compression::Gzip => {
            println!("\n📊 Gzip: ~70-80% compression ratio (industry standard)");
        }
        #[cfg(feature = "zstandard")]
        Compression::Zstd => {
            println!("\n📊 Zstd: ~75-85% compression ratio (faster than gzip!)");
        }
    }

    destination.close().await?;
    println!("\n✓ Example completed!");

    Ok(())
}

/// Create a larger event to demonstrate compression benefits
fn create_large_event(collection: &str, id: i32) -> ChangeEvent {
    use bson::doc;

    ChangeEvent {
        resume_token: doc! { "_data": format!("token_{}", id) },
        operation: OperationType::Insert,
        namespace: Namespace::new("analytics_db", collection),
        full_document: Some(doc! {
            "_id": id,
            "user_id": format!("user_{}", id % 50),
            "event_type": "page_view",
            "url": format!("/products/{}", id % 20),
            "referrer": "https://example.com/home",
            "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            "ip_address": format!("192.168.1.{}", id % 255),
            "session_id": format!("session_{}", id % 100),
            "timestamp": Utc::now().to_rfc3339(),
            "metadata": doc! {
                "browser": "Chrome",
                "os": "Windows",
                "device": "Desktop",
                "country": "US",
                "city": "San Francisco"
            },
        }),
        document_key: Some(doc! { "_id": id }),
        update_description: None,
        cluster_time: Utc::now(),
    }
}
