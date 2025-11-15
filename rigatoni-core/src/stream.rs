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

//! `MongoDB` Change Stream Wrapper
//!
//! This module provides an ergonomic async wrapper around `MongoDB` change streams with:
//! - Automatic resume token management
//! - Reconnection with exponential backoff
//! - Graceful error handling
//! - Pipeline filtering support
//! - Pre/post-image configuration
//!
//! # Architecture
//!
//! The [`ChangeStreamListener`] implements the [`Stream`] trait from `futures`,
//! making it composable with other async stream utilities. It wraps MongoDB's
//! native change stream and adds production-ready reliability features.
//!
//! ## Resume Token Flow
//!
//! ```text
//! ┌─────────────────┐
//! │ MongoDB Cluster │
//! └────────┬────────┘
//!          │ Change Events
//!          ▼
//! ┌──────────────────────┐
//! │ ChangeStreamListener │◄─── Resume Token Callback
//! └──────────────────────┘      (persists to storage)
//!          │
//!          │ ChangeEvent
//!          ▼
//! ┌─────────────────┐
//! │ Pipeline Stage  │
//! └─────────────────┘
//! ```
//!
//! ## Reconnection Algorithm
//!
//! When a connection error occurs:
//! 1. Load the last persisted resume token
//! 2. Attempt reconnection with exponential backoff: 100ms, 200ms, 400ms, ...
//! 3. Max backoff capped at `max_backoff_ms`
//! 4. Fail after `max_reconnect_attempts` attempts
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use rigatoni_core::stream::{ChangeStreamListener, ChangeStreamConfig};
//! use mongodb::{Client, options::ClientOptions};
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to MongoDB
//! let client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
//! let client = Client::with_options(client_options)?;
//! let collection = client.database("mydb").collection("users");
//!
//! // Configure change stream
//! let config = ChangeStreamConfig::builder()
//!     .full_document_before_change()  // Enable pre-images
//!     .full_document_update_lookup()  // Include full document on updates
//!     .max_reconnect_attempts(10)
//!     .build()?;
//!
//! // Create listener with resume token callback
//! let mut listener = ChangeStreamListener::new(
//!     collection,
//!     config,
//!     |token| {
//!         // Persist resume token to storage
//!         println!("Saving resume token: {:?}", token);
//!         Box::pin(async { Ok(()) })
//!     },
//! ).await?;
//!
//! // Consume events
//! while let Some(result) = listener.next().await {
//!     match result {
//!         Ok(ackable) => {
//!             let event = ackable.event_ref();
//!             println!("Received: {:?}", event.operation);
//!             ackable.ack(); // Acknowledge after processing
//!         }
//!         Err(e) => {
//!             eprintln!("Stream error: {}", e);
//!             break;
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## With Pipeline Filtering
//!
//! ```rust,no_run
//! use rigatoni_core::stream::{ChangeStreamListener, ChangeStreamConfig};
//! use bson::doc;
//! use futures::StreamExt;
//! # use mongodb::{Client, options::ClientOptions};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let client = Client::with_options(ClientOptions::parse("mongodb://localhost:27017").await?)?;
//! # let collection = client.database("mydb").collection("users");
//!
//! // Only watch insert and update operations
//! let pipeline = vec![
//!     doc! {
//!         "$match": {
//!             "operationType": { "$in": ["insert", "update"] }
//!         }
//!     }
//! ];
//!
//! let config = ChangeStreamConfig::builder()
//!     .pipeline(pipeline)
//!     .build()?;
//!
//! let mut listener = ChangeStreamListener::new(
//!     collection,
//!     config,
//!     |_| Box::pin(async { Ok(()) }),
//! ).await?;
//!
//! while let Some(result) = listener.next().await {
//!     let ackable = result?;
//!     ackable.ack();
//!     // Only insert/update events will be received
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Multi-Collection Scenario
//!
//! ```rust,no_run
//! use rigatoni_core::stream::ChangeStreamListener;
//! use futures::StreamExt;
//! use tokio::task::JoinSet;
//! # use mongodb::{Client, options::ClientOptions};
//! # use rigatoni_core::stream::ChangeStreamConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! # let client = Client::with_options(ClientOptions::parse("mongodb://localhost:27017").await?)?;
//! let db = client.database("mydb");
//!
//! let mut tasks = JoinSet::new();
//!
//! // Watch multiple collections concurrently
//! for collection_name in &["users", "orders", "products"] {
//!     let collection = db.collection(collection_name);
//!     let config = ChangeStreamConfig::default();
//!
//!     tasks.spawn(async move {
//!         let mut listener = ChangeStreamListener::new(
//!             collection,
//!             config,
//!             |_| Box::pin(async { Ok(()) }),
//!         ).await?;
//!
//!         while let Some(result) = listener.next().await {
//!             let ackable = result?;
//!             println!("{}: {:?}", collection_name, ackable.event_ref().operation);
//!             ackable.ack();
//!         }
//!
//!         Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
//!     });
//! }
//!
//! // Wait for all streams
//! while let Some(result) = tasks.join_next().await {
//!     result??;
//! }
//! # Ok(())
//! # }
//! ```

use crate::event::{ChangeEvent, ConversionError};
use bson::Document;
use futures::Stream;
use mongodb::{
    change_stream::event::{ChangeStreamEvent, ResumeToken},
    error::{Error as MongoError, ErrorKind as MongoErrorKind},
    options::ChangeStreamOptions,
    Collection,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Errors that can occur during change stream operations.
#[derive(Debug, Error)]
pub enum StreamError {
    /// MongoDB connection error (may be retryable)
    #[error("Connection error: {message}")]
    Connection {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
        /// MongoDB error code
        code: Option<i32>,
        /// MongoDB error labels (e.g., "RetryableWriteError")
        labels: Vec<String>,
    },

    /// Failed to convert MongoDB event to ChangeEvent
    #[error("Event conversion failed: {0}")]
    Conversion(#[from] ConversionError),

    /// Resume token persistence failed
    #[error("Resume token persistence failed: {0}")]
    ResumeTokenPersistence(String),

    /// Stream was invalidated (collection dropped/renamed)
    #[error("Stream invalidated: {reason}")]
    Invalidated { reason: String },

    /// Maximum reconnection attempts exceeded
    #[error("Max reconnection attempts ({0}) exceeded")]
    MaxReconnectAttemptsExceeded(u32),

    /// Resume token is invalid or oplog truncated (error code 286)
    #[error("Invalid resume token (code {code}): oplog may be truncated")]
    InvalidResumeToken { code: i32 },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
}

impl From<MongoError> for StreamError {
    fn from(err: MongoError) -> Self {
        Self::from_mongo_error(err)
    }
}

impl StreamError {
    /// Creates a StreamError from a MongoDB error with proper classification.
    ///
    /// This method checks error codes and labels for accurate categorization.
    pub fn from_mongo_error(err: MongoError) -> Self {
        // Extract error code
        let code = match err.kind.as_ref() {
            MongoErrorKind::Command(cmd_err) => Some(cmd_err.code),
            _ => None,
        };

        // Check for invalid resume token (code 286: ChangeStreamFatalError)
        if code == Some(286) {
            return Self::InvalidResumeToken { code: 286 };
        }

        // Extract labels from the error
        // Note: In MongoDB Rust driver, labels() method returns &HashSet<String>
        let labels: Vec<String> = err.labels().iter().cloned().collect();

        Self::Connection {
            message: err.to_string(),
            source: Some(Box::new(err)),
            code,
            labels,
        }
    }

    /// Returns true if this error is retryable with reconnection.
    ///
    /// Uses MongoDB error codes and labels for accurate classification:
    /// - Error labels: RetryableWriteError, TransientTransactionError, NetworkError
    /// - Transient error codes: 6, 7, 43, 89, 91, 10107, 11600, 11602, 13435, 13436
    ///
    /// Non-retryable errors:
    /// - Invalid resume token (286)
    /// - Stream invalidated (collection dropped)
    /// - Authentication/authorization errors
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Connection { code, labels, .. } => {
                // Check error labels first (most reliable)
                if labels.iter().any(|l| {
                    l == "RetryableWriteError"
                        || l == "TransientTransactionError"
                        || l == "NetworkError"
                }) {
                    return true;
                }

                // Check specific transient error codes
                if let Some(c) = code {
                    matches!(
                        c,
                        // Network errors
                        6 |    // HostUnreachable
                        7 |    // HostNotFound
                        89 |   // NetworkTimeout
                        91 |   // ShutdownInProgress
                        // Replication errors (transient during failover)
                        10107 | // NotMaster / NotPrimary
                        11600 | // InterruptedAtShutdown
                        11602 | // InterruptedDueToReplStateChange
                        13435 | // NotMasterNoSlaveOk
                        13436 | // NotMasterOrSecondary / NotPrimaryOrSecondary
                        // Cursor errors (can retry with resume token)
                        43 // CursorNotFound
                    )
                } else {
                    // No code available, be conservative
                    false
                }
            }
            Self::Invalidated { .. } => false,
            Self::InvalidResumeToken { .. } => false,
            Self::MaxReconnectAttemptsExceeded(_) => false,
            Self::Conversion(_) => false,
            Self::ResumeTokenPersistence(_) => false,
            Self::Configuration(_) => false,
        }
    }

    /// Returns the error category for metrics/logging.
    #[must_use]
    pub fn category(&self) -> &'static str {
        match self {
            Self::Connection { .. } => "connection",
            Self::Conversion(_) => "conversion",
            Self::ResumeTokenPersistence(_) => "persistence",
            Self::Invalidated { .. } => "invalidated",
            Self::MaxReconnectAttemptsExceeded(_) => "max_retries",
            Self::InvalidResumeToken { .. } => "invalid_token",
            Self::Configuration(_) => "configuration",
        }
    }
}

/// Event with acknowledgment capability for correct resume token semantics.
///
/// This type ensures that resume tokens are persisted AFTER the user successfully
/// processes the event, providing at-least-once delivery semantics.
///
/// # Important
///
/// You MUST call `.ack()` after successfully processing the event. If you don't
/// call `.ack()`, the resume token will not be persisted, and the stream cannot
/// reliably resume after a crash.
///
/// # Examples
///
/// ```rust,no_run
/// use rigatoni_core::stream::{ChangeStreamListener, ChangeStreamConfig};
/// use futures::StreamExt;
/// # use mongodb::{Client, options::ClientOptions};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// # let client = Client::with_options(ClientOptions::parse("mongodb://localhost:27017").await?)?;
/// # let collection = client.database("mydb").collection("test");
/// let mut listener = ChangeStreamListener::new(
///     collection,
///     ChangeStreamConfig::default(),
///     |_| Box::pin(async { Ok(()) }),
/// ).await?;
///
/// while let Some(ackable) = listener.next().await {
///     let ackable = ackable?;
///
///     // Access the event
///     let event = ackable.event_ref();
///     println!("Processing: {:?}", event.operation);
///
///     // Process the event
///     process_event(event).await?;
///
///     // IMPORTANT: Acknowledge after successful processing
///     ackable.ack();
/// }
/// # Ok(())
/// # }
/// # async fn process_event(event: &rigatoni_core::event::ChangeEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> { Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct AckableEvent {
    /// The change event
    pub event: ChangeEvent,

    /// Resume token for this event
    resume_token: Document,

    /// Channel to send ack
    ack_sender: mpsc::UnboundedSender<Document>,

    /// Shared reference to last resume token (updated on ack)
    last_resume_token: Arc<Mutex<Option<Document>>>,
}

impl AckableEvent {
    /// Acknowledge processing of this event, persisting the resume token.
    ///
    /// Call this AFTER successfully processing the event to ensure at-least-once semantics.
    /// The token will be sent to a background persistence task asynchronously.
    ///
    /// This method also updates the internal last_resume_token used for reconnection,
    /// ensuring that reconnection only resumes from successfully processed events.
    ///
    /// If the persistence task has stopped, this will log a warning but not fail.
    pub fn ack(self) {
        // Update last resume token for reconnection (only after user confirms processing)
        if let Ok(mut last_token) = self.last_resume_token.lock() {
            *last_token = Some(self.resume_token.clone());
        }

        // Send token for persistence
        if self.ack_sender.send(self.resume_token).is_err() {
            warn!("Failed to send ack - persistence task may have stopped");
        }
    }

    /// Get a reference to the change event without consuming the `AckableEvent`.
    ///
    /// Use this to access the event data while keeping the ability to call `.ack()` later.
    #[must_use]
    pub fn event_ref(&self) -> &ChangeEvent {
        &self.event
    }

    /// Consume the `AckableEvent` and return the inner `ChangeEvent`.
    ///
    /// # Warning
    ///
    /// After calling this, you cannot call `.ack()`. Only use this if you don't
    /// want to persist the resume token.
    #[must_use]
    pub fn into_event(self) -> ChangeEvent {
        self.event
    }
}

/// Configuration for change stream behavior.
///
/// Use [`ChangeStreamConfigBuilder`] to construct instances:
///
/// ```rust
/// use rigatoni_core::stream::ChangeStreamConfig;
///
/// let config = ChangeStreamConfig::builder()
///     .full_document_update_lookup()
///     .max_reconnect_attempts(10)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct ChangeStreamConfig {
    /// MongoDB aggregation pipeline to filter events
    pub pipeline: Vec<Document>,

    /// Whether to include full document for update operations
    pub full_document_on_update: bool,

    /// Whether to include document before change (requires MongoDB 6.0+)
    pub full_document_before_change: bool,

    /// Initial backoff in milliseconds
    pub initial_backoff_ms: u64,

    /// Maximum backoff in milliseconds
    pub max_backoff_ms: u64,

    /// Maximum number of reconnection attempts (0 = infinite)
    pub max_reconnect_attempts: u32,

    /// Batch size for fetching events
    pub batch_size: Option<u32>,

    /// Resume token to start from (if any)
    pub resume_token: Option<Document>,

    /// Backoff jitter factor (0.0 to 1.0)
    /// Default: 0.1 (10% jitter)
    pub backoff_jitter: f64,
}

impl Default for ChangeStreamConfig {
    fn default() -> Self {
        Self {
            pipeline: Vec::new(),
            full_document_on_update: false,
            full_document_before_change: false,
            initial_backoff_ms: 100,
            max_backoff_ms: 30_000, // 30 seconds
            max_reconnect_attempts: 5,
            batch_size: None,
            resume_token: None,
            backoff_jitter: 0.1, // 10% jitter to prevent thundering herd
        }
    }
}

impl ChangeStreamConfig {
    /// Creates a new builder for configuring a change stream.
    #[must_use]
    pub fn builder() -> ChangeStreamConfigBuilder {
        ChangeStreamConfigBuilder::default()
    }

    /// Validates the configuration.
    ///
    /// Returns an error if:
    /// - `initial_backoff_ms` is 0
    /// - `initial_backoff_ms` > `max_backoff_ms`
    /// - `backoff_jitter` is not in range [0.0, 1.0]
    pub fn validate(&self) -> Result<(), StreamError> {
        if self.initial_backoff_ms == 0 {
            return Err(StreamError::Configuration(
                "initial_backoff_ms must be greater than 0".to_string(),
            ));
        }

        if self.initial_backoff_ms > self.max_backoff_ms {
            return Err(StreamError::Configuration(format!(
                "initial_backoff_ms ({}) must be <= max_backoff_ms ({})",
                self.initial_backoff_ms, self.max_backoff_ms
            )));
        }

        if !(0.0..=1.0).contains(&self.backoff_jitter) {
            return Err(StreamError::Configuration(format!(
                "backoff_jitter ({}) must be between 0.0 and 1.0",
                self.backoff_jitter
            )));
        }

        Ok(())
    }

    /// Calculates backoff duration with jitter for the given attempt.
    ///
    /// Uses exponential backoff: `initial_ms * 2^(attempt-1)`, capped at `max_backoff_ms`.
    /// Adds random jitter to prevent thundering herd: ±(base * jitter_factor / 2).
    fn calculate_backoff(&self, attempt: u32) -> Duration {
        // Exponential backoff: initial * 2^(attempt-1)
        let base_ms = self
            .initial_backoff_ms
            .saturating_mul(1_u64 << attempt.saturating_sub(1))
            .min(self.max_backoff_ms);

        // Add jitter: random value in range [base * (1 - jitter/2), base * (1 + jitter/2)]
        if self.backoff_jitter > 0.0 {
            let jitter_range = (base_ms as f64) * self.backoff_jitter;
            let jitter = (rand::random::<f64>() * jitter_range) - (jitter_range / 2.0);
            let final_ms = ((base_ms as f64) + jitter).max(0.0) as u64;
            Duration::from_millis(final_ms)
        } else {
            Duration::from_millis(base_ms)
        }
    }

    /// Converts this config to MongoDB's `ChangeStreamOptions`.
    fn to_mongo_options(&self) -> ChangeStreamOptions {
        let mut options = ChangeStreamOptions::default();

        // Set full document options
        if self.full_document_on_update {
            options.full_document = Some(mongodb::options::FullDocumentType::UpdateLookup);
        }

        if self.full_document_before_change {
            options.full_document_before_change =
                Some(mongodb::options::FullDocumentBeforeChangeType::WhenAvailable);
        }

        // Set batch size
        options.batch_size = self.batch_size;

        // Set resume token if provided
        // Note: We store resume tokens as Document for persistence,
        // but MongoDB expects ResumeToken. We'll deserialize from BSON bytes.
        if let Some(ref token_doc) = self.resume_token {
            // Serialize Document to bytes, then deserialize as ResumeToken
            if let Ok(bytes) = bson::to_vec(token_doc) {
                if let Ok(resume_token) = bson::from_slice::<ResumeToken>(&bytes) {
                    options.resume_after = Some(resume_token);
                }
            }
        }

        options
    }
}

/// Builder for [`ChangeStreamConfig`].
#[derive(Debug, Default)]
pub struct ChangeStreamConfigBuilder {
    pipeline: Vec<Document>,
    full_document_on_update: bool,
    full_document_before_change: bool,
    initial_backoff_ms: Option<u64>,
    max_backoff_ms: Option<u64>,
    max_reconnect_attempts: Option<u32>,
    batch_size: Option<u32>,
    resume_token: Option<Document>,
    backoff_jitter: Option<f64>,
}

impl ChangeStreamConfigBuilder {
    /// Sets the aggregation pipeline for filtering events.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_core::stream::ChangeStreamConfig;
    /// use bson::doc;
    ///
    /// let config = ChangeStreamConfig::builder()
    ///     .pipeline(vec![
    ///         doc! { "$match": { "operationType": "insert" } }
    ///     ])
    ///     .build();
    /// ```
    #[must_use]
    pub fn pipeline(mut self, pipeline: Vec<Document>) -> Self {
        self.pipeline = pipeline;
        self
    }

    /// Enables full document retrieval for update operations.
    ///
    /// When enabled, MongoDB will include the entire updated document
    /// in the change event (requires additional lookup).
    #[must_use]
    pub fn full_document_update_lookup(mut self) -> Self {
        self.full_document_on_update = true;
        self
    }

    /// Enables pre-image for change events (requires MongoDB 6.0+).
    ///
    /// When enabled, the document state before the change will be included.
    /// Requires pre-image collection to be configured on the collection.
    #[must_use]
    pub fn full_document_before_change(mut self) -> Self {
        self.full_document_before_change = true;
        self
    }

    /// Sets the initial backoff duration in milliseconds.
    ///
    /// Default: 100ms
    #[must_use]
    pub fn initial_backoff_ms(mut self, ms: u64) -> Self {
        self.initial_backoff_ms = Some(ms);
        self
    }

    /// Sets the maximum backoff duration in milliseconds.
    ///
    /// Default: 30,000ms (30 seconds)
    #[must_use]
    pub fn max_backoff_ms(mut self, ms: u64) -> Self {
        self.max_backoff_ms = Some(ms);
        self
    }

    /// Sets the maximum number of reconnection attempts.
    ///
    /// Set to 0 for infinite retries (use with caution).
    /// Default: 5
    #[must_use]
    pub fn max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.max_reconnect_attempts = Some(attempts);
        self
    }

    /// Sets the batch size for fetching events from MongoDB.
    ///
    /// Larger batches can improve throughput but increase memory usage.
    #[must_use]
    pub fn batch_size(mut self, size: u32) -> Self {
        self.batch_size = Some(size);
        self
    }

    /// Sets a resume token to start streaming from.
    ///
    /// If provided, the stream will resume from this point instead
    /// of starting from the current time.
    #[must_use]
    pub fn resume_token(mut self, token: Document) -> Self {
        self.resume_token = Some(token);
        self
    }

    /// Sets the backoff jitter factor (0.0 to 1.0).
    ///
    /// Jitter adds randomness to backoff delays to prevent thundering herd.
    /// A value of 0.1 means ±10% randomness.
    ///
    /// Default: 0.1
    #[must_use]
    pub fn backoff_jitter(mut self, jitter: f64) -> Self {
        self.backoff_jitter = Some(jitter);
        self
    }

    /// Builds the configuration.
    ///
    /// # Errors
    ///
    /// Returns `StreamError::Configuration` if validation fails:
    /// - `initial_backoff_ms` must be > 0
    /// - `initial_backoff_ms` must be <= `max_backoff_ms`
    /// - `backoff_jitter` must be between 0.0 and 1.0
    pub fn build(self) -> Result<ChangeStreamConfig, StreamError> {
        let config = ChangeStreamConfig {
            pipeline: self.pipeline,
            full_document_on_update: self.full_document_on_update,
            full_document_before_change: self.full_document_before_change,
            initial_backoff_ms: self.initial_backoff_ms.unwrap_or(100),
            max_backoff_ms: self.max_backoff_ms.unwrap_or(30_000),
            max_reconnect_attempts: self.max_reconnect_attempts.unwrap_or(5),
            batch_size: self.batch_size,
            resume_token: self.resume_token,
            backoff_jitter: self.backoff_jitter.unwrap_or(0.1),
        };

        config.validate()?;
        Ok(config)
    }
}

/// Callback for persisting resume tokens.
///
/// This callback is invoked after each successfully processed event.
/// It should persist the token to durable storage (database, file, etc.)
/// to enable resuming after crashes or restarts.
///
/// # Returns
///
/// - `Ok(())` if token was successfully persisted
/// - `Err(String)` if persistence failed (will propagate as StreamError)
///
/// # Example
///
/// ```rust
/// use bson::Document;
/// use std::future::Future;
/// use std::pin::Pin;
///
/// fn save_to_redis(
///     token: Document,
/// ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
///     Box::pin(async move {
///         // Save to Redis
///         // redis_client.set("resume_token", token).await
///         //     .map_err(|e| e.to_string())?;
///         Ok(())
///     })
/// }
/// ```
pub type ResumeTokenCallback =
    Box<dyn Fn(Document) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> + Send + Sync>;

/// Type alias for the reconnection future to reduce complexity
type ReconnectFuture = Pin<
    Box<
        dyn Future<
                Output = Result<
                    mongodb::change_stream::ChangeStream<ChangeStreamEvent<Document>>,
                    StreamError,
                >,
            > + Send,
    >,
>;

/// State of the change stream listener.
enum StreamState {
    /// Stream is active and consuming events
    Active,
    /// Stream encountered an error and is reconnecting
    Reconnecting(ReconnectFuture),
    /// Stream is closed (terminal state)
    Closed,
}

/// A MongoDB change stream listener with automatic reconnection.
///
/// Implements the `Stream` trait, yielding `Result<ChangeEvent, StreamError>`.
///
/// # Lifecycle
///
/// 1. **Active**: Consuming events normally
/// 2. **Reconnecting**: Connection lost, attempting to reconnect with exponential backoff
/// 3. **Closed**: Terminal state after fatal error or explicit close
///
/// # Thread Safety
///
/// `ChangeStreamListener` is `Send` but not `Sync`. Each listener should be
/// owned by a single task. For multi-collection scenarios, spawn separate
/// tasks with separate listeners.
///
/// # Resume Token Semantics
///
/// Resume tokens are only updated when the user calls `.ack()` on an `AckableEvent`.
/// This ensures at-least-once delivery semantics: if the stream crashes before
/// the user acknowledges an event, reconnection will resume from before that event.
pub struct ChangeStreamListener {
    /// The MongoDB collection being watched
    collection: Collection<Document>,

    /// Configuration for the change stream
    config: ChangeStreamConfig,

    /// The underlying MongoDB change stream
    stream: Option<mongodb::change_stream::ChangeStream<ChangeStreamEvent<Document>>>,

    /// Callback for persisting resume tokens
    resume_token_callback: ResumeTokenCallback,

    /// Current state of the listener
    state: StreamState,

    /// Number of reconnection attempts made
    reconnect_attempts: u32,

    /// Last successfully processed resume token (shared with AckableEvent)
    last_resume_token: Arc<Mutex<Option<Document>>>,

    /// Channel sender for acking tokens (sends to persistence task)
    ack_sender: mpsc::UnboundedSender<Document>,

    /// Background task that persists tokens
    _persistence_task: tokio::task::JoinHandle<()>,
}

impl ChangeStreamListener {
    /// Creates a new change stream listener.
    ///
    /// # Arguments
    ///
    /// * `collection` - MongoDB collection to watch
    /// * `config` - Change stream configuration
    /// * `resume_token_callback` - Callback to persist resume tokens
    ///
    /// # Errors
    ///
    /// Returns `StreamError::Connection` if initial connection fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rigatoni_core::stream::{ChangeStreamListener, ChangeStreamConfig};
    /// use mongodb::{Client, options::ClientOptions};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::with_options(
    ///     ClientOptions::parse("mongodb://localhost:27017").await?
    /// )?;
    /// let collection = client.database("mydb").collection("users");
    ///
    /// let listener = ChangeStreamListener::new(
    ///     collection,
    ///     ChangeStreamConfig::default(),
    ///     |token| Box::pin(async move {
    ///         println!("Token: {:?}", token);
    ///         Ok(())
    ///     }),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new<F>(
        collection: Collection<Document>,
        config: ChangeStreamConfig,
        resume_token_callback: F,
    ) -> Result<Self, StreamError>
    where
        F: Fn(Document) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        info!(
            "Initializing change stream for {}.{}",
            collection.namespace().db,
            collection.namespace().coll
        );

        let options = config.to_mongo_options();
        let stream = if config.pipeline.is_empty() {
            collection.watch().with_options(options).await?
        } else {
            collection
                .watch()
                .pipeline(config.pipeline.clone())
                .with_options(options)
                .await?
        };

        // Create mpsc channel for token persistence
        let (ack_sender, mut ack_receiver) = mpsc::unbounded_channel::<Document>();

        // Spawn background task to persist tokens
        let resume_token_callback_clone = Box::new(resume_token_callback);
        let persistence_task = tokio::spawn(async move {
            while let Some(token) = ack_receiver.recv().await {
                if let Err(e) = resume_token_callback_clone(token).await {
                    warn!("Failed to persist resume token: {}", e);
                }
            }
            debug!("Token persistence task shutting down");
        });

        Ok(Self {
            collection,
            config,
            stream: Some(stream),
            resume_token_callback: Box::new(|_| Box::pin(async { Ok(()) })),
            state: StreamState::Active,
            reconnect_attempts: 0,
            last_resume_token: Arc::new(Mutex::new(None)),
            ack_sender,
            _persistence_task: persistence_task,
        })
    }

    /// Async reconnection function that doesn't borrow self.
    ///
    /// This is called from within the reconnection future in poll_next.
    /// It returns the new stream on success.
    async fn reconnect_async(
        collection: Collection<Document>,
        config: ChangeStreamConfig,
        last_resume_token: Option<Document>,
        mut reconnect_attempts: u32,
    ) -> Result<mongodb::change_stream::ChangeStream<ChangeStreamEvent<Document>>, StreamError>
    {
        reconnect_attempts += 1;

        // Check if max attempts exceeded
        if config.max_reconnect_attempts > 0 && reconnect_attempts > config.max_reconnect_attempts {
            error!(
                attempts = reconnect_attempts,
                "Max reconnection attempts exceeded"
            );
            return Err(StreamError::MaxReconnectAttemptsExceeded(
                config.max_reconnect_attempts,
            ));
        }

        // Calculate exponential backoff with jitter
        let backoff = config.calculate_backoff(reconnect_attempts);

        warn!(
            attempt = reconnect_attempts,
            backoff_ms = backoff.as_millis(),
            "Reconnecting to change stream"
        );

        // Sleep before retry
        tokio::time::sleep(backoff).await;

        // Build options with resume token if available
        let mut options = config.to_mongo_options();
        if let Some(ref token_doc) = last_resume_token {
            debug!("Resuming from token: {:?}", token_doc);
            // Serialize Document to bytes, then deserialize as ResumeToken
            if let Ok(bytes) = bson::to_vec(token_doc) {
                if let Ok(resume_token) = bson::from_slice::<ResumeToken>(&bytes) {
                    options.resume_after = Some(resume_token);
                }
            }
        }

        // Attempt to reopen stream
        let stream = if config.pipeline.is_empty() {
            collection.watch().with_options(options).await?
        } else {
            collection
                .watch()
                .pipeline(config.pipeline.clone())
                .with_options(options)
                .await?
        };

        info!(
            attempt = reconnect_attempts,
            "Successfully reconnected to change stream"
        );

        Ok(stream)
    }

    /// Attempts to reconnect the change stream with exponential backoff.
    ///
    /// This method is called internally when a retryable error occurs.
    /// It will:
    /// 1. Calculate backoff duration
    /// 2. Sleep for backoff period
    /// 3. Attempt to reopen stream with last resume token
    /// 4. Reset attempts on success
    ///
    /// # Note
    ///
    /// This method is now deprecated in favor of reconnect_async.
    #[allow(dead_code)]
    async fn reconnect(&mut self) -> Result<(), StreamError> {
        self.reconnect_attempts += 1;

        // Check if max attempts exceeded
        if self.config.max_reconnect_attempts > 0
            && self.reconnect_attempts > self.config.max_reconnect_attempts
        {
            error!(
                attempts = self.reconnect_attempts,
                "Max reconnection attempts exceeded"
            );
            return Err(StreamError::MaxReconnectAttemptsExceeded(
                self.config.max_reconnect_attempts,
            ));
        }

        // Calculate exponential backoff: initial * 2^(attempts-1)
        let backoff_ms = self
            .config
            .initial_backoff_ms
            .saturating_mul(2_u64.saturating_pow(self.reconnect_attempts - 1))
            .min(self.config.max_backoff_ms);

        warn!(
            attempt = self.reconnect_attempts,
            backoff_ms = backoff_ms,
            "Reconnecting to change stream"
        );

        // Sleep before retry
        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;

        // Build options with resume token if available
        let mut options = self.config.to_mongo_options();
        if let Ok(last_token) = self.last_resume_token.lock() {
            if let Some(ref token_doc) = *last_token {
                debug!("Resuming from token: {:?}", token_doc);
                // Serialize Document to bytes, then deserialize as ResumeToken
                if let Ok(bytes) = bson::to_vec(token_doc) {
                    if let Ok(resume_token) = bson::from_slice::<ResumeToken>(&bytes) {
                        options.resume_after = Some(resume_token);
                    }
                }
            }
        }

        // Attempt to reopen stream
        let stream = if self.config.pipeline.is_empty() {
            self.collection.watch().with_options(options).await?
        } else {
            self.collection
                .watch()
                .pipeline(self.config.pipeline.clone())
                .with_options(options)
                .await?
        };

        info!(
            attempt = self.reconnect_attempts,
            "Successfully reconnected to change stream"
        );

        self.stream = Some(stream);
        self.state = StreamState::Active;
        self.reconnect_attempts = 0; // Reset on success

        Ok(())
    }

    /// Processes a MongoDB change stream event.
    ///
    /// Converts to ChangeEvent, persists resume token, and handles invalidation.
    ///
    /// # Note
    ///
    /// Currently not used in poll_next due to async/await borrowing constraints.
    /// The logic is inlined in poll_next instead.
    #[allow(dead_code)]
    async fn process_event(
        &mut self,
        event: ChangeStreamEvent<Document>,
    ) -> Result<ChangeEvent, StreamError> {
        // Extract resume token before conversion
        let resume_token = bson::to_document(&event.id)
            .map_err(|e| StreamError::ResumeTokenPersistence(e.to_string()))?;

        // Convert to ChangeEvent
        let change_event = ChangeEvent::try_from(event)?;

        // Check for invalidation
        if change_event.is_invalidate() {
            let reason = format!(
                "Collection {}.{} was dropped or renamed",
                self.collection.namespace().db,
                self.collection.namespace().coll
            );
            error!("{}", reason);
            self.state = StreamState::Closed;
            return Err(StreamError::Invalidated { reason });
        }

        // Persist resume token
        (self.resume_token_callback)(resume_token.clone())
            .await
            .map_err(StreamError::ResumeTokenPersistence)?;

        // Update last resume token
        if let Ok(mut last_token) = self.last_resume_token.lock() {
            *last_token = Some(resume_token);
        }

        Ok(change_event)
    }

    /// Closes the change stream.
    ///
    /// This is a graceful shutdown that closes the underlying MongoDB cursor.
    /// After calling this, the stream will return `None` on next poll.
    pub async fn close(&mut self) {
        info!("Closing change stream");
        self.state = StreamState::Closed;
        self.stream = None;
    }
}

impl Stream for ChangeStreamListener {
    type Item = Result<AckableEvent, StreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        // If closed, return None
        if matches!(this.state, StreamState::Closed) {
            return Poll::Ready(None);
        }

        // If reconnecting, poll the reconnection future
        if let StreamState::Reconnecting(ref mut reconnect_fut) = this.state {
            match reconnect_fut.as_mut().poll(cx) {
                Poll::Ready(Ok(new_stream)) => {
                    // Reconnection succeeded, transition back to Active
                    info!("Reconnection successful, resuming stream");
                    this.stream = Some(new_stream);
                    this.state = StreamState::Active;
                    this.reconnect_attempts = 0;
                    // Wake to poll the stream immediately
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                Poll::Ready(Err(e)) => {
                    // Reconnection failed permanently
                    error!("Reconnection failed: {}", e);
                    this.state = StreamState::Closed;
                    return Poll::Ready(Some(Err(e)));
                }
                Poll::Pending => {
                    // Still reconnecting
                    return Poll::Pending;
                }
            }
        }

        // Poll the underlying stream
        if let Some(ref mut stream) = this.stream {
            use futures::StreamExt;
            let mut stream_pin = Pin::new(stream);
            match stream_pin.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(event))) => {
                    // Extract resume token
                    let resume_token = match bson::to_document(&event.id) {
                        Ok(token) => token,
                        Err(e) => {
                            return Poll::Ready(Some(Err(StreamError::ResumeTokenPersistence(
                                e.to_string(),
                            ))))
                        }
                    };

                    // Convert to ChangeEvent
                    let change_event = match ChangeEvent::try_from(event) {
                        Ok(evt) => evt,
                        Err(e) => return Poll::Ready(Some(Err(StreamError::Conversion(e)))),
                    };

                    // Check for invalidation
                    if change_event.is_invalidate() {
                        let reason = format!(
                            "Collection {}.{} was dropped or renamed",
                            this.collection.namespace().db,
                            this.collection.namespace().coll
                        );
                        error!("{}", reason);
                        this.state = StreamState::Closed;
                        return Poll::Ready(Some(Err(StreamError::Invalidated { reason })));
                    }

                    // Create AckableEvent - user must call ack() after processing
                    // Note: last_resume_token is NOT updated here - only when user calls ack()
                    // This ensures at-least-once semantics
                    let ackable = AckableEvent {
                        event: change_event,
                        resume_token,
                        ack_sender: this.ack_sender.clone(),
                        last_resume_token: this.last_resume_token.clone(),
                    };

                    Poll::Ready(Some(Ok(ackable)))
                }
                Poll::Ready(Some(Err(e))) => {
                    // MongoDB error
                    let stream_err = StreamError::from_mongo_error(e);

                    if stream_err.is_retryable() {
                        warn!("Retryable error occurred: {}", stream_err);

                        // Create reconnection future
                        let collection = this.collection.clone();
                        let config = this.config.clone();
                        let last_resume_token =
                            this.last_resume_token.lock().ok().and_then(|t| t.clone());
                        let reconnect_attempts = this.reconnect_attempts;

                        let reconnect_fut = Box::pin(async move {
                            Self::reconnect_async(
                                collection,
                                config,
                                last_resume_token,
                                reconnect_attempts,
                            )
                            .await
                        });

                        this.state = StreamState::Reconnecting(reconnect_fut);
                        // Wake to attempt reconnection on next poll
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    } else {
                        error!("Fatal error occurred: {}", stream_err);
                        this.state = StreamState::Closed;
                        Poll::Ready(Some(Err(stream_err)))
                    }
                }
                Poll::Ready(None) => {
                    // Stream ended (should not happen for change streams)
                    warn!("Change stream ended unexpectedly");
                    this.state = StreamState::Closed;
                    Poll::Ready(None)
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            // No stream available
            this.state = StreamState::Closed;
            Poll::Ready(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn test_default_config() {
        let config = ChangeStreamConfig::default();
        assert_eq!(config.pipeline.len(), 0);
        assert!(!config.full_document_on_update);
        assert!(!config.full_document_before_change);
        assert_eq!(config.initial_backoff_ms, 100);
        assert_eq!(config.max_backoff_ms, 30_000);
        assert_eq!(config.max_reconnect_attempts, 5);
    }

    #[test]
    fn test_config_builder() {
        let config = ChangeStreamConfig::builder()
            .full_document_update_lookup()
            .full_document_before_change()
            .max_reconnect_attempts(10)
            .initial_backoff_ms(200)
            .max_backoff_ms(60_000)
            .batch_size(100)
            .build()
            .unwrap();

        assert!(config.full_document_on_update);
        assert!(config.full_document_before_change);
        assert_eq!(config.max_reconnect_attempts, 10);
        assert_eq!(config.initial_backoff_ms, 200);
        assert_eq!(config.max_backoff_ms, 60_000);
        assert_eq!(config.batch_size, Some(100));
    }

    #[test]
    fn test_config_with_pipeline() {
        use bson::doc;

        let pipeline = vec![doc! { "$match": { "operationType": "insert" } }];

        let config = ChangeStreamConfig::builder()
            .pipeline(pipeline.clone())
            .build()
            .unwrap();

        assert_eq!(config.pipeline.len(), 1);
        assert_eq!(config.pipeline[0], pipeline[0]);
    }

    #[test]
    fn test_stream_error_is_retryable() {
        // Connection errors with retryable labels are retryable
        let err = StreamError::Connection {
            message: "connection refused".to_string(),
            source: None,
            code: Some(6), // HostUnreachable
            labels: vec![],
        };
        assert!(err.is_retryable());

        // Invalidated is not retryable
        let err = StreamError::Invalidated {
            reason: "collection dropped".to_string(),
        };
        assert!(!err.is_retryable());

        // Max attempts exceeded is not retryable
        let err = StreamError::MaxReconnectAttemptsExceeded(5);
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_stream_error_category() {
        let err = StreamError::Invalidated {
            reason: "test".to_string(),
        };
        assert_eq!(err.category(), "invalidated");

        let err = StreamError::MaxReconnectAttemptsExceeded(5);
        assert_eq!(err.category(), "max_retries");

        let err = StreamError::Configuration("test".to_string());
        assert_eq!(err.category(), "configuration");
    }

    #[tokio::test]
    async fn test_resume_token_callback() {
        use bson::doc;

        let saved_token = Arc::new(Mutex::new(None));
        let saved_token_clone = saved_token.clone();

        let callback = move |token: Document| {
            let saved = saved_token_clone.clone();
            Box::pin(async move {
                *saved.lock().await = Some(token);
                Ok(())
            }) as Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
        };

        let test_token = doc! { "_data": "test123" };
        callback(test_token.clone()).await.unwrap();

        let saved = saved_token.lock().await;
        assert_eq!(*saved, Some(test_token));
    }

    // Note: test_stream_state_transitions removed because StreamState::Reconnecting
    // now contains a Future which doesn't implement PartialEq

    #[test]
    fn test_config_resume_token() {
        use bson::doc;

        let token = doc! { "_data": "token123" };
        let config = ChangeStreamConfig::builder()
            .resume_token(token.clone())
            .build()
            .unwrap();

        assert_eq!(config.resume_token, Some(token));
    }

    // Note: Integration tests with real MongoDB would go in tests/integration_test.rs
    // and would use testcontainers for a real MongoDB instance.
}
