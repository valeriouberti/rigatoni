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

//! Pipeline orchestration for MongoDB change streams to destinations.
//!
//! The [`Pipeline`] is the core orchestrator that connects MongoDB change streams
//! to destinations. It handles:
//!
//! - **Batching**: Accumulate events and flush based on size or timeout
//! - **Retry Logic**: Exponential backoff for failed destination writes
//! - **State Management**: Persist resume tokens after successful writes
//! - **Back-pressure**: Slow down MongoDB reads if destination is slow
//! - **Graceful Shutdown**: Flush pending batches and save state
//! - **Observability**: Structured logging and metrics
//!
//! # Example
//!
//! ```rust,no_run
//! use rigatoni_core::pipeline::{Pipeline, PipelineConfig};
//! use rigatoni_core::stream::ChangeStreamConfig;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Configure pipeline
//! let config = PipelineConfig::builder()
//!     .mongodb_uri("mongodb://localhost:27017")
//!     .database("mydb")
//!     .collections(vec!["users".to_string(), "orders".to_string()])
//!     .batch_size(100)
//!     .batch_timeout(Duration::from_secs(5))
//!     .max_retries(3)
//!     .build()?;
//!
//! // Create pipeline with state store and destination
//! // let pipeline = Pipeline::new(config, store, destination).await?;
//!
//! // Start processing
//! // pipeline.start().await?;
//!
//! // Graceful shutdown
//! // pipeline.stop().await?;
//! # Ok(())
//! # }
//! ```

use crate::destination::{Destination, DestinationError};
use crate::event::ChangeEvent;
use crate::metrics;
use crate::state::StateStore;
use crate::stream::{ChangeStreamConfig, ChangeStreamListener};
use futures::StreamExt;
use mongodb::bson::Document;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{interval, Instant};
use tracing::{debug, error, info, instrument, warn};

/// Configuration for the pipeline orchestrator.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// MongoDB connection URI
    pub mongodb_uri: String,

    /// Database name to watch
    pub database: String,

    /// Collections to watch (empty = all collections)
    pub collections: Vec<String>,

    /// Maximum number of events to batch before flushing
    pub batch_size: usize,

    /// Maximum time to wait before flushing a batch
    pub batch_timeout: Duration,

    /// Maximum number of retry attempts for failed writes
    pub max_retries: usize,

    /// Initial retry delay (doubles with each retry)
    pub retry_delay: Duration,

    /// Maximum retry delay
    pub max_retry_delay: Duration,

    /// Channel buffer size for back-pressure
    pub channel_buffer_size: usize,

    /// Change stream configuration
    pub stream_config: ChangeStreamConfig,
}

impl PipelineConfig {
    /// Creates a new builder for `PipelineConfig`.
    #[must_use]
    pub fn builder() -> PipelineConfigBuilder {
        PipelineConfigBuilder::default()
    }
}

/// Builder for `PipelineConfig`.
#[derive(Debug, Default)]
pub struct PipelineConfigBuilder {
    mongodb_uri: Option<String>,
    database: Option<String>,
    collections: Vec<String>,
    batch_size: usize,
    batch_timeout: Duration,
    max_retries: usize,
    retry_delay: Duration,
    max_retry_delay: Duration,
    channel_buffer_size: usize,
    stream_config: Option<ChangeStreamConfig>,
}

impl PipelineConfigBuilder {
    /// Sets the MongoDB URI.
    #[must_use]
    pub fn mongodb_uri(mut self, uri: impl Into<String>) -> Self {
        self.mongodb_uri = Some(uri.into());
        self
    }

    /// Sets the database name.
    #[must_use]
    pub fn database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Sets the collections to watch.
    #[must_use]
    pub fn collections(mut self, collections: Vec<String>) -> Self {
        self.collections = collections;
        self
    }

    /// Sets the batch size.
    #[must_use]
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Sets the batch timeout.
    #[must_use]
    pub fn batch_timeout(mut self, timeout: Duration) -> Self {
        self.batch_timeout = timeout;
        self
    }

    /// Sets the maximum number of retries.
    #[must_use]
    pub fn max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }

    /// Sets the initial retry delay.
    #[must_use]
    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// Sets the maximum retry delay.
    #[must_use]
    pub fn max_retry_delay(mut self, delay: Duration) -> Self {
        self.max_retry_delay = delay;
        self
    }

    /// Sets the channel buffer size.
    #[must_use]
    pub fn channel_buffer_size(mut self, size: usize) -> Self {
        self.channel_buffer_size = size;
        self
    }

    /// Sets the change stream configuration.
    #[must_use]
    pub fn stream_config(mut self, config: ChangeStreamConfig) -> Self {
        self.stream_config = Some(config);
        self
    }

    /// Builds the `PipelineConfig`.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing or invalid.
    pub fn build(self) -> Result<PipelineConfig, ConfigError> {
        let mongodb_uri = self.mongodb_uri.ok_or(ConfigError::MissingMongoUri)?;
        let database = self.database.ok_or(ConfigError::MissingDatabase)?;

        // Validate batch size
        let batch_size = match self.batch_size {
            0 => 100, // Default
            size if size > 10_000 => {
                return Err(ConfigError::InvalidBatchSize {
                    value: size,
                    reason: "batch_size exceeds maximum (10,000)",
                })
            }
            size => size,
        };

        // Validate batch timeout
        let batch_timeout = if self.batch_timeout.is_zero() {
            Duration::from_secs(5) // Default
        } else {
            self.batch_timeout
        };

        // Set retry delays with defaults
        let retry_delay = if self.retry_delay.is_zero() {
            Duration::from_millis(100)
        } else {
            self.retry_delay
        };

        let max_retry_delay = if self.max_retry_delay.is_zero() {
            Duration::from_secs(30)
        } else {
            self.max_retry_delay
        };

        // Cross-field validation: retry_delay must not exceed max_retry_delay
        if retry_delay > max_retry_delay {
            return Err(ConfigError::RetryDelayExceedsMax {
                retry_delay,
                max_retry_delay,
            });
        }

        // Validate channel buffer size
        let channel_buffer_size = match self.channel_buffer_size {
            0 => 1000, // Default
            size if size < 10 => {
                return Err(ConfigError::InvalidChannelBufferSize {
                    value: size,
                    reason: "channel_buffer_size must be at least 10",
                })
            }
            size => size,
        };

        let stream_config = self.stream_config.unwrap_or_else(|| {
            ChangeStreamConfig::builder()
                .build()
                .expect("Default stream config should build")
        });

        Ok(PipelineConfig {
            mongodb_uri,
            database,
            collections: self.collections,
            batch_size,
            batch_timeout,
            max_retries: self.max_retries,
            retry_delay,
            max_retry_delay,
            channel_buffer_size,
            stream_config,
        })
    }
}

/// Pipeline statistics.
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    /// Total events processed
    pub events_processed: u64,

    /// Total batches written
    pub batches_written: u64,

    /// Total write errors
    pub write_errors: u64,

    /// Total retries
    pub retries: u64,
}

/// Type alias for worker task handles.
type WorkerHandle = JoinHandle<Result<(), PipelineError>>;

/// Pipeline orchestrator that connects MongoDB change streams to destinations.
pub struct Pipeline<S: StateStore, D: Destination> {
    /// Pipeline configuration
    config: PipelineConfig,

    /// State store for resume tokens
    store: Arc<S>,

    /// Destination for events
    destination: Arc<Mutex<D>>,

    /// Shutdown sender (taken when starting)
    shutdown_tx: Option<broadcast::Sender<()>>,

    /// Worker task handles
    workers: Arc<RwLock<Vec<WorkerHandle>>>,

    /// Pipeline statistics
    stats: Arc<RwLock<PipelineStats>>,

    /// Running flag
    running: Arc<RwLock<bool>>,
}

impl<S: StateStore + Send + Sync + 'static, D: Destination + Send + Sync + 'static> Pipeline<S, D> {
    /// Creates a new pipeline instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the destination cannot be initialized.
    pub async fn new(
        config: PipelineConfig,
        store: S,
        destination: D,
    ) -> Result<Self, PipelineError> {
        info!(
            database = %config.database,
            collections = ?config.collections,
            batch_size = config.batch_size,
            batch_timeout = ?config.batch_timeout,
            "Creating pipeline"
        );

        Ok(Self {
            config,
            store: Arc::new(store),
            destination: Arc::new(Mutex::new(destination)),
            shutdown_tx: None,
            workers: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(PipelineStats::default())),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Starts the pipeline, spawning workers for each collection.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The pipeline is already running
    /// - MongoDB connection fails
    /// - Worker spawn fails
    #[instrument(skip(self), fields(database = %self.config.database))]
    pub async fn start(&mut self) -> Result<(), PipelineError> {
        // Check if already running
        let mut running = self.running.write().await;
        if *running {
            return Err(PipelineError::AlreadyRunning);
        }

        // Early validation: watching all collections is not yet supported
        if self.config.collections.is_empty() {
            return Err(PipelineError::Configuration(
                "Watching all collections is not yet implemented. \
                 Please specify explicit collections in the configuration."
                    .to_string(),
            ));
        }

        info!("Starting pipeline");

        // Create shutdown channel (broadcast so all workers get the signal)
        let (shutdown_tx, _) = broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx.clone());

        let collections = self.config.collections.clone();
        let num_collections = collections.len();

        // Spawn worker for each collection
        let mut workers = self.workers.write().await;
        for collection in collections {
            let shutdown_rx = shutdown_tx.subscribe();
            let worker = self
                .spawn_collection_worker(collection.clone(), shutdown_rx)
                .await?;

            workers.push(worker);
        }

        *running = true;
        info!(workers = workers.len(), "Pipeline started");

        // Update metrics
        metrics::set_pipeline_status(metrics::PipelineStatus::Running);
        metrics::set_active_collections(num_collections);

        Ok(())
    }

    /// Spawns a worker task for a specific collection.
    async fn spawn_collection_worker(
        &self,
        collection: String,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<WorkerHandle, PipelineError> {
        let config = self.config.clone();
        let store = Arc::clone(&self.store);
        let destination = Arc::clone(&self.destination);
        let stats = Arc::clone(&self.stats);

        let collection_name = if collection.is_empty() {
            "all".to_string()
        } else {
            collection
        };

        let handle = tokio::spawn(async move {
            Self::collection_worker(
                collection_name,
                config,
                store,
                destination,
                stats,
                shutdown_rx,
            )
            .await
        });

        Ok(handle)
    }

    /// Worker task that processes events for a collection.
    #[allow(clippy::too_many_lines)]
    #[instrument(skip(config, store, destination, stats, shutdown_rx), fields(collection = %collection))]
    async fn collection_worker(
        collection: String,
        config: PipelineConfig,
        store: Arc<S>,
        destination: Arc<Mutex<D>>,
        stats: Arc<RwLock<PipelineStats>>,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), PipelineError> {
        info!("Starting collection worker");

        // Get resume token from state store
        let resume_token = store
            .get_resume_token(&collection)
            .await
            .map_err(|e| PipelineError::StateStore(e.to_string()))?;

        if let Some(ref token) = resume_token {
            info!(?token, "Resuming from saved token");
        }

        // Connect to MongoDB
        let client = mongodb::Client::with_uri_str(&config.mongodb_uri)
            .await
            .map_err(|e| PipelineError::MongoDB(e.to_string()))?;

        let db = client.database(&config.database);

        // Get the collection to watch
        let mongo_collection = if !collection.is_empty() && collection != "all" {
            db.collection(&collection)
        } else {
            // Watch all collections in the database
            // For now, we'll watch a specific collection - watching all is more complex
            // TODO: Support watching entire database
            return Err(PipelineError::Configuration(
                "Watching all collections not yet implemented".to_string(),
            ));
        };

        // Create resume token callback that saves to state store
        let store_clone = Arc::clone(&store);
        let collection_clone = collection.clone();
        let resume_token_callback = move |token: Document| {
            let store = Arc::clone(&store_clone);
            let coll = collection_clone.clone();
            Box::pin(async move {
                store
                    .save_resume_token(&coll, &token)
                    .await
                    .map_err(|e| e.to_string())
            }) as Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
        };

        // Create change stream listener
        let mut listener = ChangeStreamListener::new(
            mongo_collection,
            config.stream_config.clone(),
            resume_token_callback,
        )
        .await
        .map_err(|e| PipelineError::ChangeStream(e.to_string()))?;

        // Event batch accumulator
        let mut batch: Vec<ChangeEvent> = Vec::with_capacity(config.batch_size);
        let mut last_resume_token: Option<Document> = None;

        // Batch timeout interval
        let mut batch_timer = interval(config.batch_timeout);
        batch_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        info!(
            batch_size = config.batch_size,
            batch_timeout = ?config.batch_timeout,
            "Worker event loop started"
        );

        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.recv() => {
                    info!("Received shutdown signal");

                    // Flush pending batch
                    if !batch.is_empty() {
                        info!(batch_size = batch.len(), "Flushing pending batch on shutdown");
                        if let Err(e) = Self::flush_batch(
                            &collection,
                            &mut batch,
                            last_resume_token.as_ref(),
                            &destination,
                            &store,
                            &stats,
                            &config,
                        )
                        .await
                        {
                            error!(?e, "Failed to flush batch on shutdown");
                        }
                    }

                    info!("Worker shutting down gracefully");
                    break;
                }

                // Batch timeout - flush accumulated events
                _ = batch_timer.tick() => {
                    if !batch.is_empty() {
                        debug!(batch_size = batch.len(), "Batch timeout - flushing");

                        if let Err(e) = Self::flush_batch(
                            &collection,
                            &mut batch,
                            last_resume_token.as_ref(),
                            &destination,
                            &store,
                            &stats,
                            &config,
                        )
                        .await
                        {
                            error!(?e, "Failed to flush batch on timeout");
                            // Continue processing - don't break the loop
                        }
                    }
                }

                // Read next event from change stream
                event_result = listener.next() => {
                    match event_result {
                        Some(Ok(ackable_event)) => {
                            // Extract the change event
                            let event = ackable_event.event.clone();

                            debug!(
                                operation = ?event.operation,
                                collection = %event.namespace.collection,
                                "Received event"
                            );

                            // Store resume token
                            last_resume_token = Some(event.resume_token.clone());

                            // Acknowledge the event (sends resume token to callback)
                            ackable_event.ack();

                            // Add to batch
                            batch.push(event.clone());

                            // Update metrics
                            metrics::increment_batch_queue_size(&collection);

                            // Check if batch is full
                            if batch.len() >= config.batch_size {
                                debug!(batch_size = batch.len(), "Batch full - flushing");

                                if let Err(e) = Self::flush_batch(
                                    &collection,
                                    &mut batch,
                                    last_resume_token.as_ref(),
                                    &destination,
                                    &store,
                                    &stats,
                                    &config,
                                )
                                .await
                                {
                                    error!(?e, "Failed to flush full batch");
                                    // Continue processing
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!(?e, "Error reading from change stream");
                            // Try to reconnect after a delay
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                        None => {
                            // Stream ended - shouldn't happen with MongoDB change streams
                            warn!("Change stream ended unexpectedly");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Flushes a batch of events to the destination with retry logic.
    #[instrument(skip(batch, last_resume_token, destination, store, stats, config), fields(collection = %collection, batch_size = batch.len()))]
    async fn flush_batch(
        collection: &str,
        batch: &mut Vec<ChangeEvent>,
        last_resume_token: Option<&Document>,
        destination: &Arc<Mutex<D>>,
        store: &Arc<S>,
        stats: &Arc<RwLock<PipelineStats>>,
        config: &PipelineConfig,
    ) -> Result<(), PipelineError> {
        if batch.is_empty() {
            return Ok(());
        }

        let batch_size = batch.len();
        let start_time = Instant::now();

        debug!("Flushing batch to destination");

        // Record batch size metric
        metrics::record_batch_size(batch_size, collection);

        // Write to destination with retry
        Self::write_with_retry(batch, destination, config, stats).await?;

        let elapsed = start_time.elapsed();
        info!(
            batch_size,
            elapsed_ms = elapsed.as_millis(),
            "Batch written successfully"
        );

        // Record batch duration metric
        metrics::record_batch_duration(elapsed.as_secs_f64(), collection);

        // Save resume token after successful write
        if let Some(token) = last_resume_token {
            store
                .save_resume_token(collection, token)
                .await
                .map_err(|e| PipelineError::StateStore(e.to_string()))?;

            debug!("Resume token saved");
        }

        // Count processed events (bulk increment by operation type)
        let mut operation_counts = std::collections::HashMap::new();
        for event in batch.iter() {
            *operation_counts.entry(&event.operation).or_insert(0u64) += 1;
        }
        for (operation, count) in operation_counts {
            metrics::increment_events_processed_by(count, collection, operation.as_str());
        }

        // Update statistics
        let mut s = stats.write().await;
        s.events_processed += batch_size as u64;
        s.batches_written += 1;

        // Update queue size metric
        metrics::decrement_batch_queue_size(batch_size, collection);

        // Clear batch
        batch.clear();

        Ok(())
    }

    /// Writes a batch to the destination with exponential backoff retry.
    #[instrument(skip(batch, destination, config, stats), fields(batch_size = batch.len()))]
    async fn write_with_retry(
        batch: &[ChangeEvent],
        destination: &Arc<Mutex<D>>,
        config: &PipelineConfig,
        stats: &Arc<RwLock<PipelineStats>>,
    ) -> Result<(), PipelineError> {
        let mut retry_delay = config.retry_delay;
        let mut attempt = 0;

        loop {
            let result = {
                let mut dest = destination.lock().await;
                match dest.write_batch(batch).await {
                    Ok(()) => dest.flush().await,
                    Err(e) => Err(e),
                }
            };

            match result {
                Ok(()) => {
                    if attempt > 0 {
                        info!(attempts = attempt + 1, "Write succeeded after retries");
                    }
                    return Ok(());
                }
                Err(e) => {
                    // Increment write error counter for each failed attempt
                    {
                        let mut s = stats.write().await;
                        s.write_errors += 1;
                    }

                    // Record error type metric
                    let error_category = Self::categorize_error(&e);
                    let destination_type = {
                        let dest = destination.lock().await;
                        dest.metadata().destination_type.clone()
                    };
                    metrics::increment_destination_errors(&destination_type, error_category);

                    attempt += 1;

                    if attempt > config.max_retries {
                        error!(attempts = attempt, ?e, "Write failed after max retries");
                        return Err(PipelineError::Destination(e.to_string()));
                    }

                    // Check if error is retryable
                    if !Self::is_retryable_error(&e) {
                        error!(?e, "Non-retryable error encountered");
                        return Err(PipelineError::Destination(e.to_string()));
                    }

                    // Increment retry counter (only for actual retries, not initial attempt)
                    {
                        let mut s = stats.write().await;
                        s.retries += 1;
                    }

                    // Record retry metric
                    metrics::increment_retries(error_category);

                    warn!(
                        attempt,
                        max_retries = config.max_retries,
                        retry_delay_ms = retry_delay.as_millis(),
                        ?e,
                        "Write failed, retrying"
                    );

                    // Wait before retry
                    tokio::time::sleep(retry_delay).await;

                    // Exponential backoff with cap
                    retry_delay = std::cmp::min(retry_delay * 2, config.max_retry_delay);
                }
            }
        }
    }

    /// Checks if a destination error is retryable.
    fn is_retryable_error(error: &DestinationError) -> bool {
        // Check if the error indicates it's retryable
        // This depends on the DestinationError implementation
        error.to_string().contains("retryable") || error.to_string().contains("timeout")
    }

    /// Categorizes an error for metrics labeling.
    ///
    /// Maps errors to a small set of categories to avoid cardinality explosion.
    fn categorize_error(error: &DestinationError) -> metrics::ErrorCategory {
        let error_str = error.to_string().to_lowercase();

        if error_str.contains("timeout") {
            metrics::ErrorCategory::Timeout
        } else if error_str.contains("connection") || error_str.contains("network") {
            metrics::ErrorCategory::Connection
        } else if error_str.contains("serialization") || error_str.contains("encode") {
            metrics::ErrorCategory::Serialization
        } else if error_str.contains("permission") || error_str.contains("auth") {
            metrics::ErrorCategory::Permission
        } else if error_str.contains("validation") {
            metrics::ErrorCategory::Validation
        } else if error_str.contains("not found") || error_str.contains("404") {
            metrics::ErrorCategory::NotFound
        } else if error_str.contains("rate limit") || error_str.contains("throttle") {
            metrics::ErrorCategory::RateLimit
        } else {
            metrics::ErrorCategory::Unknown
        }
    }

    /// Stops the pipeline gracefully.
    ///
    /// This will:
    /// 1. Send shutdown signal to all workers
    /// 2. Wait for workers to finish processing
    /// 3. Flush any pending batches
    /// 4. Close destination connection
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown fails or workers panic.
    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> Result<(), PipelineError> {
        info!("Stopping pipeline");

        let mut running = self.running.write().await;
        if !*running {
            warn!("Pipeline is not running");
            return Ok(());
        }

        // Send shutdown signal (broadcast to all workers)
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Wait for all workers to finish
        let mut workers = self.workers.write().await;
        for worker in workers.drain(..) {
            match worker.await {
                Ok(Ok(())) => {
                    debug!("Worker stopped successfully");
                }
                Ok(Err(e)) => {
                    error!(?e, "Worker stopped with error");
                }
                Err(e) => {
                    error!(?e, "Worker panicked");
                }
            }
        }

        // Flush and close destination
        let mut dest = self.destination.lock().await;
        dest.flush()
            .await
            .map_err(|e| PipelineError::Destination(e.to_string()))?;
        dest.close()
            .await
            .map_err(|e| PipelineError::Destination(e.to_string()))?;

        *running = false;

        // Update metrics
        metrics::set_pipeline_status(metrics::PipelineStatus::Stopped);
        metrics::set_active_collections(0);

        // Log final statistics
        let stats = self.stats.read().await;
        info!(
            events_processed = stats.events_processed,
            batches_written = stats.batches_written,
            write_errors = stats.write_errors,
            retries = stats.retries,
            "Pipeline stopped"
        );

        Ok(())
    }

    /// Returns the current pipeline statistics.
    #[must_use]
    pub async fn stats(&self) -> PipelineStats {
        self.stats.read().await.clone()
    }

    /// Checks if the pipeline is currently running.
    #[must_use]
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// Pipeline configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Missing required MongoDB URI
    #[error("mongodb_uri is required")]
    MissingMongoUri,

    /// Missing required database name
    #[error("database is required")]
    MissingDatabase,

    /// Invalid batch size
    #[error("Invalid batch_size: {value} ({reason})")]
    InvalidBatchSize { value: usize, reason: &'static str },

    /// Invalid batch timeout
    #[error("Invalid batch_timeout: {reason}")]
    InvalidBatchTimeout { reason: &'static str },

    /// Retry delay exceeds maximum
    #[error("retry_delay ({retry_delay:?}) exceeds max_retry_delay ({max_retry_delay:?})")]
    RetryDelayExceedsMax {
        retry_delay: Duration,
        max_retry_delay: Duration,
    },

    /// Invalid channel buffer size
    #[error("Invalid channel_buffer_size: {value} ({reason})")]
    InvalidChannelBufferSize { value: usize, reason: &'static str },
}

/// Pipeline errors.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// Pipeline is already running
    #[error("Pipeline is already running")]
    AlreadyRunning,

    /// MongoDB connection error
    #[error("MongoDB error: {0}")]
    MongoDB(String),

    /// Change stream error
    #[error("Change stream error: {0}")]
    ChangeStream(String),

    /// Destination error
    #[error("Destination error: {0}")]
    Destination(String),

    /// State store error
    #[error("State store error: {0}")]
    StateStore(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Other errors
    #[error("Pipeline error: {0}")]
    Other(String),
}
