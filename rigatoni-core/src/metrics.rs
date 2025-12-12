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

//! Metrics instrumentation for Rigatoni pipeline observability.
//!
//! This module provides comprehensive metrics collection for monitoring
//! Rigatoni pipelines in production. It uses the `metrics` crate which
//! supports multiple exporters (Prometheus, StatsD, etc.).
//!
//! # Metric Types
//!
//! - **Counters**: Monotonically increasing values (events processed, errors)
//! - **Histograms**: Value distributions (batch sizes, latencies)
//! - **Gauges**: Point-in-time values (active collections, queue depth)
//!
//! # Naming Conventions
//!
//! All metrics follow Prometheus naming conventions:
//! - Use underscores (not hyphens or camelCase)
//! - Include unit suffix (\_seconds, \_bytes, \_total)
//! - Prefix with application name (rigatoni\_)
//! - Counter metrics end with \_total
//!
//! # Labels
//!
//! Labels add dimensionality but increase cardinality. Use sparingly:
//! - **collection**: MongoDB collection name (low cardinality)
//! - **destination\_type**: Destination type like "s3", "bigquery" (very low cardinality)
//! - **error\_type**: Error category (low cardinality, max ~20 types)
//! - **operation**: Operation type like "insert", "update", "delete" (very low cardinality)
//!
//! ⚠️ **Cardinality Warning**: Never use high-cardinality values as labels:
//! - Document IDs
//! - Timestamps
//! - User IDs
//! - Full error messages
//!
//! # Performance Considerations
//!
//! The `metrics` crate is designed for low overhead:
//! - Lock-free atomic operations for counters
//! - Thread-local histograms with periodic aggregation
//! - No-op recorder when metrics are disabled
//! - Typical overhead: <1μs per metric call
//!
//! # Examples
//!
//! ## Recording Events
//!
//! ```rust
//! use rigatoni_core::metrics;
//!
//! // Increment counter
//! metrics::increment_events_processed("users", "insert");
//!
//! // Record histogram
//! metrics::record_batch_size(150, "orders");
//!
//! // Update gauge
//! metrics::set_active_collections(3);
//! ```
//!
//! ## Measuring Duration
//!
//! ```rust
//! use rigatoni_core::metrics;
//! use std::time::Instant;
//!
//! let start = Instant::now();
//! // ... do work ...
//! metrics::record_batch_duration(start.elapsed().as_secs_f64(), "products");
//! ```
//!
//! ## Error Tracking
//!
//! ```rust
//! use rigatoni_core::metrics::{self, ErrorCategory};
//!
//! metrics::increment_events_failed("users", ErrorCategory::Serialization);
//! metrics::increment_retries(ErrorCategory::Timeout);
//! ```

use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use std::time::Duration;

/// Metric name prefix for all Rigatoni metrics.
#[doc(hidden)]
pub const METRIC_PREFIX: &str = "rigatoni";

// ============================================================================
// Metric Name Constants
// ============================================================================

/// Total number of events successfully processed through the pipeline.
///
/// Type: Counter
/// Labels: collection, operation
#[doc(hidden)]
pub const EVENTS_PROCESSED_TOTAL: &str = "rigatoni_events_processed_total";

/// Total number of events that failed processing.
///
/// Type: Counter
/// Labels: collection, error_type
const EVENTS_FAILED_TOTAL: &str = "rigatoni_events_failed_total";

/// Total number of retry attempts across all operations.
///
/// Type: Counter
/// Labels: error_type
const RETRIES_TOTAL: &str = "rigatoni_retries_total";

/// Distribution of batch sizes sent to destinations.
///
/// Type: Histogram
/// Labels: collection
/// Unit: events
const BATCH_SIZE: &str = "rigatoni_batch_size";

/// Time taken to process a batch (from accumulation to write).
///
/// Type: Histogram
/// Labels: collection
/// Unit: seconds
#[doc(hidden)]
pub const BATCH_DURATION_SECONDS: &str = "rigatoni_batch_duration_seconds";

/// Time taken for destination write operations.
///
/// Type: Histogram
/// Labels: destination_type
/// Unit: seconds
const DESTINATION_WRITE_DURATION_SECONDS: &str = "rigatoni_destination_write_duration_seconds";

/// Number of collections currently being monitored.
///
/// Type: Gauge
/// Unit: count
const ACTIVE_COLLECTIONS: &str = "rigatoni_active_collections";

/// Current pipeline status (0=stopped, 1=running, 2=error).
///
/// Type: Gauge
/// Unit: status code
const PIPELINE_STATUS: &str = "rigatoni_pipeline_status";

/// Number of events currently buffered in batch queue.
///
/// Type: Gauge
/// Labels: collection
/// Unit: events
const BATCH_QUEUE_SIZE: &str = "rigatoni_batch_queue_size";

/// Total number of batches successfully written to destinations.
///
/// Type: Counter
/// Labels: destination_type
const BATCHES_WRITTEN_TOTAL: &str = "rigatoni_batches_written_total";

/// Total number of destination write errors.
///
/// Type: Counter
/// Labels: destination_type, error_type
const DESTINATION_WRITE_ERRORS_TOTAL: &str = "rigatoni_destination_write_errors_total";

/// Size of data written to destination (compressed if applicable).
///
/// Type: Histogram
/// Labels: destination_type
/// Unit: bytes
const DESTINATION_WRITE_BYTES: &str = "rigatoni_destination_write_bytes";

/// Time taken for change stream to receive an event.
///
/// Type: Histogram
/// Labels: collection
/// Unit: seconds
const CHANGE_STREAM_LAG_SECONDS: &str = "rigatoni_change_stream_lag_seconds";

// ============================================================================
// Distributed Locking Metrics
// ============================================================================

/// Total number of locks currently held by this instance.
///
/// Type: Gauge
/// Unit: count
const LOCKS_HELD_TOTAL: &str = "rigatoni_locks_held_total";

/// Total number of successful lock acquisitions.
///
/// Type: Counter
const LOCK_ACQUISITIONS_TOTAL: &str = "rigatoni_lock_acquisitions_total";

/// Total number of failed lock acquisition attempts.
///
/// Type: Counter
/// Labels: reason (already_held, error)
const LOCK_ACQUISITION_FAILURES_TOTAL: &str = "rigatoni_lock_acquisition_failures_total";

/// Total number of locks lost (expired or stolen).
///
/// Type: Counter
const LOCKS_LOST_TOTAL: &str = "rigatoni_locks_lost_total";

/// Total number of successful lock refreshes.
///
/// Type: Counter
const LOCK_REFRESHES_TOTAL: &str = "rigatoni_lock_refreshes_total";

/// Total number of locks released gracefully.
///
/// Type: Counter
const LOCKS_RELEASED_TOTAL: &str = "rigatoni_locks_released_total";

// ============================================================================
// Initialization
// ============================================================================

/// Initializes metric descriptions for documentation and introspection.
///
/// This should be called once at application startup, before recording any metrics.
/// It provides human-readable descriptions for metrics exporters like Prometheus.
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::init_metrics();
/// // ... start pipeline ...
/// ```
pub fn init_metrics() {
    // Counters
    describe_counter!(
        EVENTS_PROCESSED_TOTAL,
        "Total number of change stream events successfully processed through the pipeline"
    );

    describe_counter!(
        EVENTS_FAILED_TOTAL,
        "Total number of events that failed processing due to errors"
    );

    describe_counter!(
        RETRIES_TOTAL,
        "Total number of retry attempts for failed operations"
    );

    describe_counter!(
        BATCHES_WRITTEN_TOTAL,
        "Total number of batches successfully written to destinations"
    );

    describe_counter!(
        DESTINATION_WRITE_ERRORS_TOTAL,
        "Total number of errors writing to destinations"
    );

    // Histograms
    describe_histogram!(
        BATCH_SIZE,
        metrics::Unit::Count,
        "Distribution of batch sizes (number of events per batch)"
    );

    describe_histogram!(
        BATCH_DURATION_SECONDS,
        metrics::Unit::Seconds,
        "Time taken to process a batch from accumulation to successful write"
    );

    describe_histogram!(
        DESTINATION_WRITE_DURATION_SECONDS,
        metrics::Unit::Seconds,
        "Time taken for destination write operations (including retries)"
    );

    describe_histogram!(
        DESTINATION_WRITE_BYTES,
        metrics::Unit::Bytes,
        "Size of data written to destination (compressed if applicable)"
    );

    describe_histogram!(
        CHANGE_STREAM_LAG_SECONDS,
        metrics::Unit::Seconds,
        "Time between MongoDB operation and change stream receipt (approximation)"
    );

    // Gauges
    describe_gauge!(
        ACTIVE_COLLECTIONS,
        metrics::Unit::Count,
        "Number of MongoDB collections currently being monitored"
    );

    describe_gauge!(
        PIPELINE_STATUS,
        "Current pipeline status: 0=stopped, 1=running, 2=error"
    );

    describe_gauge!(
        BATCH_QUEUE_SIZE,
        metrics::Unit::Count,
        "Number of events currently buffered awaiting batch write"
    );

    // Distributed Locking Metrics
    describe_gauge!(
        LOCKS_HELD_TOTAL,
        metrics::Unit::Count,
        "Number of distributed locks currently held by this instance"
    );

    describe_counter!(
        LOCK_ACQUISITIONS_TOTAL,
        "Total number of successful distributed lock acquisitions"
    );

    describe_counter!(
        LOCK_ACQUISITION_FAILURES_TOTAL,
        "Total number of failed distributed lock acquisition attempts"
    );

    describe_counter!(
        LOCKS_LOST_TOTAL,
        "Total number of distributed locks lost (expired or stolen by another instance)"
    );

    describe_counter!(
        LOCK_REFRESHES_TOTAL,
        "Total number of successful distributed lock refreshes"
    );

    describe_counter!(
        LOCKS_RELEASED_TOTAL,
        "Total number of distributed locks released gracefully"
    );
}

// ============================================================================
// Counter Metrics
// ============================================================================

/// Increments the count of successfully processed events.
///
/// This should be called after a batch is successfully written to a destination
/// and the resume token is saved.
///
/// # Arguments
///
/// * `collection` - MongoDB collection name
/// * `operation` - Operation type: "insert", "update", "delete", "replace", etc.
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::increment_events_processed("users", "insert");
/// ```
pub fn increment_events_processed(collection: &str, operation: &str) {
    counter!(EVENTS_PROCESSED_TOTAL, "collection" => collection.to_string(), "operation" => operation.to_string())
        .increment(1);
}

/// Increments the count of successfully processed events by a specific amount.
///
/// Use this when processing a batch of events at once.
///
/// # Arguments
///
/// * `count` - Number of events processed
/// * `collection` - MongoDB collection name
/// * `operation` - Operation type
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// // After writing a batch of 150 events
/// metrics::increment_events_processed_by(150, "orders", "insert");
/// ```
pub fn increment_events_processed_by(count: u64, collection: &str, operation: &str) {
    counter!(EVENTS_PROCESSED_TOTAL, "collection" => collection.to_string(), "operation" => operation.to_string())
        .increment(count);
}

/// Increments the count of failed events.
///
/// This should be called when an event fails processing due to an error.
///
/// # Arguments
///
/// * `collection` - MongoDB collection name
/// * `error_category` - Category of error that occurred
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics::{self, ErrorCategory};
///
/// metrics::increment_events_failed("users", ErrorCategory::Serialization);
/// ```
pub fn increment_events_failed(collection: &str, error_category: ErrorCategory) {
    counter!(EVENTS_FAILED_TOTAL, "collection" => collection.to_string(), "error_type" => error_category.as_str())
        .increment(1);
}

/// Increments the retry counter.
///
/// This should be called each time an operation is retried.
///
/// # Arguments
///
/// * `error_category` - Category of error that triggered the retry
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics::{self, ErrorCategory};
///
/// metrics::increment_retries(ErrorCategory::Timeout);
/// ```
pub fn increment_retries(error_category: ErrorCategory) {
    counter!(RETRIES_TOTAL, "error_type" => error_category.as_str()).increment(1);
}

/// Increments the count of successfully written batches.
///
/// # Arguments
///
/// * `destination_type` - Type of destination (e.g., "s3", "bigquery", "kafka")
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::increment_batches_written("s3");
/// ```
pub fn increment_batches_written(destination_type: &str) {
    counter!(BATCHES_WRITTEN_TOTAL, "destination_type" => destination_type.to_string())
        .increment(1);
}

/// Increments the count of destination write errors.
///
/// # Arguments
///
/// * `destination_type` - Type of destination
/// * `error_type` - Error category
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics::{self, ErrorCategory};
///
/// metrics::increment_destination_errors("s3", ErrorCategory::Timeout);
/// ```
pub fn increment_destination_errors(destination_type: &str, error_category: ErrorCategory) {
    counter!(
        DESTINATION_WRITE_ERRORS_TOTAL,
        "destination_type" => destination_type.to_string(),
        "error_type" => error_category.as_str()
    )
    .increment(1);
}

// ============================================================================
// Histogram Metrics
// ============================================================================

/// Records a batch size.
///
/// # Arguments
///
/// * `size` - Number of events in the batch
/// * `collection` - MongoDB collection name
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::record_batch_size(150, "users");
/// ```
pub fn record_batch_size(size: usize, collection: &str) {
    histogram!(BATCH_SIZE, "collection" => collection.to_string()).record(size as f64);
}

/// Records the duration of batch processing.
///
/// This measures the time from when a batch starts accumulating events
/// to when it's successfully written to the destination.
///
/// # Arguments
///
/// * `duration_seconds` - Duration in seconds
/// * `collection` - MongoDB collection name
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
/// use std::time::Instant;
///
/// let start = Instant::now();
/// // ... process batch ...
/// metrics::record_batch_duration(start.elapsed().as_secs_f64(), "users");
/// ```
pub fn record_batch_duration(duration_seconds: f64, collection: &str) {
    histogram!(BATCH_DURATION_SECONDS, "collection" => collection.to_string())
        .record(duration_seconds);
}

/// Records the duration of a destination write operation.
///
/// This includes time spent in retries if applicable.
///
/// # Arguments
///
/// * `duration` - Duration of the write operation
/// * `destination_type` - Type of destination
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
/// use std::time::{Duration, Instant};
///
/// let start = Instant::now();
/// // ... write to destination ...
/// metrics::record_destination_write_duration(start.elapsed(), "s3");
/// ```
pub fn record_destination_write_duration(duration: Duration, destination_type: &str) {
    histogram!(DESTINATION_WRITE_DURATION_SECONDS, "destination_type" => destination_type.to_string())
        .record(duration.as_secs_f64());
}

/// Records the size of data written to a destination.
///
/// # Arguments
///
/// * `bytes` - Number of bytes written (compressed if applicable)
/// * `destination_type` - Type of destination
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::record_destination_write_bytes(1024 * 1024, "s3"); // 1 MB
/// ```
pub fn record_destination_write_bytes(bytes: usize, destination_type: &str) {
    histogram!(DESTINATION_WRITE_BYTES, "destination_type" => destination_type.to_string())
        .record(bytes as f64);
}

/// Records change stream lag (approximation).
///
/// This estimates the time between when a MongoDB operation occurred
/// and when the change stream event was received.
///
/// # Arguments
///
/// * `lag_seconds` - Estimated lag in seconds
/// * `collection` - MongoDB collection name
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::record_change_stream_lag(0.05, "users"); // 50ms lag
/// ```
pub fn record_change_stream_lag(lag_seconds: f64, collection: &str) {
    histogram!(CHANGE_STREAM_LAG_SECONDS, "collection" => collection.to_string())
        .record(lag_seconds);
}

// ============================================================================
// Gauge Metrics
// ============================================================================

/// Sets the number of active collections being monitored.
///
/// # Arguments
///
/// * `count` - Number of active collections
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::set_active_collections(3);
/// ```
pub fn set_active_collections(count: usize) {
    gauge!(ACTIVE_COLLECTIONS).set(count as f64);
}

/// Sets the pipeline status.
///
/// # Arguments
///
/// * `status` - Pipeline status: 0=stopped, 1=running, 2=error
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics::{self, PipelineStatus};
///
/// metrics::set_pipeline_status(PipelineStatus::Running);
/// ```
pub fn set_pipeline_status(status: PipelineStatus) {
    gauge!(PIPELINE_STATUS).set(f64::from(status as u8));
}

/// Sets the current batch queue size.
///
/// This represents how many events are currently buffered awaiting batch write.
///
/// # Arguments
///
/// * `size` - Number of events in queue
/// * `collection` - MongoDB collection name
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::set_batch_queue_size(47, "users");
/// ```
pub fn set_batch_queue_size(size: usize, collection: &str) {
    gauge!(BATCH_QUEUE_SIZE, "collection" => collection.to_string()).set(size as f64);
}

/// Increments the batch queue size by 1.
///
/// More efficient than `set_batch_queue_size` when adding a single event.
///
/// # Arguments
///
/// * `collection` - MongoDB collection name
pub fn increment_batch_queue_size(collection: &str) {
    gauge!(BATCH_QUEUE_SIZE, "collection" => collection.to_string()).increment(1.0);
}

/// Decrements the batch queue size by a specific amount.
///
/// Use this after flushing a batch.
///
/// # Arguments
///
/// * `count` - Number of events removed from queue
/// * `collection` - MongoDB collection name
pub fn decrement_batch_queue_size(count: usize, collection: &str) {
    gauge!(BATCH_QUEUE_SIZE, "collection" => collection.to_string()).decrement(count as f64);
}

// ============================================================================
// Distributed Locking Metrics
// ============================================================================

/// Sets the number of locks currently held by this instance.
///
/// # Arguments
///
/// * `count` - Number of locks held
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::set_locks_held(3);
/// ```
pub fn set_locks_held(count: usize) {
    gauge!(LOCKS_HELD_TOTAL).set(count as f64);
}

/// Increments the count of successful lock acquisitions.
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::increment_lock_acquisitions();
/// ```
pub fn increment_lock_acquisitions() {
    counter!(LOCK_ACQUISITIONS_TOTAL).increment(1);
}

/// Increments the count of failed lock acquisition attempts.
///
/// # Arguments
///
/// * `reason` - Why the lock acquisition failed
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics::{self, LockFailureReason};
///
/// metrics::increment_lock_acquisition_failures(LockFailureReason::AlreadyHeld);
/// ```
pub fn increment_lock_acquisition_failures(reason: LockFailureReason) {
    counter!(LOCK_ACQUISITION_FAILURES_TOTAL, "reason" => reason.as_str()).increment(1);
}

/// Increments the count of locks lost (expired or stolen).
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::increment_locks_lost();
/// ```
pub fn increment_locks_lost() {
    counter!(LOCKS_LOST_TOTAL).increment(1);
}

/// Increments the count of successful lock refreshes.
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::increment_lock_refreshes();
/// ```
pub fn increment_lock_refreshes() {
    counter!(LOCK_REFRESHES_TOTAL).increment(1);
}

/// Increments the count of locks released gracefully.
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics;
///
/// metrics::increment_locks_released();
/// ```
pub fn increment_locks_released() {
    counter!(LOCKS_RELEASED_TOTAL).increment(1);
}

/// Reasons for lock acquisition failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockFailureReason {
    /// Lock is already held by another instance.
    AlreadyHeld,
    /// Error communicating with state store.
    Error,
}

impl LockFailureReason {
    /// Returns the reason as a static string for metrics labels.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::AlreadyHeld => "already_held",
            Self::Error => "error",
        }
    }
}

// ============================================================================
// Pipeline Status Type
// ============================================================================

/// Pipeline status for the `pipeline_status` gauge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PipelineStatus {
    /// Pipeline is stopped.
    Stopped = 0,
    /// Pipeline is running normally.
    Running = 1,
    /// Pipeline encountered an error.
    Error = 2,
}

/// Error categories for consistent metric labeling.
///
/// This enum provides a fixed set of error types to prevent cardinality explosion
/// from using free-form error messages as labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Timeout error (operation exceeded time limit)
    Timeout,
    /// Connection error (network, TCP, DNS failures)
    Connection,
    /// Serialization error (JSON, BSON, encoding failures)
    Serialization,
    /// Permission error (authentication, authorization failures)
    Permission,
    /// Validation error (invalid data, schema violations)
    Validation,
    /// Not found error (resource doesn't exist)
    NotFound,
    /// Rate limit error (throttling, quota exceeded)
    RateLimit,
    /// Unknown error (unclassified)
    Unknown,
}

impl ErrorCategory {
    /// Returns the error category as a static string for metrics labels.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Timeout => "timeout_error",
            Self::Connection => "connection_error",
            Self::Serialization => "serialization_error",
            Self::Permission => "permission_error",
            Self::Validation => "validation_error",
            Self::NotFound => "not_found_error",
            Self::RateLimit => "rate_limit_error",
            Self::Unknown => "unknown_error",
        }
    }
}

// ============================================================================
// Metric Helper Utilities
// ============================================================================

/// Helper for timing operations and automatically recording the duration.
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::metrics::Timer;
///
/// {
///     let _timer = Timer::new("s3", |duration, dest_type| {
///         rigatoni_core::metrics::record_destination_write_duration(duration, dest_type);
///     });
///     // ... operation to time ...
/// } // Timer automatically records when dropped
/// ```
pub struct Timer<F>
where
    F: FnOnce(Duration, &str),
{
    start: std::time::Instant,
    label: String,
    record_fn: Option<F>,
}

impl<F> Timer<F>
where
    F: FnOnce(Duration, &str),
{
    /// Creates a new timer that will record the duration when dropped.
    pub fn new(label: impl Into<String>, record_fn: F) -> Self {
        Self {
            start: std::time::Instant::now(),
            label: label.into(),
            record_fn: Some(record_fn),
        }
    }
}

impl<F> Drop for Timer<F>
where
    F: FnOnce(Duration, &str),
{
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        if let Some(record_fn) = self.record_fn.take() {
            record_fn(duration, &self.label);
        }
    }
}
