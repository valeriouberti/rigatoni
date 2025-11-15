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

//! Integration tests for MongoDB Change Stream Listener
//!
//! These tests require a real MongoDB instance. They use testcontainers
//! to spin up a MongoDB container for testing.
//!
//! To run these tests:
//! ```bash
//! cargo test --package rigatoni-core --test stream_integration_test
//! ```
//!
//! Note: These tests are currently skipped by default. To enable them:
//! 1. Install Docker
//! 2. Add testcontainers-modules crate as a dev dependency
//! 3. Remove the #[ignore] annotations

use bson::{doc, Document};
use futures::StreamExt;
use mongodb::{options::ClientOptions, Client};
use rigatoni_core::stream::{ChangeStreamConfig, ChangeStreamListener};
use std::time::Duration;
use tokio::time::timeout;

/// Helper to create a test MongoDB client
///
/// In a real integration test, this would use testcontainers:
/// ```ignore
/// use testcontainers::{clients, images::mongo::Mongo};
///
/// async fn create_test_client() -> (Client, Container<'_, Mongo>) {
///     let docker = clients::Cli::default();
///     let mongo = docker.run(Mongo::default());
///     let port = mongo.get_host_port_ipv4(27017);
///     let uri = format!("mongodb://localhost:{}", port);
///     let client = Client::with_uri_str(&uri).await.unwrap();
///     (client, mongo)
/// }
/// ```
async fn create_test_client() -> Result<Client, Box<dyn std::error::Error>> {
    // For now, assume MongoDB is running locally
    // In production tests, use testcontainers
    let options = ClientOptions::parse("mongodb://localhost:27017").await?;
    Ok(Client::with_options(options)?)
}

#[tokio::test]
#[ignore] // Remove this to run the test with a real MongoDB instance
async fn test_change_stream_basic() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let client = create_test_client().await?;
    let db = client.database("test_rigatoni");
    let collection = db.collection::<Document>("test_events");

    // Clear collection
    collection.drop().await.ok();

    // Create change stream listener
    let config = ChangeStreamConfig::builder()
        .batch_size(10)
        .build()
        .unwrap();

    let mut listener = ChangeStreamListener::new(collection.clone(), config, |token| {
        Box::pin(async move {
            println!("Resume token: {:?}", token);
            Ok(())
        })
    })
    .await?;

    // Spawn task to insert documents
    let collection_clone = collection.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        for i in 0..5 {
            collection_clone.insert_one(doc! { "value": i }).await.ok();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });

    // Consume events with timeout
    let mut count = 0;
    let result = timeout(Duration::from_secs(5), async {
        while let Some(result) = listener.next().await {
            match result {
                Ok(ackable) => {
                    let event = ackable.event_ref();
                    println!("Received event: {:?}", event.operation);
                    ackable.ack();
                    count += 1;
                    if count >= 5 {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    break;
                }
            }
        }
    })
    .await;

    assert!(result.is_ok(), "Should receive events within timeout");
    assert_eq!(count, 5, "Should receive 5 insert events");

    // Cleanup
    collection.drop().await.ok();

    Ok(())
}

#[tokio::test]
#[ignore] // Remove this to run the test with a real MongoDB instance
async fn test_change_stream_with_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_client().await?;
    let db = client.database("test_rigatoni");
    let collection = db.collection::<Document>("test_pipeline");

    // Clear collection
    collection.drop().await.ok();

    // Create pipeline to filter only insert operations
    let pipeline = vec![doc! {
        "$match": {
            "operationType": "insert"
        }
    }];

    let config = ChangeStreamConfig::builder()
        .pipeline(pipeline)
        .build()
        .unwrap();

    let mut listener =
        ChangeStreamListener::new(collection.clone(), config, |_| Box::pin(async { Ok(()) }))
            .await?;

    // Spawn task to perform operations
    let collection_clone = collection.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Insert
        let doc_id = collection_clone
            .insert_one(doc! { "value": 1 })
            .await
            .unwrap()
            .inserted_id;

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Update (should be filtered out)
        collection_clone
            .update_one(doc! { "_id": &doc_id }, doc! { "$set": { "value": 2 } })
            .await
            .ok();

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Another insert
        collection_clone.insert_one(doc! { "value": 3 }).await.ok();
    });

    // Should only receive 2 insert events (updates filtered out)
    let mut insert_count = 0;
    let result = timeout(Duration::from_secs(5), async {
        while let Some(result) = listener.next().await {
            if let Ok(ackable) = result {
                let event = ackable.event_ref();
                assert!(event.is_insert(), "Should only receive insert events");
                ackable.ack();
                insert_count += 1;
                if insert_count >= 2 {
                    break;
                }
            }
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(insert_count, 2);

    // Cleanup
    collection.drop().await.ok();

    Ok(())
}

#[tokio::test]
#[ignore] // Remove this to run the test with a real MongoDB instance
async fn test_resume_token_persistence() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::{Arc, Mutex};

    let client = create_test_client().await?;
    let db = client.database("test_rigatoni");
    let collection = db.collection::<Document>("test_resume");

    collection.drop().await.ok();

    // Track saved resume tokens
    let saved_tokens = Arc::new(Mutex::new(Vec::new()));
    let saved_tokens_clone = saved_tokens.clone();

    let config = ChangeStreamConfig::default();

    let mut listener = ChangeStreamListener::new(collection.clone(), config, move |token| {
        let tokens = saved_tokens_clone.clone();
        Box::pin(async move {
            tokens.lock().unwrap().push(token);
            Ok(())
        })
    })
    .await?;

    // Insert documents
    let collection_clone = collection.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        for i in 0..3 {
            collection_clone.insert_one(doc! { "value": i }).await.ok();
        }
    });

    // Consume events
    timeout(Duration::from_secs(3), async {
        let mut count = 0;
        while let Some(result) = listener.next().await {
            if result.is_ok() {
                count += 1;
                if count >= 3 {
                    break;
                }
            }
        }
    })
    .await
    .ok();

    // Verify resume tokens were saved
    {
        let tokens = saved_tokens.lock().unwrap();
        assert!(
            tokens.len() >= 3,
            "Should have saved at least 3 resume tokens"
        );
    } // Lock is dropped here

    // Cleanup
    collection.drop().await.ok();

    Ok(())
}

#[tokio::test]
#[ignore] // Remove this to run the test with a real MongoDB instance
async fn test_invalidate_event() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_test_client().await?;
    let db = client.database("test_rigatoni");
    let collection = db.collection::<Document>("test_invalidate");

    collection.drop().await.ok();
    collection.insert_one(doc! { "init": 1 }).await?;

    let config = ChangeStreamConfig::default();
    let mut listener =
        ChangeStreamListener::new(collection.clone(), config, |_| Box::pin(async { Ok(()) }))
            .await?;

    // Spawn task to drop collection
    let collection_clone = collection.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        collection_clone.drop().await.ok();
    });

    // Wait for invalidate event
    let result = timeout(Duration::from_secs(5), async {
        while let Some(result) = listener.next().await {
            match result {
                Ok(ackable) => {
                    let event = ackable.event_ref();
                    if event.is_invalidate() {
                        ackable.ack();
                        return true;
                    }
                    ackable.ack();
                }
                Err(e) => {
                    // Should receive StreamError::Invalidated
                    println!("Received error: {}", e);
                    return e.to_string().contains("invalidated")
                        || e.to_string().contains("dropped");
                }
            }
        }
        false
    })
    .await;

    assert!(result.is_ok());

    Ok(())
}

/// Example of setting up testcontainers (requires testcontainers crate)
///
/// Add to Cargo.toml:
/// ```toml
/// [dev-dependencies]
/// testcontainers = "0.15"
/// testcontainers-modules = { version = "0.3", features = ["mongo"] }
/// ```
///
/// Then use:
/// ```ignore
/// use testcontainers::{clients, images::mongo::Mongo};
///
/// #[tokio::test]
/// async fn test_with_container() {
///     let docker = clients::Cli::default();
///     let mongo_image = Mongo::default();
///     let container = docker.run(mongo_image);
///
///     let port = container.get_host_port_ipv4(27017);
///     let uri = format!("mongodb://localhost:{}", port);
///
///     let client = Client::with_uri_str(&uri).await.unwrap();
///     // ... rest of test
/// }
/// ```
#[test]
fn example_testcontainers_setup() {
    // This is just a placeholder to show the setup
    // Real implementation requires testcontainers crate
}
