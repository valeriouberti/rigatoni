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

//! Example usage of ChangeStreamListener
//!
//! This example demonstrates various ways to use the MongoDB Change Stream Listener.
//!
//! To run this example:
//! ```bash
//! # Make sure MongoDB is running on localhost:27017
//! cargo run --example change_stream_listener
//! ```

use bson::{doc, Document};
use futures::StreamExt;
use mongodb::{options::ClientOptions, Client};
use rigatoni_core::stream::{ChangeStreamConfig, ChangeStreamListener};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("MongoDB Change Stream Listener Examples\n");
    println!("Make sure MongoDB is running on localhost:27017\n");

    // Run examples
    example_basic_usage().await?;
    example_with_filtering().await?;
    example_with_resume_token().await?;
    example_multi_collection().await?;

    println!("\nAll examples completed successfully!");

    Ok(())
}

/// Example 1: Basic change stream usage
async fn example_basic_usage() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 1: Basic Usage ===\n");

    // Connect to MongoDB
    let client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    let client = Client::with_options(client_options)?;

    let db = client.database("rigatoni_examples");
    let collection = db.collection::<Document>("basic_example");

    // Clear collection
    collection.drop().await.ok();

    // Create change stream listener with default config
    let config = ChangeStreamConfig::default();

    let mut listener = ChangeStreamListener::new(collection.clone(), config, |token| {
        Box::pin(async move {
            println!("  [Resume Token Saved]: {:?}", token.get("_data"));
            Ok(())
        })
    })
    .await?;

    // Spawn task to generate some events
    let collection_clone = collection.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;

        println!("  Inserting documents...");
        for i in 1..=3 {
            collection_clone
                .insert_one(doc! {
                    "name": format!("User {}", i),
                    "email": format!("user{}@example.com", i),
                    "age": 20 + i
                })
                .await
                .ok();
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });

    // Consume events
    println!("  Listening for change events...\n");
    let mut count = 0;

    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(result) = listener.next().await {
            match result {
                Ok(ackable) => {
                    let event = ackable.event_ref();
                    println!(
                        "  Received: {:?} event in collection '{}'",
                        event.operation,
                        event.collection_name()
                    );

                    if let Some(doc) = &event.full_document {
                        println!("    Document: {:?}", doc.get("name"));
                    }

                    // Acknowledge the event after processing
                    ackable.ack();

                    count += 1;
                    if count >= 3 {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("  Error: {}", e);
                    break;
                }
            }
        }
    })
    .await
    .ok();

    println!("\n  Processed {} events\n", count);

    // Cleanup
    collection.drop().await.ok();

    Ok(())
}

/// Example 2: Using pipeline filters
async fn example_with_filtering() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 2: Pipeline Filtering ===\n");

    let client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    let client = Client::with_options(client_options)?;

    let db = client.database("rigatoni_examples");
    let collection = db.collection::<Document>("filter_example");

    collection.drop().await.ok();

    // Create pipeline to only watch insert and update operations
    let pipeline = vec![
        doc! {
            "$match": {
                "operationType": { "$in": ["insert", "update"] }
            }
        },
        // Optionally filter by specific fields
        doc! {
            "$match": {
                "fullDocument.priority": { "$gte": 5 }
            }
        },
    ];

    let config = ChangeStreamConfig::builder()
        .pipeline(pipeline)
        .full_document_update_lookup() // Include full document for updates
        .build()?;

    let mut listener =
        ChangeStreamListener::new(collection.clone(), config, |_| Box::pin(async { Ok(()) }))
            .await?;

    // Generate mixed events
    let collection_clone = collection.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;

        // High priority insert (will be captured)
        let doc_id = collection_clone
            .insert_one(doc! { "task": "Important", "priority": 10 })
            .await
            .unwrap()
            .inserted_id;

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Low priority insert (will be filtered out)
        collection_clone
            .insert_one(doc! { "task": "Minor", "priority": 2 })
            .await
            .ok();

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Update high priority (will be captured)
        collection_clone
            .update_one(
                doc! { "_id": &doc_id },
                doc! { "$set": { "status": "in_progress" } },
            )
            .await
            .ok();

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Delete (will be filtered out by operationType filter)
        collection_clone
            .delete_one(doc! { "_id": &doc_id })
            .await
            .ok();
    });

    println!("  Listening for filtered events (priority >= 5, insert/update only)...\n");

    tokio::time::timeout(Duration::from_secs(5), async {
        let mut count = 0;
        while let Some(result) = listener.next().await {
            if let Ok(ackable) = result {
                let event = ackable.event_ref();
                println!("  Captured: {:?} operation", event.operation);
                if let Some(doc) = &event.full_document {
                    println!("    Priority: {:?}", doc.get("priority"));
                }
                ackable.ack();
                count += 1;
                if count >= 2 {
                    break;
                }
            }
        }
    })
    .await
    .ok();

    println!();

    collection.drop().await.ok();

    Ok(())
}

/// Example 3: Resume token persistence with Redis (simulated)
async fn example_with_resume_token() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 3: Resume Token Persistence ===\n");

    use std::sync::{Arc, Mutex};

    let client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    let client = Client::with_options(client_options)?;

    let db = client.database("rigatoni_examples");
    let collection = db.collection::<Document>("resume_example");

    collection.drop().await.ok();

    // Simulate resume token storage (in real app, use Redis/Database)
    let last_token = Arc::new(Mutex::new(None::<Document>));
    let last_token_clone = last_token.clone();

    // First stream: consume some events
    println!("  Starting first stream...");

    let config = ChangeStreamConfig::builder()
        .max_reconnect_attempts(3)
        .initial_backoff_ms(100)
        .build()?;

    let mut listener =
        ChangeStreamListener::new(collection.clone(), config.clone(), move |token| {
            let token_storage = last_token_clone.clone();
            Box::pin(async move {
                *token_storage.lock().unwrap() = Some(token.clone());
                println!("    Saved resume token");
                Ok(())
            })
        })
        .await?;

    // Insert some documents
    let collection_clone = collection.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        for i in 1..=2 {
            collection_clone
                .insert_one(doc! { "batch": 1, "seq": i })
                .await
                .ok();
        }
    });

    // Consume first batch
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut count = 0;
        while let Some(result) = listener.next().await {
            if result.is_ok() {
                count += 1;
                println!("    Event {} received", count);
                if count >= 2 {
                    break;
                }
            }
        }
    })
    .await
    .ok();

    // Close first listener
    listener.close().await;
    println!("  First stream closed");

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Insert more documents while stream is closed
    println!("\n  Inserting more documents while stream is closed...");
    for i in 1..=2 {
        collection.insert_one(doc! { "batch": 2, "seq": i }).await?;
    }

    // Resume from last token
    println!("\n  Starting second stream with resume token...");

    let resume_token = last_token.lock().unwrap().clone();

    let config_with_resume = ChangeStreamConfig::builder()
        .resume_token(resume_token.unwrap())
        .build()?;

    let mut listener2 = ChangeStreamListener::new(collection.clone(), config_with_resume, |_| {
        Box::pin(async { Ok(()) })
    })
    .await?;

    // Should receive the events that occurred while stream was closed
    println!("  Consuming missed events...\n");
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut count = 0;
        while let Some(result) = listener2.next().await {
            if let Ok(ackable) = result {
                let event = ackable.event_ref();
                if let Some(doc) = &event.full_document {
                    println!(
                        "    Received missed event: batch={}, seq={}",
                        doc.get("batch").unwrap(),
                        doc.get("seq").unwrap()
                    );
                }
                ackable.ack();
                count += 1;
                if count >= 2 {
                    break;
                }
            }
        }
    })
    .await
    .ok();

    println!();

    collection.drop().await.ok();

    Ok(())
}

/// Example 4: Monitoring multiple collections
async fn example_multi_collection() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 4: Multi-Collection Monitoring ===\n");

    use tokio::task::JoinSet;

    let client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    let client = Client::with_options(client_options)?;

    let db = client.database("rigatoni_examples");

    // Collections to monitor
    let collections = vec!["users", "orders", "products"];

    // Clear collections
    for name in &collections {
        db.collection::<Document>(name).drop().await.ok();
    }

    let mut tasks = JoinSet::new();

    // Spawn a listener for each collection
    for collection_name in &collections {
        let collection = db.collection::<Document>(collection_name);
        let name = collection_name.to_string();

        tasks.spawn(async move {
            let config = ChangeStreamConfig::default();

            let mut listener = ChangeStreamListener::new(collection.clone(), config, move |_| {
                Box::pin(async move { Ok(()) })
            })
            .await
            .unwrap();

            println!("  Monitoring collection: {}", name);

            // Process events for 3 seconds
            tokio::time::timeout(Duration::from_secs(3), async {
                while let Some(result) = listener.next().await {
                    if let Ok(ackable) = result {
                        let event = ackable.event_ref();
                        println!("  [{}] {:?} event", name, event.operation);
                        ackable.ack();
                    }
                }
            })
            .await
            .ok();
        });
    }

    // Generate events in different collections
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;

        db.collection::<Document>("users")
            .insert_one(doc! { "name": "Alice" })
            .await
            .ok();

        tokio::time::sleep(Duration::from_millis(100)).await;

        db.collection::<Document>("orders")
            .insert_one(doc! { "order_id": 1001 })
            .await
            .ok();

        tokio::time::sleep(Duration::from_millis(100)).await;

        db.collection::<Document>("products")
            .insert_one(doc! { "sku": "PROD-123" })
            .await
            .ok();
    });

    // Wait for all tasks
    while let Some(result) = tasks.join_next().await {
        result?;
    }

    println!();

    Ok(())
}
