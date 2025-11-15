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

use async_trait::async_trait;
use rigatoni_core::destination::{Destination, DestinationError, DestinationMetadata};
use rigatoni_core::event::ChangeEvent;
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
    async fn write_batch(&mut self, events: &[ChangeEvent]) -> Result<(), DestinationError> {
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

        self.events.extend_from_slice(events);
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
    use bson::doc;
    use chrono::Utc;
    use rigatoni_core::event::{Namespace, OperationType};

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
        dest.write_batch(&events).await.unwrap();

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

        dest.write_batch(&[]).await.unwrap();
        assert_eq!(dest.total_events_written(), 0);
    }

    #[tokio::test]
    async fn test_mock_destination_write_failures() {
        let mut dest = MockDestination::new().with_write_failures();

        let events = vec![create_test_event()];
        let result = dest.write_batch(&events).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DestinationError::WriteError { .. }));
        assert!(err.is_retryable());
    }

    #[tokio::test]
    async fn test_mock_destination_capacity_errors() {
        let mut dest = MockDestination::new().with_capacity_errors();

        let events = vec![create_test_event()];
        let result = dest.write_batch(&events).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DestinationError::CapacityError { .. }));
        assert!(err.is_retryable());
        assert_eq!(err.retry_after(), Some(std::time::Duration::from_secs(1)));
    }

    #[tokio::test]
    async fn test_mock_destination_reset() {
        let mut dest = MockDestination::new();

        dest.write_batch(&[create_test_event()]).await.unwrap();
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
