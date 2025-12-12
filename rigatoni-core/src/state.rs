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

//! State storage for pipeline resume tokens.
//!
//! The [`StateStore`] trait provides an abstraction for persisting MongoDB
//! change stream resume tokens. This enables the pipeline to resume from where
//! it left off after restarts or failures.
//!
//! # Example
//!
//! ```rust
//! use rigatoni_core::state::StateStore;
//! use mongodb::bson::doc;
//! use std::collections::HashMap;
//!
//! // In-memory implementation for testing
//! #[derive(Default)]
//! struct MemoryStateStore {
//!     tokens: std::sync::Arc<tokio::sync::Mutex<HashMap<String, mongodb::bson::Document>>>,
//! }
//!
//! #[async_trait::async_trait]
//! impl StateStore for MemoryStateStore {
//!     async fn save_resume_token(
//!         &self,
//!         collection: &str,
//!         token: &mongodb::bson::Document,
//!     ) -> Result<(), rigatoni_core::state::StateStoreError> {
//!         self.tokens.lock().await.insert(collection.to_string(), token.clone());
//!         Ok(())
//!     }
//!
//!     async fn get_resume_token(
//!         &self,
//!         collection: &str,
//!     ) -> Result<Option<mongodb::bson::Document>, rigatoni_core::state::StateStoreError> {
//!         Ok(self.tokens.lock().await.get(collection).cloned())
//!     }
//!
//!     async fn delete_resume_token(&self, collection: &str) -> Result<(), rigatoni_core::state::StateStoreError> {
//!         self.tokens.lock().await.remove(collection);
//!         Ok(())
//!     }
//!
//!     async fn list_resume_tokens(
//!         &self,
//!     ) -> Result<HashMap<String, mongodb::bson::Document>, rigatoni_core::state::StateStoreError> {
//!         Ok(self.tokens.lock().await.clone())
//!     }
//!
//!     async fn close(&self) -> Result<(), rigatoni_core::state::StateStoreError> {
//!         Ok(())
//!     }
//!
//!     async fn try_acquire_lock(
//!         &self,
//!         _key: &str,
//!         _owner_id: &str,
//!         _ttl: std::time::Duration,
//!     ) -> Result<bool, rigatoni_core::state::StateStoreError> {
//!         Ok(true) // Single instance, always succeed
//!     }
//!
//!     async fn refresh_lock(
//!         &self,
//!         _key: &str,
//!         _owner_id: &str,
//!         _ttl: std::time::Duration,
//!     ) -> Result<bool, rigatoni_core::state::StateStoreError> {
//!         Ok(true)
//!     }
//!
//!     async fn release_lock(
//!         &self,
//!         _key: &str,
//!         _owner_id: &str,
//!     ) -> Result<bool, rigatoni_core::state::StateStoreError> {
//!         Ok(true)
//!     }
//!
//!     async fn is_locked(&self, _key: &str) -> Result<bool, rigatoni_core::state::StateStoreError> {
//!         Ok(false)
//!     }
//! }
//! ```

use mongodb::bson::Document;
use std::collections::HashMap;
use std::time::Duration;

/// Trait for state storage backends.
///
/// Implementations should persist resume tokens durably to survive process restarts.
#[async_trait::async_trait]
pub trait StateStore {
    /// Saves a resume token for a collection.
    ///
    /// # Errors
    ///
    /// Returns an error if the token cannot be saved.
    async fn save_resume_token(
        &self,
        collection: &str,
        token: &Document,
    ) -> Result<(), StateStoreError>;

    /// Retrieves the resume token for a collection.
    ///
    /// Returns `None` if no token exists for the collection.
    ///
    /// # Errors
    ///
    /// Returns an error if the token cannot be retrieved.
    async fn get_resume_token(&self, collection: &str)
        -> Result<Option<Document>, StateStoreError>;

    /// Deletes the resume token for a collection.
    ///
    /// # Errors
    ///
    /// Returns an error if the token cannot be deleted.
    async fn delete_resume_token(&self, collection: &str) -> Result<(), StateStoreError>;

    /// Lists all resume tokens.
    ///
    /// Returns a map of collection names to resume tokens.
    ///
    /// # Errors
    ///
    /// Returns an error if the tokens cannot be listed.
    async fn list_resume_tokens(&self) -> Result<HashMap<String, Document>, StateStoreError>;

    /// Closes the state store, releasing any resources.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be closed cleanly.
    async fn close(&self) -> Result<(), StateStoreError>;

    // ==========================================================================
    // Distributed Locking Methods
    // ==========================================================================

    /// Try to acquire a distributed lock with TTL.
    ///
    /// This method attempts to acquire an exclusive lock for the given key.
    /// The lock is held by the specified owner and will automatically expire
    /// after the TTL duration if not refreshed or released.
    ///
    /// # Arguments
    ///
    /// * `key` - Lock identifier (e.g., "rigatoni:lock:mydb:users")
    /// * `owner_id` - Unique identifier for this instance (e.g., UUID, hostname)
    /// * `ttl` - Time-to-live for the lock (auto-expires if owner crashes)
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Lock acquired successfully
    /// * `Ok(false)` - Lock already held by another instance
    /// * `Err(_)` - Error communicating with state store
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// let acquired = store.try_acquire_lock(
    ///     "rigatoni:lock:mydb:users",
    ///     "instance-abc123",
    ///     Duration::from_secs(30),
    /// ).await?;
    ///
    /// if acquired {
    ///     println!("Lock acquired, safe to process");
    /// } else {
    ///     println!("Lock held by another instance");
    /// }
    /// ```
    async fn try_acquire_lock(
        &self,
        key: &str,
        owner_id: &str,
        ttl: Duration,
    ) -> Result<bool, StateStoreError>;

    /// Refresh an existing lock to extend its TTL.
    ///
    /// This method should be called periodically (heartbeat) to prevent
    /// the lock from expiring during normal operation. The refresh only
    /// succeeds if the lock is still held by the specified owner.
    ///
    /// # Arguments
    ///
    /// * `key` - Lock identifier
    /// * `owner_id` - Unique identifier for this instance
    /// * `ttl` - New time-to-live for the lock
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Lock refreshed successfully
    /// * `Ok(false)` - Lock not held by this owner (was acquired by someone else or expired)
    /// * `Err(_)` - Error communicating with state store
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Refresh lock every 10 seconds with 30 second TTL
    /// let refreshed = store.refresh_lock(
    ///     "rigatoni:lock:mydb:users",
    ///     "instance-abc123",
    ///     Duration::from_secs(30),
    /// ).await?;
    ///
    /// if !refreshed {
    ///     // Lost the lock - another instance took over
    ///     panic!("Lost lock for collection");
    /// }
    /// ```
    async fn refresh_lock(
        &self,
        key: &str,
        owner_id: &str,
        ttl: Duration,
    ) -> Result<bool, StateStoreError>;

    /// Release a lock.
    ///
    /// This method releases the lock if it is held by the specified owner.
    /// Should be called during graceful shutdown to allow other instances
    /// to take over immediately without waiting for TTL expiry.
    ///
    /// # Arguments
    ///
    /// * `key` - Lock identifier
    /// * `owner_id` - Unique identifier for this instance
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Lock released successfully
    /// * `Ok(false)` - Lock not held by this owner (already released or acquired by another)
    /// * `Err(_)` - Error communicating with state store
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Release lock on shutdown
    /// let released = store.release_lock(
    ///     "rigatoni:lock:mydb:users",
    ///     "instance-abc123",
    /// ).await?;
    ///
    /// if released {
    ///     println!("Lock released, other instances can now acquire");
    /// }
    /// ```
    async fn release_lock(&self, key: &str, owner_id: &str) -> Result<bool, StateStoreError>;

    /// Check if a lock is held (by any instance).
    ///
    /// This is a read-only operation to check if a lock exists.
    /// Note: The result may be stale by the time it's used due to
    /// distributed system timing.
    ///
    /// # Arguments
    ///
    /// * `key` - Lock identifier
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Lock is currently held
    /// * `Ok(false)` - Lock is not held (available)
    /// * `Err(_)` - Error communicating with state store
    async fn is_locked(&self, key: &str) -> Result<bool, StateStoreError>;
}

/// Errors that can occur during state store operations.
#[derive(Debug, thiserror::Error)]
pub enum StateStoreError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Other errors
    #[error("State store error: {0}")]
    Other(String),
}
