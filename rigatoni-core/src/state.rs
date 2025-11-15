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
//! }
//! ```

use mongodb::bson::Document;
use std::collections::HashMap;

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
