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

//! In-memory state store implementation.
//!
//! This module provides a thread-safe, in-memory implementation of the
//! [`StateStore`] trait for storing `MongoDB`
//! change stream resume tokens.
//!
//! # Use Cases
//!
//! The in-memory store is suitable for:
//!
//! - **Local development and testing** - No external dependencies required
//! - **Single-instance deployments** - Where distributed state isn't needed
//! - **Prototyping** - Quick setup without infrastructure
//!
//! # Limitations
//!
//! ⚠️ **Important**: The in-memory store has the following limitations:
//!
//! - **No persistence** - Tokens are lost on process restart
//! - **Single process only** - Cannot be shared across multiple instances
//! - **No durability** - Data lost on crash or shutdown
//!
//! For production deployments with multiple instances or high availability
//! requirements, use [`RedisStore`](crate::redis::RedisStore) instead.
//!
//! # Example
//!
//! ```rust
//! use rigatoni_stores::memory::MemoryStore;
//! use rigatoni_core::state::StateStore;
//! use mongodb::bson::doc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create in-memory store
//! let store = MemoryStore::new();
//!
//! // Save resume token
//! let token = doc! { "_data": "token_abc123" };
//! store.save_resume_token("users", &token).await?;
//!
//! // Retrieve token
//! let retrieved = store.get_resume_token("users").await?;
//! assert!(retrieved.is_some());
//!
//! // List all tokens
//! let all_tokens = store.list_resume_tokens().await?;
//! assert_eq!(all_tokens.len(), 1);
//!
//! // Delete token
//! store.delete_resume_token("users").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Thread Safety
//!
//! The store uses [`Arc`] and [`RwLock`] internally, making it safe to share
//! across async tasks and threads.
//!
//! ```rust
//! use rigatoni_stores::memory::MemoryStore;
//! use std::sync::Arc;
//!
//! # async fn example() {
//! let store = Arc::new(MemoryStore::new());
//!
//! // Clone and use in multiple tasks
//! let store_clone = Arc::clone(&store);
//! tokio::spawn(async move {
//!     // Use store_clone in async task
//! });
//! # }
//! ```

use bson::Document;
use rigatoni_core::state::{StateStore, StateStoreError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

/// In-memory state store for `MongoDB` change stream resume tokens.
///
/// This store keeps all resume tokens in memory using a thread-safe
/// [`HashMap`] protected by an [`RwLock`].
///
/// # Performance
///
/// - **Read operations** - Fast, multiple readers can access concurrently
/// - **Write operations** - Slightly slower due to exclusive lock
/// - **Memory usage** - Proportional to number of collections tracked
///
/// # Example
///
/// ```rust
/// use rigatoni_stores::memory::MemoryStore;
/// use rigatoni_core::pipeline::{Pipeline, PipelineConfig};
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create store
/// let store = MemoryStore::new();
///
/// // Use with pipeline
/// let config = PipelineConfig::builder()
///     .mongodb_uri("mongodb://localhost:27017")
///     .database("mydb")
///     .collections(vec!["users".to_string()])
///     .build()?;
///
/// // let destination = /* your destination */;
/// // let pipeline = Pipeline::new(config, store, destination).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct MemoryStore {
    /// Internal storage for resume tokens
    tokens: Arc<RwLock<HashMap<String, Document>>>,
}

impl MemoryStore {
    /// Creates a new in-memory state store.
    ///
    /// The store is initially empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_stores::memory::MemoryStore;
    ///
    /// let store = MemoryStore::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        debug!("Creating new in-memory state store");
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new in-memory store with pre-populated tokens.
    ///
    /// This is useful for testing or when migrating from another store.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_stores::memory::MemoryStore;
    /// use mongodb::bson::doc;
    /// use std::collections::HashMap;
    ///
    /// let mut initial_tokens = HashMap::new();
    /// initial_tokens.insert("users".to_string(), doc! { "_data": "token123" });
    ///
    /// let store = MemoryStore::with_tokens(initial_tokens);
    /// ```
    #[must_use]
    pub fn with_tokens(tokens: HashMap<String, Document>) -> Self {
        debug!(
            token_count = tokens.len(),
            "Creating in-memory state store with initial tokens"
        );
        Self {
            tokens: Arc::new(RwLock::new(tokens)),
        }
    }

    /// Returns the current number of stored tokens.
    ///
    /// This is useful for monitoring and debugging.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rigatoni_stores::memory::MemoryStore;
    /// # use rigatoni_core::state::StateStore;
    /// # use mongodb::bson::doc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = MemoryStore::new();
    ///
    /// assert_eq!(store.len().await, 0);
    ///
    /// store.save_resume_token("users", &doc! { "_data": "token" }).await?;
    /// assert_eq!(store.len().await, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn len(&self) -> usize {
        self.tokens.read().await.len()
    }

    /// Returns `true` if the store contains no tokens.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rigatoni_stores::memory::MemoryStore;
    /// # async fn example() {
    /// let store = MemoryStore::new();
    /// assert!(store.is_empty().await);
    /// # }
    /// ```
    pub async fn is_empty(&self) -> bool {
        self.tokens.read().await.is_empty()
    }

    /// Clears all stored tokens.
    ///
    /// This removes all resume tokens from the store.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rigatoni_stores::memory::MemoryStore;
    /// # use rigatoni_core::state::StateStore;
    /// # use mongodb::bson::doc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = MemoryStore::new();
    /// store.save_resume_token("users", &doc! { "_data": "token" }).await?;
    ///
    /// store.clear().await;
    /// assert!(store.is_empty().await);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn clear(&self) {
        let mut tokens = self.tokens.write().await;
        let count = tokens.len();
        tokens.clear();
        debug!(
            cleared_count = count,
            "Cleared all tokens from memory store"
        );
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StateStore for MemoryStore {
    async fn save_resume_token(
        &self,
        collection: &str,
        token: &Document,
    ) -> Result<(), StateStoreError> {
        trace!(
            collection = collection,
            token = ?token,
            "Saving resume token to memory"
        );

        let mut tokens = self.tokens.write().await;
        tokens.insert(collection.to_string(), token.clone());

        debug!(
            collection = collection,
            total_tokens = tokens.len(),
            "Saved resume token to memory"
        );

        Ok(())
    }

    async fn get_resume_token(
        &self,
        collection: &str,
    ) -> Result<Option<Document>, StateStoreError> {
        trace!(
            collection = collection,
            "Retrieving resume token from memory"
        );

        let tokens = self.tokens.read().await;
        let token = tokens.get(collection).cloned();

        if token.is_some() {
            debug!(collection = collection, "Found resume token in memory");
        } else {
            debug!(collection = collection, "No resume token found in memory");
        }

        Ok(token)
    }

    async fn delete_resume_token(&self, collection: &str) -> Result<(), StateStoreError> {
        trace!(collection = collection, "Deleting resume token from memory");

        let mut tokens = self.tokens.write().await;
        let removed = tokens.remove(collection);

        if removed.is_some() {
            debug!(
                collection = collection,
                remaining_tokens = tokens.len(),
                "Deleted resume token from memory"
            );
        } else {
            warn!(
                collection = collection,
                "Attempted to delete non-existent resume token"
            );
        }

        Ok(())
    }

    async fn list_resume_tokens(&self) -> Result<HashMap<String, Document>, StateStoreError> {
        trace!("Listing all resume tokens from memory");

        let tokens = self.tokens.read().await;
        let token_count = tokens.len();

        debug!(
            token_count = token_count,
            "Listed all resume tokens from memory"
        );

        Ok(tokens.clone())
    }

    async fn close(&self) -> Result<(), StateStoreError> {
        debug!("Closing in-memory state store (no-op)");
        // No resources to clean up for in-memory store
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bson::doc;

    #[tokio::test]
    async fn test_new_store_is_empty() {
        let store = MemoryStore::new();
        assert!(store.is_empty().await);
        assert_eq!(store.len().await, 0);
    }

    #[tokio::test]
    async fn test_save_and_retrieve_token() {
        let store = MemoryStore::new();
        let token = doc! { "_data": "test_token_123" };

        // Save token
        store
            .save_resume_token("users", &token)
            .await
            .expect("Failed to save token");

        // Retrieve token
        let retrieved = store
            .get_resume_token("users")
            .await
            .expect("Failed to retrieve token");

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), token);
        assert_eq!(store.len().await, 1);
    }

    #[tokio::test]
    async fn test_retrieve_nonexistent_token() {
        let store = MemoryStore::new();

        let retrieved = store
            .get_resume_token("nonexistent")
            .await
            .expect("Failed to retrieve token");

        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_update_token() {
        let store = MemoryStore::new();
        let token1 = doc! { "_data": "token_v1" };
        let token2 = doc! { "_data": "token_v2" };

        // Save initial token
        store
            .save_resume_token("users", &token1)
            .await
            .expect("Failed to save token");

        // Update with new token
        store
            .save_resume_token("users", &token2)
            .await
            .expect("Failed to update token");

        // Verify updated
        let retrieved = store.get_resume_token("users").await.unwrap();
        assert_eq!(retrieved.unwrap(), token2);
        assert_eq!(store.len().await, 1); // Still only one token
    }

    #[tokio::test]
    async fn test_delete_token() {
        let store = MemoryStore::new();
        let token = doc! { "_data": "test_token" };

        // Save and then delete
        store.save_resume_token("users", &token).await.unwrap();
        assert_eq!(store.len().await, 1);

        store.delete_resume_token("users").await.unwrap();
        assert_eq!(store.len().await, 0);

        // Verify deleted
        let retrieved = store.get_resume_token("users").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_token() {
        let store = MemoryStore::new();

        // Should not error when deleting nonexistent token
        store.delete_resume_token("nonexistent").await.unwrap();
        assert_eq!(store.len().await, 0);
    }

    #[tokio::test]
    async fn test_list_tokens() {
        let store = MemoryStore::new();
        let token1 = doc! { "_data": "token1" };
        let token2 = doc! { "_data": "token2" };
        let token3 = doc! { "_data": "token3" };

        // Save multiple tokens
        store.save_resume_token("users", &token1).await.unwrap();
        store.save_resume_token("orders", &token2).await.unwrap();
        store.save_resume_token("products", &token3).await.unwrap();

        // List all
        let all_tokens = store.list_resume_tokens().await.unwrap();
        assert_eq!(all_tokens.len(), 3);
        assert_eq!(all_tokens.get("users"), Some(&token1));
        assert_eq!(all_tokens.get("orders"), Some(&token2));
        assert_eq!(all_tokens.get("products"), Some(&token3));
    }

    #[tokio::test]
    async fn test_list_empty_store() {
        let store = MemoryStore::new();
        let tokens = store.list_resume_tokens().await.unwrap();
        assert!(tokens.is_empty());
    }

    #[tokio::test]
    async fn test_clear() {
        let store = MemoryStore::new();

        // Add some tokens
        store
            .save_resume_token("users", &doc! { "_data": "token1" })
            .await
            .unwrap();
        store
            .save_resume_token("orders", &doc! { "_data": "token2" })
            .await
            .unwrap();
        assert_eq!(store.len().await, 2);

        // Clear
        store.clear().await;
        assert!(store.is_empty().await);
        assert_eq!(store.len().await, 0);
    }

    #[tokio::test]
    async fn test_with_tokens() {
        let mut initial = HashMap::new();
        initial.insert("users".to_string(), doc! { "_data": "token1" });
        initial.insert("orders".to_string(), doc! { "_data": "token2" });

        let store = MemoryStore::with_tokens(initial);
        assert_eq!(store.len().await, 2);

        let token = store.get_resume_token("users").await.unwrap();
        assert!(token.is_some());
    }

    #[tokio::test]
    async fn test_close() {
        let store = MemoryStore::new();
        // Close should succeed
        store.close().await.expect("Failed to close store");
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        use std::sync::Arc;

        let store = Arc::new(MemoryStore::new());
        let mut handles = vec![];

        // Spawn multiple tasks writing to different collections
        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            let handle = tokio::spawn(async move {
                let collection = format!("collection_{i}");
                let token = doc! { "_data": format!("token_{i}") };
                store_clone
                    .save_resume_token(&collection, &token)
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all tokens saved
        assert_eq!(store.len().await, 10);
    }

    #[tokio::test]
    async fn test_clone() {
        let store1 = MemoryStore::new();
        store1
            .save_resume_token("users", &doc! { "_data": "token" })
            .await
            .unwrap();

        // Clone shares the same underlying storage
        let store2 = store1.clone();
        assert_eq!(store2.len().await, 1);

        // Changes in store2 are visible in store1
        store2
            .save_resume_token("orders", &doc! { "_data": "token2" })
            .await
            .unwrap();
        assert_eq!(store1.len().await, 2);
    }
}
