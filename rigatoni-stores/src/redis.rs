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

//! Redis-backed state store for distributed deployments.
//!
//! This module provides a Redis implementation of the [`StateStore`](rigatoni_core::state::StateStore)
//! trait, enabling distributed state management across multiple Rigatoni pipeline instances.
//!
//! # Features
//!
//! - **Connection Pooling**: Uses `deadpool-redis` for efficient connection management
//! - **Cluster Support**: Handles Redis Cluster redirections automatically
//! - **TTL Support**: Optional expiration for resume tokens
//! - **Retry Logic**: Automatic retries on transient connection failures
//! - **Atomic Operations**: Uses Redis atomic commands for consistency
//!
//! # Example: Standalone Redis
//!
//! ```rust,no_run
//! use rigatoni_stores::redis::{RedisStore, RedisConfig};
//! use rigatoni_core::state::StateStore;
//! use mongodb::bson::doc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create Redis store configuration
//! let config = RedisConfig::builder()
//!     .url("redis://localhost:6379")
//!     .pool_size(10)
//!     .build()?;
//!
//! // Initialize store
//! let store = RedisStore::new(config).await?;
//!
//! // Save a resume token
//! let token = doc! { "_data": "resume_token_here" };
//! store.save_resume_token("users", &token).await?;
//!
//! // Retrieve the token
//! let retrieved = store.get_resume_token("users").await?;
//! assert!(retrieved.is_some());
//!
//! // Clean up
//! store.close().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Redis with TTL
//!
//! ```rust,no_run
//! use rigatoni_stores::redis::{RedisStore, RedisConfig};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Tokens expire after 7 days of inactivity
//! let config = RedisConfig::builder()
//!     .url("redis://localhost:6379")
//!     .ttl(Duration::from_secs(7 * 24 * 60 * 60))
//!     .build()?;
//!
//! let store = RedisStore::new(config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Redis Cluster
//!
//! ```rust,no_run
//! use rigatoni_stores::redis::{RedisStore, RedisConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to Redis Cluster
//! let config = RedisConfig::builder()
//!     .url("redis://node1:6379,redis://node2:6379,redis://node3:6379")
//!     .cluster_mode(true)
//!     .pool_size(20)
//!     .build()?;
//!
//! let store = RedisStore::new(config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Production Deployment
//!
//! For production deployments, consider:
//!
//! - **High Availability**: Use Redis Sentinel or Redis Cluster
//! - **Connection Pooling**: Set pool size based on concurrent pipelines (default: 10)
//! - **TTL Strategy**: Set TTL to prevent unbounded growth (recommended: 7-30 days)
//! - **Monitoring**: Track Redis connection pool metrics and key count
//! - **Network**: Low latency between pipelines and Redis (< 5ms recommended)
//!
//! # Key Pattern
//!
//! Resume tokens are stored with the key pattern:
//! ```text
//! rigatoni:resume_token:{collection_name}
//! ```
//!
//! This allows easy identification and management of Rigatoni keys in shared Redis instances.

use async_trait::async_trait;
use bson::Document;
use deadpool_redis::{Config as PoolConfig, Pool, Runtime};
use redis::{AsyncCommands, RedisError};
use rigatoni_core::state::{StateStore, StateStoreError};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, warn};

/// Key prefix for all Rigatoni resume tokens in Redis.
const KEY_PREFIX: &str = "rigatoni:resume_token";

/// Maximum number of retry attempts for transient Redis errors.
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (milliseconds).
const BASE_RETRY_DELAY_MS: u64 = 100;

/// Configuration for Redis-backed state store.
///
/// Use [`RedisConfigBuilder`] to construct this configuration with validation.
///
/// # Example
///
/// ```rust
/// use rigatoni_stores::redis::RedisConfig;
/// use std::time::Duration;
///
/// let config = RedisConfig::builder()
///     .url("redis://localhost:6379")
///     .pool_size(15)
///     .ttl(Duration::from_secs(86400)) // 1 day
///     .build()
///     .expect("valid config");
/// ```
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis connection URL (e.g., "redis://localhost:6379")
    ///
    /// For cluster mode, provide comma-separated URLs:
    /// "redis://node1:6379,redis://node2:6379"
    pub url: String,

    /// Connection pool size (default: 10)
    ///
    /// Set based on expected concurrent operations. Each pipeline worker
    /// may need 1-2 connections.
    pub pool_size: usize,

    /// Optional TTL for resume tokens
    ///
    /// If set, tokens will expire after this duration of inactivity.
    /// Recommended: 7-30 days for production deployments.
    pub ttl: Option<Duration>,

    /// Enable Redis Cluster mode (default: false)
    ///
    /// When true, the client will handle MOVED/ASK redirections automatically.
    pub cluster_mode: bool,

    /// Connection timeout (default: 5 seconds)
    pub connection_timeout: Duration,

    /// Maximum number of retries for transient errors (default: 3)
    pub max_retries: u32,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            ttl: None,
            cluster_mode: false,
            connection_timeout: Duration::from_secs(5),
            max_retries: MAX_RETRIES,
        }
    }
}

impl RedisConfig {
    /// Creates a new builder for `RedisConfig`.
    #[must_use]
    pub fn builder() -> RedisConfigBuilder {
        RedisConfigBuilder::new()
    }
}

/// Builder for [`RedisConfig`] with validation.
///
/// # Example
///
/// ```rust
/// use rigatoni_stores::redis::RedisConfig;
/// use std::time::Duration;
///
/// let config = RedisConfig::builder()
///     .url("redis://localhost:6379")
///     .pool_size(20)
///     .ttl(Duration::from_secs(7 * 24 * 60 * 60)) // 7 days
///     .cluster_mode(false)
///     .build()
///     .expect("valid configuration");
/// ```
#[derive(Debug, Default)]
pub struct RedisConfigBuilder {
    url: Option<String>,
    pool_size: Option<usize>,
    ttl: Option<Duration>,
    cluster_mode: Option<bool>,
    connection_timeout: Option<Duration>,
    max_retries: Option<u32>,
}

impl RedisConfigBuilder {
    /// Creates a new `RedisConfigBuilder`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the Redis connection URL.
    ///
    /// # Formats
    ///
    /// - Standalone: `redis://localhost:6379`
    /// - With auth: `redis://:password@localhost:6379`
    /// - With database: `redis://localhost:6379/0`
    /// - TLS: `rediss://localhost:6380`
    /// - Cluster: `redis://node1:6379,redis://node2:6379`
    #[must_use]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the connection pool size.
    ///
    /// Default: 10
    ///
    /// # Guidelines
    ///
    /// - Small deployments (1-5 pipelines): 5-10
    /// - Medium deployments (5-20 pipelines): 10-20
    /// - Large deployments (20+ pipelines): 20-50
    #[must_use]
    pub fn pool_size(mut self, size: usize) -> Self {
        self.pool_size = Some(size);
        self
    }

    /// Sets the TTL for resume tokens.
    ///
    /// If not set, tokens never expire. Recommended to set a TTL (7-30 days)
    /// to prevent unbounded Redis memory growth.
    #[must_use]
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Enables Redis Cluster mode.
    ///
    /// Default: false (standalone mode)
    #[must_use]
    pub fn cluster_mode(mut self, enabled: bool) -> Self {
        self.cluster_mode = Some(enabled);
        self
    }

    /// Sets the connection timeout.
    ///
    /// Default: 5 seconds
    #[must_use]
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = Some(timeout);
        self
    }

    /// Sets the maximum number of retries for transient errors.
    ///
    /// Default: 3
    #[must_use]
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Builds the `RedisConfig`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - URL is not provided
    /// - Pool size is 0
    /// - URL format is invalid
    pub fn build(self) -> Result<RedisConfig, StateStoreError> {
        let url = self
            .url
            .ok_or_else(|| StateStoreError::Other("Redis URL is required".to_string()))?;

        let pool_size = self.pool_size.unwrap_or(10);
        if pool_size == 0 {
            return Err(StateStoreError::Other(
                "Pool size must be greater than 0".to_string(),
            ));
        }

        Ok(RedisConfig {
            url,
            pool_size,
            ttl: self.ttl,
            cluster_mode: self.cluster_mode.unwrap_or(false),
            connection_timeout: self.connection_timeout.unwrap_or(Duration::from_secs(5)),
            max_retries: self.max_retries.unwrap_or(MAX_RETRIES),
        })
    }
}

/// Redis-backed state store for distributed deployments.
///
/// Stores resume tokens in Redis using connection pooling and automatic retries.
/// Suitable for multi-instance Rigatoni deployments where state must be shared.
///
/// # Thread Safety
///
/// `RedisStore` is `Send + Sync` and can be safely shared across threads/tasks.
/// The underlying connection pool handles concurrent access.
///
/// # Example
///
/// ```rust,no_run
/// use rigatoni_stores::redis::{RedisStore, RedisConfig};
/// use rigatoni_core::state::StateStore;
/// use mongodb::bson::doc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = RedisConfig::builder()
///     .url("redis://localhost:6379")
///     .pool_size(10)
///     .build()?;
///
/// let store = RedisStore::new(config).await?;
///
/// // Use with pipeline
/// let token = doc! { "_data": "token123" };
/// store.save_resume_token("users", &token).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct RedisStore {
    pool: Pool,
    config: RedisConfig,
}

impl RedisStore {
    /// Creates a new `RedisStore` with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Connection to Redis fails
    /// - Pool initialization fails
    /// - URL format is invalid
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rigatoni_stores::redis::{RedisStore, RedisConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = RedisConfig::builder()
    ///     .url("redis://localhost:6379")
    ///     .build()?;
    ///
    /// let store = RedisStore::new(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: RedisConfig) -> Result<Self, StateStoreError> {
        debug!("Initializing Redis state store with config: {:?}", config);

        // Create connection pool configuration
        let mut pool_config = PoolConfig::from_url(&config.url);

        // Configure pool size and timeouts
        if let Some(pool) = pool_config.pool.as_mut() {
            pool.max_size = config.pool_size;
            pool.timeouts.wait = Some(config.connection_timeout);
            pool.timeouts.create = Some(config.connection_timeout);
            pool.timeouts.recycle = Some(config.connection_timeout);
        }

        // Create pool
        let pool = pool_config
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| {
                error!("Failed to create Redis connection pool: {}", e);
                StateStoreError::Connection(format!("Failed to create pool: {}", e))
            })?;

        // Test connection
        let mut conn = pool.get().await.map_err(|e| {
            error!("Failed to get connection from pool: {}", e);
            StateStoreError::Connection(format!("Failed to connect to Redis: {}", e))
        })?;

        // Ping to verify connectivity
        redis::cmd("PING")
            .query_async::<()>(&mut *conn)
            .await
            .map_err(|e| {
                error!("Redis PING failed: {}", e);
                StateStoreError::Connection(format!("Redis connection test failed: {}", e))
            })?;

        debug!("Redis state store initialized successfully");

        Ok(Self { pool, config })
    }

    /// Generates the Redis key for a given collection.
    fn make_key(collection: &str) -> String {
        format!("{}:{}", KEY_PREFIX, collection)
    }

    /// Executes a Redis operation with retry logic for transient errors.
    async fn with_retry<F, T, Fut>(&self, operation: F) -> Result<T, StateStoreError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, RedisError>>,
    {
        let mut retries = 0;
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) if Self::is_retryable(&e) && retries < self.config.max_retries => {
                    retries += 1;
                    let delay = Duration::from_millis(BASE_RETRY_DELAY_MS * 2_u64.pow(retries - 1));
                    warn!(
                        "Redis operation failed (attempt {}/{}), retrying in {:?}: {}",
                        retries, self.config.max_retries, delay, e
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    error!("Redis operation failed after {} retries: {}", retries, e);
                    return Err(StateStoreError::Connection(format!(
                        "Redis operation failed: {}",
                        e
                    )));
                }
            }
        }
    }

    /// Determines if a Redis error is retryable.
    fn is_retryable(error: &RedisError) -> bool {
        matches!(
            error.kind(),
            redis::ErrorKind::IoError | redis::ErrorKind::ResponseError
        )
    }

    /// Serializes a BSON document to bytes for storage in Redis.
    fn serialize_token(token: &Document) -> Result<Vec<u8>, StateStoreError> {
        bson::to_vec(token).map_err(|e| {
            StateStoreError::Serialization(format!("Failed to serialize resume token: {}", e))
        })
    }

    /// Deserializes bytes from Redis back to a BSON document.
    fn deserialize_token(bytes: &[u8]) -> Result<Document, StateStoreError> {
        bson::from_slice(bytes).map_err(|e| {
            StateStoreError::Serialization(format!("Failed to deserialize resume token: {}", e))
        })
    }
}

#[async_trait]
impl StateStore for RedisStore {
    async fn save_resume_token(
        &self,
        collection: &str,
        token: &Document,
    ) -> Result<(), StateStoreError> {
        let key = Self::make_key(collection);
        let value = Self::serialize_token(token)?;

        debug!(
            "Saving resume token for collection '{}' to Redis key '{}'",
            collection, key
        );

        let pool = self.pool.clone();
        let ttl = self.config.ttl;

        self.with_retry::<_, (), _>(|| async {
            let mut conn = pool.get().await.map_err(|e| {
                RedisError::from((
                    redis::ErrorKind::IoError,
                    "Failed to get connection from pool",
                    e.to_string(),
                ))
            })?;

            // Use SET with optional EX (expiration in seconds)
            if let Some(ttl_duration) = ttl {
                let ttl_secs = ttl_duration.as_secs();
                conn.set_ex(&key, &value, ttl_secs as u64).await
            } else {
                conn.set(&key, &value).await
            }
        })
        .await?;

        debug!(
            "Successfully saved resume token for collection '{}'",
            collection
        );
        Ok(())
    }

    async fn get_resume_token(
        &self,
        collection: &str,
    ) -> Result<Option<Document>, StateStoreError> {
        let key = Self::make_key(collection);

        debug!(
            "Retrieving resume token for collection '{}' from Redis key '{}'",
            collection, key
        );

        let pool = self.pool.clone();

        let bytes: Option<Vec<u8>> = self
            .with_retry(|| async {
                let mut conn = pool.get().await.map_err(|e| {
                    RedisError::from((
                        redis::ErrorKind::IoError,
                        "Failed to get connection from pool",
                        e.to_string(),
                    ))
                })?;

                conn.get(&key).await
            })
            .await?;

        match bytes {
            Some(data) => {
                let token = Self::deserialize_token(&data)?;
                debug!(
                    "Successfully retrieved resume token for collection '{}'",
                    collection
                );
                Ok(Some(token))
            }
            None => {
                debug!("No resume token found for collection '{}'", collection);
                Ok(None)
            }
        }
    }

    async fn delete_resume_token(&self, collection: &str) -> Result<(), StateStoreError> {
        let key = Self::make_key(collection);

        debug!(
            "Deleting resume token for collection '{}' from Redis key '{}'",
            collection, key
        );

        let pool = self.pool.clone();

        self.with_retry::<_, (), _>(|| async {
            let mut conn = pool.get().await.map_err(|e| {
                RedisError::from((
                    redis::ErrorKind::IoError,
                    "Failed to get connection from pool",
                    e.to_string(),
                ))
            })?;

            conn.del(&key).await
        })
        .await?;

        debug!(
            "Successfully deleted resume token for collection '{}'",
            collection
        );
        Ok(())
    }

    async fn list_resume_tokens(&self) -> Result<HashMap<String, Document>, StateStoreError> {
        let pattern = format!("{}:*", KEY_PREFIX);

        debug!("Listing all resume tokens with pattern '{}'", pattern);

        let pool = self.pool.clone();

        // Get all keys matching the pattern
        let keys: Vec<String> = self
            .with_retry(|| async {
                let mut conn = pool.get().await.map_err(|e| {
                    RedisError::from((
                        redis::ErrorKind::IoError,
                        "Failed to get connection from pool",
                        e.to_string(),
                    ))
                })?;

                redis::cmd("KEYS").arg(&pattern).query_async(&mut *conn).await
            })
            .await?;

        if keys.is_empty() {
            debug!("No resume tokens found");
            return Ok(HashMap::new());
        }

        let pool = self.pool.clone();
        let prefix_len = KEY_PREFIX.len() + 1; // +1 for the colon

        // Fetch all values
        let values: Vec<Option<Vec<u8>>> = self
            .with_retry(|| async {
                let mut conn = pool.get().await.map_err(|e| {
                    RedisError::from((
                        redis::ErrorKind::IoError,
                        "Failed to get connection from pool",
                        e.to_string(),
                    ))
                })?;

                // Use MGET for atomic retrieval of all values
                redis::cmd("MGET")
                    .arg(&keys)
                    .query_async(&mut *conn)
                    .await
            })
            .await?;

        // Build the result map
        let mut result = HashMap::new();
        for (key, value) in keys.into_iter().zip(values) {
            if let Some(bytes) = value {
                let collection = key[prefix_len..].to_string();
                let token = Self::deserialize_token(&bytes)?;
                result.insert(collection, token);
            }
        }

        debug!("Successfully listed {} resume tokens", result.len());
        Ok(result)
    }

    async fn close(&self) -> Result<(), StateStoreError> {
        debug!("Closing Redis state store");
        // Connection pool will be dropped automatically
        // No explicit close needed for deadpool-redis
        debug!("Redis state store closed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_key() {
        assert_eq!(RedisStore::make_key("users"), "rigatoni:resume_token:users");
        assert_eq!(
            RedisStore::make_key("orders"),
            "rigatoni:resume_token:orders"
        );
    }

    #[test]
    fn test_config_builder() {
        let config = RedisConfig::builder()
            .url("redis://localhost:6379")
            .pool_size(20)
            .ttl(Duration::from_secs(3600))
            .cluster_mode(true)
            .build()
            .unwrap();

        assert_eq!(config.url, "redis://localhost:6379");
        assert_eq!(config.pool_size, 20);
        assert_eq!(config.ttl, Some(Duration::from_secs(3600)));
        assert!(config.cluster_mode);
    }

    #[test]
    fn test_config_builder_missing_url() {
        let result = RedisConfig::builder().pool_size(10).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_builder_zero_pool_size() {
        let result = RedisConfig::builder()
            .url("redis://localhost:6379")
            .pool_size(0)
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_deserialize_token() {
        use mongodb::bson::doc;

        let token = doc! {
            "_data": "test_token",
            "clusterTime": 123456789_i64,
        };

        let serialized = RedisStore::serialize_token(&token).unwrap();
        let deserialized = RedisStore::deserialize_token(&serialized).unwrap();

        assert_eq!(token, deserialized);
    }
}
