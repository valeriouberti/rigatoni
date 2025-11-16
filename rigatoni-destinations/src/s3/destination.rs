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

//! S3 destination implementation.
//!
//! This module provides the main [`S3Destination`] implementation that writes
//! change events to AWS S3 (or S3-compatible storage like MinIO, LocalStack).

use crate::s3::config::{Compression, S3Config, SerializationFormat};
use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use chrono::Utc;
use rigatoni_core::destination::{Destination, DestinationError, DestinationMetadata};
use rigatoni_core::event::ChangeEvent;
use std::io::Write;
use tracing::{debug, error, info, warn};

/// S3 destination for writing change stream events to AWS S3.
///
/// This destination buffers events in memory and writes them to S3 in batches.
/// It supports multiple serialization formats, compression, and configurable
/// key generation strategies for partitioning.
///
/// # Buffering Strategy
///
/// Events are buffered in memory until either:
/// - `flush()` is called explicitly
/// - The destination is closed
///
/// This allows efficient batching of small events into larger S3 objects,
/// reducing PUT request costs and improving throughput.
///
/// # Thread Safety
///
/// This type is `Send` but not `Sync`. Each S3Destination should be owned
/// by a single writer task. For concurrent writes, create multiple instances.
///
/// # Examples
///
/// ```rust,ignore
/// use rigatoni_destinations::s3::{S3Destination, S3Config};
/// use rigatoni_core::Destination;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = S3Config::builder()
///         .bucket("my-data-lake")
///         .region("us-east-1")
///         .prefix("events/mongodb")
///         .build()?;
///
///     let mut destination = S3Destination::new(config).await?;
///
///     // Write events...
///     // destination.write_batch(events).await?;
///
///     // Flush and close
///     destination.close().await?;
///
///     Ok(())
/// }
/// ```
pub struct S3Destination {
    /// AWS S3 client
    client: S3Client,

    /// Configuration
    config: S3Config,

    /// Buffered events waiting to be written
    buffer: Vec<ChangeEvent>,

    /// Whether the destination has been closed
    closed: bool,
}

impl S3Destination {
    /// Creates a new S3 destination with the given configuration.
    ///
    /// This will initialize the AWS SDK and create an S3 client. The client
    /// will use default credential providers (environment variables, instance
    /// profiles, etc.).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - AWS SDK initialization fails
    /// - S3 client creation fails
    /// - Configuration is invalid
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use rigatoni_destinations::s3::{S3Destination, S3Config};
    ///
    /// let config = S3Config::builder()
    ///     .bucket("my-bucket")
    ///     .region("us-east-1")
    ///     .build()?;
    ///
    /// let destination = S3Destination::new(config).await?;
    /// ```
    pub async fn new(config: S3Config) -> Result<Self, DestinationError> {
        info!(
            "Initializing S3 destination: bucket={}, region={}",
            config.bucket, config.region
        );

        // Build AWS SDK config
        let mut aws_config_builder = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(config.region.clone()));

        // Configure custom endpoint if specified (for LocalStack, MinIO, etc.)
        if let Some(endpoint_url) = &config.endpoint_url {
            debug!("Using custom S3 endpoint: {}", endpoint_url);
            aws_config_builder = aws_config_builder.endpoint_url(endpoint_url);
        }

        let aws_config = aws_config_builder.load().await;

        // Build S3 client
        let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&aws_config).retry_config(
            aws_sdk_s3::config::retry::RetryConfig::standard()
                .with_max_attempts(config.max_retries),
        );

        // Configure path-style addressing if needed
        if config.force_path_style {
            debug!("Using path-style S3 addressing");
            s3_config_builder = s3_config_builder.force_path_style(true);
        }

        let s3_config = s3_config_builder.build();
        let client = S3Client::from_conf(s3_config);

        info!("S3 destination initialized successfully");

        Ok(Self {
            client,
            config,
            buffer: Vec::new(),
            closed: false,
        })
    }

    /// Returns the current number of buffered events.
    #[must_use]
    pub fn buffered_count(&self) -> usize {
        self.buffer.len()
    }

    /// Serializes events to bytes based on the configured format.
    fn serialize_events(&self, events: &[ChangeEvent]) -> Result<Vec<u8>, DestinationError> {
        match self.config.format {
            SerializationFormat::Json => Self::serialize_json(events),
            #[cfg(feature = "parquet")]
            SerializationFormat::Parquet => Self::serialize_parquet(events),
            #[cfg(feature = "csv")]
            SerializationFormat::Csv => Self::serialize_csv(events),
            #[cfg(feature = "avro")]
            SerializationFormat::Avro => Self::serialize_avro(events),
        }
    }

    /// Serializes events to newline-delimited JSON (JSONL).
    fn serialize_json(events: &[ChangeEvent]) -> Result<Vec<u8>, DestinationError> {
        let mut output = Vec::new();

        for event in events {
            let json = serde_json::to_string(event).map_err(|e| {
                DestinationError::serialization(e, "Failed to serialize event to JSON")
            })?;

            writeln!(output, "{}", json)
                .map_err(|e| DestinationError::serialization(e, "Failed to write JSON line"))?;
        }

        Ok(output)
    }

    /// Serializes events to CSV format.
    ///
    /// Each event becomes a row with flattened fields. Nested documents are serialized as JSON strings.
    #[cfg(feature = "csv")]
    fn serialize_csv(events: &[ChangeEvent]) -> Result<Vec<u8>, DestinationError> {
        use csv::WriterBuilder;

        let mut writer = WriterBuilder::new().from_writer(Vec::new());

        // Write CSV header
        writer
            .write_record([
                "operation",
                "database",
                "collection",
                "cluster_time",
                "document_key",
                "full_document",
                "resume_token",
            ])
            .map_err(|e| DestinationError::serialization(e, "Failed to write CSV header"))?;

        // Write each event as a row
        for event in events {
            let document_key = event
                .document_key
                .as_ref()
                .map(|d| serde_json::to_string(d).unwrap_or_default())
                .unwrap_or_default();

            let full_document = event
                .full_document
                .as_ref()
                .map(|d| serde_json::to_string(d).unwrap_or_default())
                .unwrap_or_default();

            let resume_token = serde_json::to_string(&event.resume_token).unwrap_or_default();

            writer
                .write_record([
                    format!("{:?}", event.operation),
                    event.namespace.database.clone(),
                    event.namespace.collection.clone(),
                    event.cluster_time.to_rfc3339(),
                    document_key,
                    full_document,
                    resume_token,
                ])
                .map_err(|e| DestinationError::serialization(e, "Failed to write CSV record"))?;
        }

        writer
            .into_inner()
            .map_err(|e| DestinationError::serialization(e, "Failed to finalize CSV"))
    }

    /// Serializes events to Apache Parquet format.
    ///
    /// Parquet is a columnar storage format that provides excellent compression
    /// and is optimized for analytics queries.
    #[cfg(feature = "parquet")]
    fn serialize_parquet(events: &[ChangeEvent]) -> Result<Vec<u8>, DestinationError> {
        // Note: Full Parquet implementation requires defining an Arrow schema
        // and converting events to Arrow RecordBatches. This is a simplified
        // implementation that stores events as JSON within Parquet for now.

        // For a production implementation, you would:
        // 1. Define an Arrow schema matching ChangeEvent structure
        // 2. Convert events to Arrow RecordBatch
        // 3. Write RecordBatch to Parquet format

        // Placeholder: store as JSON strings in a single-column Parquet file
        use arrow_array::{RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use parquet::arrow::ArrowWriter;
        use parquet::file::properties::WriterProperties;
        use std::sync::Arc;

        // Create a simple schema with event data as JSON strings
        let schema = Schema::new(vec![Field::new("event_json", DataType::Utf8, false)]);

        // Convert events to JSON strings
        let json_strings: Result<Vec<String>, _> =
            events.iter().map(serde_json::to_string).collect();

        let json_strings = json_strings.map_err(|e| {
            DestinationError::serialization(e, "Failed to serialize event to JSON for Parquet")
        })?;

        // Create Arrow array
        let array = StringArray::from(json_strings);

        // Create RecordBatch
        let batch =
            RecordBatch::try_new(Arc::new(schema.clone()), vec![Arc::new(array)]).map_err(|e| {
                DestinationError::serialization(e, "Failed to create Arrow RecordBatch")
            })?;

        // Write to Parquet
        let mut buffer = Vec::new();
        let props = WriterProperties::builder().build();
        let mut writer = ArrowWriter::try_new(&mut buffer, Arc::new(schema), Some(props))
            .map_err(|e| DestinationError::serialization(e, "Failed to create Parquet writer"))?;

        writer
            .write(&batch)
            .map_err(|e| DestinationError::serialization(e, "Failed to write Parquet batch"))?;

        writer
            .close()
            .map_err(|e| DestinationError::serialization(e, "Failed to close Parquet writer"))?;

        Ok(buffer)
    }

    /// Serializes events to Apache Avro format.
    ///
    /// Avro provides schema evolution support and compact binary serialization.
    #[cfg(feature = "avro")]
    fn serialize_avro(events: &[ChangeEvent]) -> Result<Vec<u8>, DestinationError> {
        use apache_avro::{Schema as AvroSchema, Writer as AvroWriter};
        use serde::Serialize;

        // Define a serializable representation of ChangeEvent for Avro
        #[derive(Serialize)]
        struct AvroChangeEvent {
            operation: String,
            database: String,
            collection: String,
            cluster_time: String,
            document_key: Option<String>,
            full_document: Option<String>,
            resume_token: String,
        }

        // Define Avro schema
        let schema_str = r#"
        {
            "type": "record",
            "name": "ChangeEvent",
            "namespace": "rigatoni",
            "fields": [
                {"name": "operation", "type": "string"},
                {"name": "database", "type": "string"},
                {"name": "collection", "type": "string"},
                {"name": "cluster_time", "type": "string"},
                {"name": "document_key", "type": ["null", "string"], "default": null},
                {"name": "full_document", "type": ["null", "string"], "default": null},
                {"name": "resume_token", "type": "string"}
            ]
        }
        "#;

        let schema = AvroSchema::parse_str(schema_str)
            .map_err(|e| DestinationError::serialization(e, "Failed to parse Avro schema"))?;

        let mut writer = AvroWriter::new(&schema, Vec::new());

        // Convert and write each event
        for event in events {
            let avro_event = AvroChangeEvent {
                operation: format!("{:?}", event.operation),
                database: event.namespace.database.clone(),
                collection: event.namespace.collection.clone(),
                cluster_time: event.cluster_time.to_rfc3339(),
                document_key: event
                    .document_key
                    .as_ref()
                    .and_then(|d| serde_json::to_string(d).ok()),
                full_document: event
                    .full_document
                    .as_ref()
                    .and_then(|d| serde_json::to_string(d).ok()),
                resume_token: serde_json::to_string(&event.resume_token)
                    .unwrap_or_else(|_| String::from("{}")),
            };

            writer
                .append_ser(avro_event)
                .map_err(|e| DestinationError::serialization(e, "Failed to append Avro record"))?;
        }

        writer
            .flush()
            .map_err(|e| DestinationError::serialization(e, "Failed to flush Avro writer"))?;

        writer
            .into_inner()
            .map_err(|e| DestinationError::serialization(e, "Failed to get Avro buffer"))
    }

    /// Compresses data based on the configured compression algorithm.
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, DestinationError> {
        match self.config.compression {
            Compression::None => Ok(data.to_vec()),
            #[cfg(feature = "gzip")]
            Compression::Gzip => Self::compress_gzip(data),
            #[cfg(feature = "zstandard")]
            Compression::Zstd => Self::compress_zstd(data),
        }
    }

    /// Compresses data using gzip.
    #[cfg(feature = "gzip")]
    fn compress_gzip(data: &[u8]) -> Result<Vec<u8>, DestinationError> {
        use flate2::write::GzEncoder;
        use flate2::Compression as GzCompression;

        let mut encoder = GzEncoder::new(Vec::new(), GzCompression::default());
        encoder
            .write_all(data)
            .map_err(|e| DestinationError::serialization(e, "Failed to compress with gzip"))?;

        encoder
            .finish()
            .map_err(|e| DestinationError::serialization(e, "Failed to finalize gzip compression"))
    }

    /// Compresses data using zstd.
    #[cfg(feature = "zstandard")]
    fn compress_zstd(data: &[u8]) -> Result<Vec<u8>, DestinationError> {
        let mut encoder = zstd::Encoder::new(Vec::new(), 3)
            .map_err(|e| DestinationError::serialization(e, "Failed to create zstd encoder"))?;

        encoder
            .write_all(data)
            .map_err(|e| DestinationError::serialization(e, "Failed to compress with zstd"))?;

        encoder
            .finish()
            .map_err(|e| DestinationError::serialization(e, "Failed to finalize zstd compression"))
    }

    /// Generates an S3 key for the given collection.
    fn generate_key(&self, collection: &str) -> String {
        let timestamp = Utc::now();
        let extension = self.config.format.extension();
        let compression_ext = self.config.compression.extension();

        self.config.key_strategy.generate_key(
            self.config.prefix.as_deref(),
            collection,
            &timestamp,
            extension,
            compression_ext,
        )
    }

    /// Writes buffered events to S3.
    async fn write_to_s3(&mut self) -> Result<(), DestinationError> {
        if self.buffer.is_empty() {
            debug!("No events to write, skipping S3 upload");
            return Ok(());
        }

        // Group events by collection for efficient partitioning
        let mut events_by_collection: std::collections::HashMap<String, Vec<ChangeEvent>> =
            std::collections::HashMap::new();

        for event in self.buffer.drain(..) {
            events_by_collection
                .entry(event.collection_name().to_string())
                .or_default()
                .push(event);
        }

        // Write each collection to a separate S3 object
        for (collection, events) in events_by_collection {
            let count = events.len();
            debug!("Writing {} events for collection '{}'", count, collection);

            // Serialize events
            let serialized = self.serialize_events(&events)?;
            let uncompressed_size = serialized.len();

            // Compress if configured
            let data = self.compress(&serialized)?;
            let compressed_size = data.len();

            // Generate S3 key
            let key = self.generate_key(&collection);

            // Upload to S3
            debug!(
                "Uploading to s3://{}/{} (size: {} bytes, compressed: {} bytes)",
                self.config.bucket, key, uncompressed_size, compressed_size
            );

            let result = self
                .client
                .put_object()
                .bucket(&self.config.bucket)
                .key(&key)
                .body(ByteStream::from(data))
                .content_type(self.config.format.content_type())
                .send()
                .await;

            match result {
                Ok(_) => {
                    info!(
                        "Successfully wrote {} events to s3://{}/{}",
                        count, self.config.bucket, key
                    );
                }
                Err(e) => {
                    error!("Failed to write to S3: {}", e);
                    return Err(Self::classify_s3_error(e));
                }
            }
        }

        Ok(())
    }

    /// Classifies S3 SDK errors into appropriate `DestinationError` variants.
    fn classify_s3_error(
        error: aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::put_object::PutObjectError>,
    ) -> DestinationError {
        use aws_sdk_s3::error::SdkError;

        match error {
            // Network/connection errors - retryable
            SdkError::TimeoutError(_) | SdkError::DispatchFailure(_) => {
                DestinationError::connection(error)
            }

            // Service errors - check specific error type
            SdkError::ServiceError(ref service_err) => {
                let err_msg = service_err.err().to_string();

                // Check for common retryable conditions
                if err_msg.contains("SlowDown")
                    || err_msg.contains("ServiceUnavailable")
                    || err_msg.contains("InternalError")
                {
                    DestinationError::write(error, true)
                } else if err_msg.contains("AccessDenied") || err_msg.contains("InvalidBucketName")
                {
                    // Non-retryable authorization/configuration errors
                    DestinationError::write(error, false)
                } else {
                    // Default to non-retryable for unknown service errors
                    DestinationError::write(error, false)
                }
            }

            // Construction errors - configuration issue
            SdkError::ConstructionFailure(_) => {
                DestinationError::configuration(error.to_string(), Some("s3_client".to_string()))
            }

            // Other errors - default to non-retryable
            _ => DestinationError::other(error, false),
        }
    }
}

#[async_trait]
impl Destination for S3Destination {
    async fn write_batch(&mut self, events: &[ChangeEvent]) -> Result<(), DestinationError> {
        if self.closed {
            return Err(DestinationError::write_msg(
                "Cannot write to closed S3 destination",
                false,
            ));
        }

        if events.is_empty() {
            debug!("Received empty batch, skipping");
            return Ok(());
        }

        debug!("Buffering {} events for S3", events.len());
        self.buffer.extend_from_slice(events);

        Ok(())
    }

    async fn flush(&mut self) -> Result<(), DestinationError> {
        if self.closed {
            warn!("Attempted to flush closed S3 destination");
            return Ok(());
        }

        debug!("Flushing {} buffered events to S3", self.buffer.len());
        self.write_to_s3().await
    }

    fn supports_transactions(&self) -> bool {
        // S3 does not support ACID transactions
        false
    }

    async fn close(&mut self) -> Result<(), DestinationError> {
        if self.closed {
            debug!("S3 destination already closed");
            return Ok(());
        }

        info!("Closing S3 destination");

        // Flush any remaining buffered events
        self.flush().await?;

        self.closed = true;
        info!("S3 destination closed successfully");

        Ok(())
    }

    fn metadata(&self) -> DestinationMetadata {
        let format_str = match self.config.format {
            SerializationFormat::Json => "json",
            #[cfg(feature = "parquet")]
            SerializationFormat::Parquet => "parquet",
            #[cfg(feature = "csv")]
            SerializationFormat::Csv => "csv",
            #[cfg(feature = "avro")]
            SerializationFormat::Avro => "avro",
        };

        let compression_str = match self.config.compression {
            Compression::None => "none",
            #[cfg(feature = "gzip")]
            Compression::Gzip => "gzip",
            #[cfg(feature = "zstandard")]
            Compression::Zstd => "zstd",
        };

        let key_strategy_str = self.config.key_strategy.pattern_description();

        DestinationMetadata::new("AWS S3", "s3")
            .with_transactions(false)
            .with_concurrent_writes(true)
            .with_property("bucket", &self.config.bucket)
            .with_property("region", &self.config.region)
            .with_property("format", format_str)
            .with_property("compression", compression_str)
            .with_property("key_pattern", key_strategy_str)
            .with_property("prefix", self.config.prefix.as_deref().unwrap_or(""))
    }
}

impl Drop for S3Destination {
    fn drop(&mut self) {
        if !self.closed && !self.buffer.is_empty() {
            warn!(
                "S3Destination dropped without calling close() - {} buffered events lost!",
                self.buffer.len()
            );
        }
    }
}
