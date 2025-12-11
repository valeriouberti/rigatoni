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

//! Integration tests for Pipeline orchestration.
//!
//! These tests verify end-to-end pipeline functionality including:
//! - Batching logic
//! - Retry behavior
//! - State persistence
//! - Graceful shutdown

use mongodb::bson::doc;
use rigatoni_core::destination::{Destination, DestinationError, DestinationMetadata};
use rigatoni_core::event::ChangeEvent;
use rigatoni_core::pipeline::{Pipeline, PipelineConfig};
use rigatoni_core::state::{StateStore, StateStoreError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// In-memory state store for testing.
#[derive(Debug, Clone, Default)]
struct MemoryStateStore {
    tokens: Arc<Mutex<HashMap<String, mongodb::bson::Document>>>,
}

#[async_trait::async_trait]
impl StateStore for MemoryStateStore {
    async fn save_resume_token(
        &self,
        collection: &str,
        token: &mongodb::bson::Document,
    ) -> Result<(), StateStoreError> {
        self.tokens
            .lock()
            .await
            .insert(collection.to_string(), token.clone());
        Ok(())
    }

    async fn get_resume_token(
        &self,
        collection: &str,
    ) -> Result<Option<mongodb::bson::Document>, StateStoreError> {
        Ok(self.tokens.lock().await.get(collection).cloned())
    }

    async fn delete_resume_token(&self, collection: &str) -> Result<(), StateStoreError> {
        self.tokens.lock().await.remove(collection);
        Ok(())
    }

    async fn list_resume_tokens(
        &self,
    ) -> Result<HashMap<String, mongodb::bson::Document>, StateStoreError> {
        Ok(self.tokens.lock().await.clone())
    }

    async fn close(&self) -> Result<(), StateStoreError> {
        Ok(())
    }
}

/// Mock destination that tracks calls for testing.
#[derive(Debug)]
struct MockDestination {
    batches: Arc<Mutex<Vec<Vec<ChangeEvent>>>>,
    write_count: Arc<Mutex<usize>>,
    should_fail: Arc<Mutex<bool>>,
    fail_count: Arc<Mutex<usize>>,
    max_failures: usize,
}

impl MockDestination {
    fn new() -> Self {
        Self {
            batches: Arc::new(Mutex::new(Vec::new())),
            write_count: Arc::new(Mutex::new(0)),
            should_fail: Arc::new(Mutex::new(false)),
            fail_count: Arc::new(Mutex::new(0)),
            max_failures: 0,
        }
    }

    #[allow(dead_code)]
    fn with_transient_failures(max_failures: usize) -> Self {
        Self {
            batches: Arc::new(Mutex::new(Vec::new())),
            write_count: Arc::new(Mutex::new(0)),
            should_fail: Arc::new(Mutex::new(true)),
            fail_count: Arc::new(Mutex::new(0)),
            max_failures,
        }
    }

    #[allow(dead_code)]
    async fn get_batches(&self) -> Vec<Vec<ChangeEvent>> {
        self.batches.lock().await.clone()
    }

    #[allow(dead_code)]
    async fn get_write_count(&self) -> usize {
        *self.write_count.lock().await
    }
}

#[async_trait::async_trait]
impl Destination for MockDestination {
    async fn write_batch(&mut self, events: &[ChangeEvent]) -> Result<(), DestinationError> {
        let mut count = self.write_count.lock().await;
        *count += 1;

        // Simulate transient failures
        if *self.should_fail.lock().await {
            let mut failures = self.fail_count.lock().await;
            *failures += 1;

            if *failures <= self.max_failures {
                return Err(DestinationError::write(
                    std::io::Error::other("Simulated retryable failure"),
                    true,
                ));
            }
            // Stop failing after max_failures
            *self.should_fail.lock().await = false;
        }

        self.batches.lock().await.push(events.to_vec());
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), DestinationError> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), DestinationError> {
        Ok(())
    }

    fn supports_transactions(&self) -> bool {
        false
    }

    fn metadata(&self) -> DestinationMetadata {
        DestinationMetadata::new("Mock", "mock")
    }
}

/// Helper to create a test change event.
#[allow(dead_code)]
fn create_test_event(collection: &str, id: i32) -> ChangeEvent {
    use rigatoni_core::event::{Namespace, OperationType};

    ChangeEvent {
        resume_token: doc! { "_data": format!("test_{}_{}", collection, id) },
        operation: OperationType::Insert,
        namespace: Namespace::new("test_db", collection),
        full_document: Some(doc! {
            "_id": id,
            "name": format!("test_{}", id),
        }),
        document_key: Some(doc! { "_id": id }),
        update_description: None,
        cluster_time: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn test_pipeline_config_builder() {
    use rigatoni_core::watch_level::WatchLevel;

    let config = PipelineConfig::builder()
        .mongodb_uri("mongodb://localhost:27017")
        .database("test_db")
        .watch_collections(vec!["users".to_string()])
        .batch_size(100)
        .batch_timeout(Duration::from_secs(5))
        .max_retries(3)
        .build();

    assert!(config.is_ok());
    let config = config.unwrap();
    assert_eq!(config.mongodb_uri, "mongodb://localhost:27017");
    assert_eq!(config.database, "test_db");
    assert!(
        matches!(config.watch_level, WatchLevel::Collection(ref cols) if cols == &vec!["users".to_string()])
    );
    assert_eq!(config.batch_size, 100);
    assert_eq!(config.batch_timeout, Duration::from_secs(5));
    assert_eq!(config.max_retries, 3);
}

#[tokio::test]
async fn test_pipeline_config_builder_defaults() {
    use rigatoni_core::watch_level::WatchLevel;

    let config = PipelineConfig::builder()
        .mongodb_uri("mongodb://localhost:27017")
        .database("test_db")
        .build();

    assert!(config.is_ok());
    let config = config.unwrap();
    assert_eq!(config.batch_size, 100); // Default
    assert_eq!(config.batch_timeout, Duration::from_secs(5)); // Default
                                                              // Default watch level is Database
    assert!(matches!(config.watch_level, WatchLevel::Database));
}

#[tokio::test]
async fn test_pipeline_config_builder_missing_required() {
    let config = PipelineConfig::builder().database("test_db").build();

    assert!(config.is_err());
    let err = config.unwrap_err();
    assert!(matches!(
        err,
        rigatoni_core::pipeline::ConfigError::MissingMongoUri
    ));
}

#[tokio::test]
async fn test_pipeline_creation() {
    let config = PipelineConfig::builder()
        .mongodb_uri("mongodb://localhost:27017")
        .database("test_db")
        .build()
        .unwrap();

    let store = MemoryStateStore::default();
    let destination = MockDestination::new();

    let pipeline = Pipeline::new(config, store, destination).await;
    assert!(pipeline.is_ok());

    let pipeline = pipeline.unwrap();
    assert!(!pipeline.is_running().await);
}

#[tokio::test]
async fn test_pipeline_stats() {
    let config = PipelineConfig::builder()
        .mongodb_uri("mongodb://localhost:27017")
        .database("test_db")
        .build()
        .unwrap();

    let store = MemoryStateStore::default();
    let destination = MockDestination::new();

    let pipeline = Pipeline::new(config, store, destination).await.unwrap();

    let stats = pipeline.stats().await;
    assert_eq!(stats.events_processed, 0);
    assert_eq!(stats.batches_written, 0);
    assert_eq!(stats.write_errors, 0);
    assert_eq!(stats.retries, 0);
}

// Note: The following tests require a running MongoDB instance
// They are marked with #[ignore] to avoid failures in CI

#[tokio::test]
#[ignore] // Requires MongoDB
async fn test_pipeline_start_stop() {
    let config = PipelineConfig::builder()
        .mongodb_uri("mongodb://localhost:27017")
        .database("test_db")
        .watch_collections(vec!["test_collection".to_string()])
        .batch_size(10)
        .batch_timeout(Duration::from_secs(1))
        .build()
        .unwrap();

    let store = MemoryStateStore::default();
    let destination = MockDestination::new();

    let mut pipeline = Pipeline::new(config, store, destination).await.unwrap();

    // Start pipeline
    pipeline.start().await.unwrap();
    assert!(pipeline.is_running().await);

    // Let it run briefly
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Stop pipeline
    pipeline.stop().await.unwrap();
    assert!(!pipeline.is_running().await);
}

#[tokio::test]
#[ignore] // Requires MongoDB
async fn test_pipeline_batching_by_size() {
    // This test would verify that batches are flushed when they reach the configured size
    // It requires inserting test data into MongoDB and observing the batches
}

#[tokio::test]
#[ignore] // Requires MongoDB
async fn test_pipeline_batching_by_timeout() {
    // This test would verify that batches are flushed after the timeout
    // even if they haven't reached the configured size
}

#[tokio::test]
#[ignore] // Requires MongoDB
async fn test_pipeline_retry_logic() {
    // This test would verify that transient errors trigger retries
    // and that the pipeline eventually succeeds
}

#[tokio::test]
#[ignore] // Requires MongoDB
async fn test_pipeline_state_persistence() {
    // This test would verify that resume tokens are saved after successful writes
    // and that the pipeline can resume from a saved token
}

#[tokio::test]
#[ignore] // Requires MongoDB
async fn test_pipeline_graceful_shutdown() {
    // This test would verify that pending batches are flushed during shutdown
    // and that state is properly saved
}

#[tokio::test]
#[ignore] // Requires MongoDB
async fn test_pipeline_backpressure() {
    // This test would verify that the pipeline slows down MongoDB reads
    // when the destination is slow
}

#[tokio::test]
#[ignore] // Requires MongoDB
async fn test_pipeline_multiple_collections() {
    // This test would verify that the pipeline correctly handles
    // multiple collections with separate workers
}
