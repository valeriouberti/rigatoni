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

//! Destination Trait and Error Types
//!
//! This module defines the core [`Destination`] trait that all destination implementations
//! must satisfy. Destinations are the final stage in the ETL pipeline where processed
//! events are written to external systems like S3, `BigQuery`, Kafka, or databases.
//!
//! # Architecture
//!
//! The [`Destination`] trait provides a uniform interface for writing batches of events
//! and managing destination state. It's designed to be:
//!
//! - **Async-first**: All operations are asynchronous for high throughput
//! - **Batch-oriented**: Events are written in batches for efficiency
//! - **Thread-safe**: Implementations must be `Send + Sync` for concurrent access
//! - **Error-resilient**: Comprehensive error handling with retry capabilities
//! - **Extensible**: Optional methods allow future additions without breaking changes
//!
//! # Design Rationale
//!
//! ## Why Batching?
//!
//! Most external systems perform better with batch operations rather than individual writes.
//! The [`Destination::write_batch`] method allows implementations to:
//! - Amortize network overhead across multiple events
//! - Use bulk APIs for better throughput
//! - Implement efficient memory management
//!
//! ## Why Separate `flush()`?
//!
//! The [`Destination::flush`] method is separate from [`Destination::write_batch`] because:
//! - **Buffering control**: Destinations may buffer writes internally for efficiency
//! - **Durability guarantees**: `flush()` provides a synchronization point for durability
//! - **Transaction boundaries**: Allows grouping multiple batches into transactions
//! - **Graceful shutdown**: Ensures all buffered data is written before termination
//!
//! ## Thread Safety
//!
//! All implementations must be `Send + Sync + 'static` to support:
//! - Concurrent writes from multiple producers
//! - Safe sharing across async task boundaries
//! - Use in thread pools and work stealing schedulers
//!
//! # Examples
//!
//! ## Implementing a Custom Destination
//!
//! ```rust
//! use rigatoni_core::destination::{Destination, DestinationError};
//! use rigatoni_core::event::ChangeEvent;
//! use async_trait::async_trait;
//!
//! /// A simple file-based destination
//! pub struct FileDestination {
//!     path: String,
//!     buffer: Vec<String>,
//! }
//!
//! #[async_trait]
//! impl Destination for FileDestination {
//!     async fn write_batch(&mut self, events: Vec<ChangeEvent>) -> Result<(), DestinationError> {
//!         // Buffer events as JSON lines
//!         for event in events {
//!             let json = serde_json::to_string(&event)
//!                 .map_err(|e| DestinationError::serialization(e, "Failed to serialize event"))?;
//!             self.buffer.push(json);
//!         }
//!         Ok(())
//!     }
//!
//!     async fn flush(&mut self) -> Result<(), DestinationError> {
//!         // Write all buffered events to file
//!         if !self.buffer.is_empty() {
//!             // Implementation would write to file here
//!             self.buffer.clear();
//!         }
//!         Ok(())
//!     }
//!
//!     fn supports_transactions(&self) -> bool {
//!         false // File writes don't support transactions
//!     }
//!
//!     async fn close(&mut self) -> Result<(), DestinationError> {
//!         self.flush().await
//!     }
//! }
//! ```
//!
//! ## Using a Destination
//!
//! ```rust,no_run
//! # use rigatoni_core::destination::{Destination, DestinationError};
//! # use rigatoni_core::event::ChangeEvent;
//! # async fn example(mut dest: impl Destination, events: Vec<ChangeEvent>) -> Result<(), DestinationError> {
//! // Write a batch of events
//! dest.write_batch(events).await?;
//!
//! // Flush to ensure durability
//! dest.flush().await?;
//!
//! // Clean shutdown
//! dest.close().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Error Handling
//!
//! The [`DestinationError`] type provides detailed error classification:
//! - [`DestinationError::ConnectionError`]: Network/connection failures (retryable)
//! - [`DestinationError::SerializationError`]: Data serialization failures (non-retryable)
//! - [`DestinationError::WriteError`]: Write operation failures (may be retryable)
//! - [`DestinationError::ConfigurationError`]: Invalid configuration (non-retryable)
//! - [`DestinationError::CapacityError`]: Resource limits exceeded (backpressure signal)
//!
//! Each error variant includes context about retryability to help callers implement
//! appropriate retry strategies.
//!
//! # Performance Considerations
//!
//! ## Trait Objects vs Generics
//!
//! The trait can be used in two ways:
//!
//! ```rust
//! # use rigatoni_core::destination::Destination;
//! // Static dispatch (zero-cost abstraction, monomorphization)
//! async fn process_with_static<D: Destination>(dest: D) {
//!     // Compiler generates specialized code for each destination type
//! }
//!
//! // Dynamic dispatch (runtime flexibility, single compiled function)
//! async fn process_with_dynamic(dest: Box<dyn Destination>) {
//!     // Single compiled function, virtual dispatch overhead
//! }
//! ```
//!
//! **Trade-offs**:
//! - Static dispatch: Faster (no vtable), larger binary (code duplication), compile-time type
//! - Dynamic dispatch: Smaller binary, allows runtime destination selection, slight overhead
//!
//! For most ETL workloads, the difference is negligible compared to I/O costs.
//!
//! # Future Evolution
//!
//! The trait is designed for non-breaking evolution:
//! - New optional methods can be added with default implementations
//! - The [`Destination::metadata`] method provides capability discovery
//! - Version negotiation can be added through metadata
//!
//! Planned additions:
//! - `async fn begin_transaction(&mut self) -> Result<TransactionId, DestinationError>`
//! - `async fn commit_transaction(&mut self, id: TransactionId) -> Result<(), DestinationError>`
//! - `async fn health_check(&self) -> Result<HealthStatus, DestinationError>`

use crate::event::ChangeEvent;
use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur when writing to a destination.
///
/// Each error variant includes information about whether the operation is retryable
/// and provides context for error handling and observability.
#[derive(Error, Debug)]
pub enum DestinationError {
    /// Connection to the destination failed.
    ///
    /// This is typically retryable after a backoff period. Examples include:
    /// - Network timeouts
    /// - DNS resolution failures
    /// - Connection refused
    /// - TLS handshake failures
    #[error("Connection error: {message}")]
    ConnectionError {
        /// Human-readable error message
        message: String,
        /// The underlying connection error
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Failed to serialize events for writing.
    ///
    /// This is typically non-retryable as it indicates a data quality issue.
    /// Examples include:
    /// - Invalid UTF-8 sequences
    /// - Schema validation failures
    /// - Unsupported data types
    #[error("Serialization error: {message}")]
    SerializationError {
        /// Human-readable error message
        message: String,
        /// The underlying serialization error
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Failed to write events to the destination.
    ///
    /// Retryability depends on the specific cause. Examples include:
    /// - Quota exceeded (retryable after backoff)
    /// - Permission denied (non-retryable)
    /// - Destination unavailable (retryable)
    /// - Invalid data format (non-retryable)
    #[error("Write error: {message}")]
    WriteError {
        /// Human-readable error message
        message: String,
        /// Whether this specific write error is retryable
        retryable: bool,
        /// The underlying error
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Invalid destination configuration.
    ///
    /// This is non-retryable and indicates a programming or configuration error.
    /// Examples include:
    /// - Missing required configuration parameters
    /// - Invalid connection strings
    /// - Conflicting options
    #[error("Configuration error: {message}")]
    ConfigurationError {
        /// Human-readable error message
        message: String,
        /// Configuration parameter name if applicable
        parameter: Option<String>,
    },

    /// Destination capacity exceeded.
    ///
    /// This signals backpressure and should pause writes. Examples include:
    /// - Write buffer full
    /// - Rate limit exceeded
    /// - Disk space exhausted
    /// - Memory limit reached
    #[error("Capacity error: {message}")]
    CapacityError {
        /// Human-readable error message
        message: String,
        /// Current capacity utilization (0.0 - 1.0)
        utilization: Option<f64>,
        /// Suggested wait time before retry
        retry_after: Option<std::time::Duration>,
    },

    /// A generic error occurred.
    ///
    /// Used for errors that don't fit other categories or for wrapping
    /// destination-specific errors.
    #[error("Destination error: {message}")]
    Other {
        /// Human-readable error message
        message: String,
        /// Whether this error is retryable
        retryable: bool,
        /// The underlying error
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl DestinationError {
    /// Creates a connection error from any error type.
    #[must_use]
    pub fn connection(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::ConnectionError {
            message: source.to_string(),
            source: Some(Box::new(source)),
        }
    }

    /// Creates a connection error with a custom message.
    #[must_use]
    pub fn connection_msg(message: impl Into<String>) -> Self {
        Self::ConnectionError {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a serialization error from any error type.
    #[must_use]
    pub fn serialization(
        source: impl std::error::Error + Send + Sync + 'static,
        message: impl Into<String>,
    ) -> Self {
        Self::SerializationError {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Creates a write error with retryability information.
    #[must_use]
    pub fn write(source: impl std::error::Error + Send + Sync + 'static, retryable: bool) -> Self {
        Self::WriteError {
            message: source.to_string(),
            retryable,
            source: Some(Box::new(source)),
        }
    }

    /// Creates a write error with a custom message.
    #[must_use]
    pub fn write_msg(message: impl Into<String>, retryable: bool) -> Self {
        Self::WriteError {
            message: message.into(),
            retryable,
            source: None,
        }
    }

    /// Creates a configuration error.
    #[must_use]
    pub fn configuration(message: impl Into<String>, parameter: Option<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
            parameter,
        }
    }

    /// Creates a capacity error with optional metadata.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `utilization` is provided and not in range [0.0, 1.0].
    #[must_use]
    pub fn capacity(
        message: impl Into<String>,
        utilization: Option<f64>,
        retry_after: Option<std::time::Duration>,
    ) -> Self {
        // Validate utilization range in debug builds
        if let Some(u) = utilization {
            debug_assert!(
                (0.0..=1.0).contains(&u),
                "utilization must be in range [0.0, 1.0], got {u}"
            );
        }

        Self::CapacityError {
            message: message.into(),
            utilization,
            retry_after,
        }
    }

    /// Creates a generic error.
    #[must_use]
    pub fn other(source: impl std::error::Error + Send + Sync + 'static, retryable: bool) -> Self {
        Self::Other {
            message: source.to_string(),
            retryable,
            source: Some(Box::new(source)),
        }
    }

    /// Returns whether this error is retryable.
    ///
    /// This helps callers implement appropriate retry strategies:
    /// - Retryable errors should be retried with exponential backoff
    /// - Non-retryable errors should fail fast or route to dead letter queue
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::ConnectionError { .. } | Self::CapacityError { .. } => true,
            Self::SerializationError { .. } | Self::ConfigurationError { .. } => false,
            Self::WriteError { retryable, .. } | Self::Other { retryable, .. } => *retryable,
        }
    }

    /// Returns suggested wait time before retry, if applicable.
    #[must_use]
    pub const fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            Self::CapacityError { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

/// Metadata about a destination's capabilities.
///
/// This allows runtime introspection of destination features and is used for:
/// - Feature detection and capability negotiation
/// - Monitoring and observability
/// - Dynamic pipeline configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationMetadata {
    /// Human-readable destination name (e.g., "S3", "`BigQuery`", "Kafka")
    pub name: String,

    /// Destination type identifier (e.g., "s3", "bigquery", "kafka")
    pub destination_type: String,

    /// Whether the destination supports transactions
    pub supports_transactions: bool,

    /// Maximum batch size hint (None = no limit)
    pub max_batch_size: Option<usize>,

    /// Whether the destination supports concurrent writes
    pub supports_concurrent_writes: bool,

    /// Additional destination-specific metadata
    pub properties: HashMap<String, String>,
}

impl DestinationMetadata {
    /// Creates new metadata with required fields.
    #[must_use]
    pub fn new(name: impl Into<String>, destination_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            destination_type: destination_type.into(),
            supports_transactions: false,
            max_batch_size: None,
            supports_concurrent_writes: true,
            properties: HashMap::new(),
        }
    }

    /// Sets transaction support.
    #[must_use]
    pub const fn with_transactions(mut self, supports: bool) -> Self {
        self.supports_transactions = supports;
        self
    }

    /// Sets maximum batch size.
    #[must_use]
    pub const fn with_max_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = Some(size);
        self
    }

    /// Sets concurrent write support.
    #[must_use]
    pub const fn with_concurrent_writes(mut self, supports: bool) -> Self {
        self.supports_concurrent_writes = supports;
        self
    }

    /// Adds a custom property.
    #[must_use]
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }
}

/// The core destination trait for writing events to external systems.
///
/// All destination implementations (S3, `BigQuery`, Kafka, databases, etc.) must implement
/// this trait. It provides a uniform interface for the Rigatoni pipeline to write events
/// regardless of the underlying destination system.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync + 'static` to support:
/// - Concurrent access from multiple async tasks
/// - Safe transfer across thread boundaries
/// - Use in work-stealing schedulers
///
/// # Async Considerations
///
/// All methods are async to support:
/// - Non-blocking I/O operations
/// - Efficient resource utilization
/// - High concurrency without thread-per-connection
///
/// The `#[async_trait]` macro is used because native async trait methods have limitations
/// in Rust (as of 2024). This macro transforms async trait methods into methods that
/// return `Pin<Box<dyn Future + Send>>`, enabling dynamic dispatch while maintaining
/// ergonomic async/await syntax.
///
/// # Error Handling
///
/// All fallible operations return [`DestinationError`] which provides:
/// - Detailed error classification
/// - Retryability information
/// - Error context for observability
/// - Source error chaining
///
/// Implementations should provide specific error information to help callers make
/// informed retry decisions.
///
/// # Implementation Guide
///
/// When implementing this trait:
///
/// 1. **`write_batch`**: Should handle batch writes efficiently
///    - Consider internal buffering for performance
///    - Validate events before writing
///    - Provide detailed errors for each failure mode
///    - Handle partial failures appropriately
///
/// 2. **`flush`**: Must ensure all buffered data is persisted
///    - Block until writes complete
///    - Return errors if persistence fails
///    - Safe to call multiple times (idempotent)
///
/// 3. **`supports_transactions`**: Indicate if ACID transactions are supported
///    - Return `true` only if full ACID guarantees available
///    - Consider returning `false` for "best effort" systems
///
/// 4. **`close`**: Clean up resources and ensure data durability
///    - Call `flush()` internally
///    - Close connections/files
///    - Release resources
///    - Safe to call multiple times (idempotent)
///
/// 5. **`metadata`**: Provide accurate capability information
///    - Helps pipeline make optimization decisions
///    - Enables dynamic configuration
///    - Supports observability
///
/// # Examples
///
/// See module-level documentation for complete implementation examples.
#[async_trait]
pub trait Destination: Send + Sync {
    /// Writes a batch of events to the destination.
    ///
    /// This is the primary method for writing data. Implementations should:
    /// - Process events efficiently (consider batching to destination API)
    /// - Maintain event ordering within the batch when possible
    /// - Handle partial failures gracefully
    /// - Provide detailed error information
    ///
    /// # Buffering
    ///
    /// Implementations may buffer events internally for performance. If buffering is used:
    /// - Document the buffering behavior
    /// - Implement `flush()` to persist buffered data
    /// - Consider memory limits
    ///
    /// # Error Handling
    ///
    /// If this method returns an error:
    /// - The batch should be considered failed
    /// - Callers should check `DestinationError::is_retryable()`
    /// - Partial writes should be rolled back if possible
    ///
    /// # Arguments
    ///
    /// * `events` - The batch of events to write. Empty batches should succeed without error.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the batch was written successfully (or buffered for later write)
    /// - `Err(DestinationError)` if the write failed
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use rigatoni_core::destination::{Destination, DestinationError};
    /// # use rigatoni_core::event::ChangeEvent;
    /// # async fn example(mut dest: impl Destination, events: Vec<ChangeEvent>) -> Result<(), DestinationError> {
    /// // Write a batch
    /// dest.write_batch(events).await?;
    ///
    /// // Empty batches are allowed
    /// dest.write_batch(vec![]).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn write_batch(&mut self, events: Vec<ChangeEvent>) -> Result<(), DestinationError>;

    /// Flushes any buffered data to ensure durability.
    ///
    /// This method ensures that all previously written data is persisted to the destination.
    /// After `flush()` returns successfully, all prior `write_batch()` calls should be durable.
    ///
    /// # Ordering Guarantees
    ///
    /// - **Write ordering**: Events within a batch maintain their order
    /// - **Batch ordering**: Batches are persisted in the order `write_batch()` was called
    /// - **Happens-before**: A successful `flush()` establishes a happens-before relationship
    ///   with all prior writes, guaranteeing visibility to subsequent reads
    ///
    /// # When to Call
    ///
    /// Call `flush()`:
    /// - Before shutting down the application
    /// - At transaction boundaries
    /// - After critical batches that require durability
    /// - Periodically to limit data loss window
    ///
    /// # Idempotency
    ///
    /// This method should be idempotent - calling it multiple times should be safe.
    /// Subsequent calls with no intervening writes should be no-ops.
    ///
    /// # Blocking
    ///
    /// This method should block until all data is persisted. Don't return early with
    /// "flush initiated" - wait for confirmation.
    ///
    /// # Error Handling
    ///
    /// If this method returns an error, some buffered data may be lost. Callers should:
    /// - Consider the pipeline state corrupted
    /// - Attempt recovery or fail fast
    /// - Log the error with full context
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use rigatoni_core::destination::{Destination, DestinationError};
    /// # use rigatoni_core::event::ChangeEvent;
    /// # async fn example(mut dest: impl Destination, batches: Vec<Vec<ChangeEvent>>) -> Result<(), DestinationError> {
    /// // Write multiple batches
    /// for batch in batches {
    ///     dest.write_batch(batch).await?;
    /// }
    ///
    /// // Ensure everything is persisted
    /// dest.flush().await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn flush(&mut self) -> Result<(), DestinationError>;

    /// Returns whether this destination supports ACID transactions.
    ///
    /// Return `true` only if the destination provides full ACID guarantees:
    /// - **Atomicity**: All or nothing for transaction operations
    /// - **Consistency**: Transactions maintain invariants
    /// - **Isolation**: Concurrent transactions don't interfere
    /// - **Durability**: Committed data survives failures
    ///
    /// # Use Cases
    ///
    /// Transaction support enables:
    /// - Exactly-once semantics across batches
    /// - Coordinated multi-destination writes
    /// - Reliable state management
    ///
    /// # Implementation Note
    ///
    /// This is a synchronous method (not async) because it should return a compile-time
    /// constant. The result should not depend on runtime state.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rigatoni_core::destination::{Destination, DestinationError};
    /// # async fn example(dest: impl Destination) {
    /// if dest.supports_transactions() {
    ///     println!("Destination supports ACID transactions");
    /// } else {
    ///     println!("Destination provides at-least-once semantics");
    /// }
    /// # }
    /// ```
    fn supports_transactions(&self) -> bool;

    /// Closes the destination and releases resources.
    ///
    /// This method should:
    /// 1. Call `flush()` to persist any buffered data
    /// 2. Close network connections
    /// 3. Release file handles
    /// 4. Clean up temporary resources
    ///
    /// # Idempotency
    ///
    /// This method should be idempotent - safe to call multiple times.
    /// Subsequent calls should be no-ops. Implementers SHOULD ensure multiple calls
    /// are safe even after resource teardown (e.g., by tracking closed state).
    ///
    /// # Drop vs Close
    ///
    /// While Rust's `Drop` trait handles cleanup, `close()` is async and can return errors.
    /// Always call `close()` explicitly for graceful shutdown. The `Drop` impl should:
    /// - Log a warning if `close()` wasn't called
    /// - Attempt best-effort cleanup
    /// - Not panic
    ///
    /// # Default Implementation
    ///
    /// The default implementation just calls `flush()`. Override this method if you need
    /// additional cleanup logic (connection closure, file handles, etc.).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use rigatoni_core::destination::{Destination, DestinationError};
    /// # async fn example(mut dest: impl Destination) -> Result<(), DestinationError> {
    /// // Use the destination
    /// // ...
    ///
    /// // Clean shutdown
    /// dest.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn close(&mut self) -> Result<(), DestinationError> {
        self.flush().await
    }

    /// Returns metadata about this destination's capabilities.
    ///
    /// This enables runtime introspection and dynamic pipeline configuration.
    ///
    /// # Default Implementation
    ///
    /// The default implementation returns basic metadata. Override to provide
    /// destination-specific information.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rigatoni_core::destination::{Destination, DestinationError};
    /// # async fn example(dest: impl Destination) {
    /// let meta = dest.metadata();
    /// println!("Destination: {}", meta.name);
    /// println!("Supports transactions: {}", meta.supports_transactions);
    ///
    /// if let Some(max_batch) = meta.max_batch_size {
    ///     println!("Max batch size: {}", max_batch);
    /// }
    /// # }
    /// ```
    fn metadata(&self) -> DestinationMetadata {
        DestinationMetadata::new("Unknown", "unknown")
            .with_transactions(self.supports_transactions())
    }
}

/// A mock destination implementation for testing.
///
/// This implementation stores events in memory and provides inspection methods for testing.
/// It supports simulating various failure modes through configuration.
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::destination::{Destination, MockDestination};
/// use rigatoni_core::event::{ChangeEvent, OperationType, Namespace};
/// use bson::doc;
/// use chrono::Utc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut dest = MockDestination::new();
///
/// // Write some events
/// let events = vec![
///     ChangeEvent {
///         operation: OperationType::Insert,
///         namespace: Namespace::new("db", "collection"),
///         document_key: Some(doc! { "_id": 1 }),
///         full_document: Some(doc! { "name": "test" }),
///         update_description: None,
///         cluster_time: Utc::now(),
///         resume_token: doc! { "_data": "token" },
///     }
/// ];
///
/// dest.write_batch(events).await?;
/// dest.flush().await?;
///
/// // Verify
/// assert_eq!(dest.total_events_written(), 1);
/// assert_eq!(dest.flush_count(), 1);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct MockDestination {
    /// All events written to this destination
    events: Vec<ChangeEvent>,
    /// Number of times flush was called
    flush_count: usize,
    /// Number of times close was called
    close_count: usize,
    /// Whether to simulate failures
    fail_writes: bool,
    /// Whether to simulate capacity errors
    simulate_capacity_error: bool,
    /// Custom metadata
    metadata: Option<DestinationMetadata>,
}

impl MockDestination {
    /// Creates a new mock destination.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the mock to fail all write operations.
    #[must_use]
    pub const fn with_write_failures(mut self) -> Self {
        self.fail_writes = true;
        self
    }

    /// Configures the mock to simulate capacity errors.
    #[must_use]
    pub const fn with_capacity_errors(mut self) -> Self {
        self.simulate_capacity_error = true;
        self
    }

    /// Sets custom metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: DestinationMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Returns all events written to this destination.
    #[must_use]
    pub fn events(&self) -> &[ChangeEvent] {
        &self.events
    }

    /// Returns the total number of events written.
    #[must_use]
    pub fn total_events_written(&self) -> usize {
        self.events.len()
    }

    /// Returns the number of times flush was called.
    #[must_use]
    pub const fn flush_count(&self) -> usize {
        self.flush_count
    }

    /// Returns the number of times close was called.
    #[must_use]
    pub const fn close_count(&self) -> usize {
        self.close_count
    }

    /// Clears all stored events and resets counters.
    pub fn reset(&mut self) {
        self.events.clear();
        self.flush_count = 0;
        self.close_count = 0;
    }
}

#[async_trait]
impl Destination for MockDestination {
    async fn write_batch(&mut self, events: Vec<ChangeEvent>) -> Result<(), DestinationError> {
        if self.fail_writes {
            return Err(DestinationError::write_msg("Simulated write failure", true));
        }

        if self.simulate_capacity_error {
            return Err(DestinationError::capacity(
                "Simulated capacity error",
                Some(1.0),
                Some(std::time::Duration::from_secs(1)),
            ));
        }

        self.events.extend(events);
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), DestinationError> {
        self.flush_count += 1;
        Ok(())
    }

    fn supports_transactions(&self) -> bool {
        false
    }

    async fn close(&mut self) -> Result<(), DestinationError> {
        self.close_count += 1;
        self.flush().await
    }

    fn metadata(&self) -> DestinationMetadata {
        self.metadata.clone().unwrap_or_else(|| {
            DestinationMetadata::new("MockDestination", "mock")
                .with_transactions(false)
                .with_concurrent_writes(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Namespace, OperationType};
    use bson::doc;
    use chrono::Utc;

    fn create_test_event() -> ChangeEvent {
        ChangeEvent {
            operation: OperationType::Insert,
            namespace: Namespace::new("test_db", "test_collection"),
            document_key: Some(doc! { "_id": 1 }),
            full_document: Some(doc! { "name": "test", "value": 42 }),
            update_description: None,
            cluster_time: Utc::now(),
            resume_token: doc! { "_data": "test_token" },
        }
    }

    #[tokio::test]
    async fn test_mock_destination_write_batch() {
        let mut dest = MockDestination::new();

        let events = vec![create_test_event(), create_test_event()];
        dest.write_batch(events).await.unwrap();

        assert_eq!(dest.total_events_written(), 2);
        assert_eq!(dest.events().len(), 2);
    }

    #[tokio::test]
    async fn test_mock_destination_flush() {
        let mut dest = MockDestination::new();

        assert_eq!(dest.flush_count(), 0);

        dest.flush().await.unwrap();
        assert_eq!(dest.flush_count(), 1);

        dest.flush().await.unwrap();
        assert_eq!(dest.flush_count(), 2);
    }

    #[tokio::test]
    async fn test_mock_destination_close() {
        let mut dest = MockDestination::new();

        dest.close().await.unwrap();
        assert_eq!(dest.close_count(), 1);
        assert_eq!(dest.flush_count(), 1); // close calls flush
    }

    #[tokio::test]
    async fn test_mock_destination_empty_batch() {
        let mut dest = MockDestination::new();

        dest.write_batch(vec![]).await.unwrap();
        assert_eq!(dest.total_events_written(), 0);
    }

    #[tokio::test]
    async fn test_mock_destination_write_failures() {
        let mut dest = MockDestination::new().with_write_failures();

        let events = vec![create_test_event()];
        let result = dest.write_batch(events).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DestinationError::WriteError { .. }));
        assert!(err.is_retryable());
    }

    #[tokio::test]
    async fn test_mock_destination_capacity_errors() {
        let mut dest = MockDestination::new().with_capacity_errors();

        let events = vec![create_test_event()];
        let result = dest.write_batch(events).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DestinationError::CapacityError { .. }));
        assert!(err.is_retryable());
        assert_eq!(err.retry_after(), Some(std::time::Duration::from_secs(1)));
    }

    #[tokio::test]
    async fn test_mock_destination_reset() {
        let mut dest = MockDestination::new();

        dest.write_batch(vec![create_test_event()]).await.unwrap();
        dest.flush().await.unwrap();
        dest.close().await.unwrap();

        assert_eq!(dest.total_events_written(), 1);
        assert_eq!(dest.flush_count(), 2); // flush + close
        assert_eq!(dest.close_count(), 1);

        dest.reset();

        assert_eq!(dest.total_events_written(), 0);
        assert_eq!(dest.flush_count(), 0);
        assert_eq!(dest.close_count(), 0);
    }

    #[tokio::test]
    async fn test_destination_metadata() {
        let dest = MockDestination::new();
        let meta = dest.metadata();

        assert_eq!(meta.name, "MockDestination");
        assert_eq!(meta.destination_type, "mock");
        assert!(!meta.supports_transactions);
        assert!(meta.supports_concurrent_writes);
    }

    #[tokio::test]
    async fn test_custom_metadata() {
        let custom_meta = DestinationMetadata::new("CustomDest", "custom")
            .with_transactions(true)
            .with_max_batch_size(1000)
            .with_property("region", "us-west-2");

        let dest = MockDestination::new().with_metadata(custom_meta);
        let meta = dest.metadata();

        assert_eq!(meta.name, "CustomDest");
        assert_eq!(meta.destination_type, "custom");
        assert!(meta.supports_transactions);
        assert_eq!(meta.max_batch_size, Some(1000));
        assert_eq!(
            meta.properties.get("region"),
            Some(&"us-west-2".to_string())
        );
    }

    #[test]
    fn test_destination_error_retryable() {
        assert!(DestinationError::connection_msg("test").is_retryable());
        assert!(
            !DestinationError::serialization(std::io::Error::other("test"), "test").is_retryable()
        );
        assert!(DestinationError::write_msg("test", true).is_retryable());
        assert!(!DestinationError::write_msg("test", false).is_retryable());
        assert!(!DestinationError::configuration("test", None).is_retryable());
        assert!(DestinationError::capacity("test", None, None).is_retryable());
    }

    #[test]
    fn test_destination_error_retry_after() {
        let duration = std::time::Duration::from_secs(5);
        let err = DestinationError::capacity("test", Some(0.9), Some(duration));

        assert_eq!(err.retry_after(), Some(duration));
    }

    #[test]
    fn test_destination_metadata_builder() {
        let meta = DestinationMetadata::new("S3", "s3")
            .with_transactions(false)
            .with_max_batch_size(10_000)
            .with_concurrent_writes(true)
            .with_property("bucket", "my-bucket")
            .with_property("region", "us-east-1");

        assert_eq!(meta.name, "S3");
        assert_eq!(meta.destination_type, "s3");
        assert!(!meta.supports_transactions);
        assert_eq!(meta.max_batch_size, Some(10_000));
        assert!(meta.supports_concurrent_writes);
        assert_eq!(
            meta.properties.get("bucket"),
            Some(&"my-bucket".to_string())
        );
        assert_eq!(
            meta.properties.get("region"),
            Some(&"us-east-1".to_string())
        );
    }
}
