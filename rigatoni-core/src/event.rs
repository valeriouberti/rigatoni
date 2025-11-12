//! MongoDB Change Stream Event Representation
//!
//! This module defines the core event types used throughout the Rigatoni ETL pipeline.
//! Events represent MongoDB change stream operations and flow from sources to destinations.
//!
//! # Examples
//!
//! ```rust
//! use rigatoni_core::event::{ChangeEvent, OperationType, Namespace};
//! use bson::{doc, Document};
//! use chrono::Utc;
//!
//! // Create an insert event manually
//! let event = ChangeEvent {
//!     operation: OperationType::Insert,
//!     namespace: Namespace {
//!         database: "mydb".to_string(),
//!         collection: "users".to_string(),
//!     },
//!     document_key: Some(doc! { "_id": 123 }),
//!     full_document: Some(doc! {
//!         "_id": 123,
//!         "name": "Alice",
//!         "email": "alice@example.com"
//!     }),
//!     update_description: None,
//!     cluster_time: Utc::now(),
//!     resume_token: doc! { "_data": "token123" },
//! };
//!
//! // Check operation type
//! assert!(event.is_insert());
//! assert_eq!(event.collection_name(), "users");
//!
//! // Access document data
//! if let Some(doc) = &event.full_document {
//!     println!("Inserted: {:?}", doc);
//! }
//! ```

use bson::Document;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Error that can occur when converting from MongoDB driver's ChangeStreamEvent.
#[derive(Debug, Clone)]
pub enum ConversionError {
    /// Failed to convert resume token to BSON document
    ResumeTokenConversion(String),
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionError::ResumeTokenConversion(msg) => {
                write!(f, "Failed to convert resume token: {}", msg)
            }
        }
    }
}

impl std::error::Error for ConversionError {}

/// MongoDB change stream operation types.
///
/// Represents all possible operations that can occur in a MongoDB change stream.
/// Each variant corresponds to a specific database operation.
///
/// The `Unknown` variant allows forward compatibility with future MongoDB versions
/// that may introduce new operation types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum OperationType {
    /// A document was inserted into a collection
    Insert,

    /// A document was updated (modified in place)
    Update,

    /// A document was deleted from a collection
    Delete,

    /// A document was replaced entirely (all fields changed)
    Replace,

    /// The change stream was invalidated (collection dropped, renamed, etc.)
    Invalidate,

    /// A collection was dropped
    Drop,

    /// A database was dropped
    #[serde(rename = "dropdatabase")]
    DropDatabase,

    /// A collection was renamed
    Rename,

    /// An unknown operation type from a newer MongoDB version
    ///
    /// Contains the original operation type string for logging and debugging.
    #[serde(untagged)]
    Unknown(String),
}

impl OperationType {
    /// Returns true if this operation modifies data (insert, update, replace).
    #[inline]
    pub fn is_data_modification(&self) -> bool {
        matches!(
            self,
            OperationType::Insert | OperationType::Update | OperationType::Replace
        )
    }

    /// Returns true if this operation removes data (delete, drop, drop database).
    #[inline]
    pub fn is_data_removal(&self) -> bool {
        matches!(
            self,
            OperationType::Delete | OperationType::Drop | OperationType::DropDatabase
        )
    }

    /// Returns true if this operation is a DDL operation (drop, rename, drop database).
    #[inline]
    pub fn is_ddl(&self) -> bool {
        matches!(
            self,
            OperationType::Drop | OperationType::DropDatabase | OperationType::Rename
        )
    }

    /// Returns true if this is an unknown operation type.
    ///
    /// Unknown operation types may appear when using a newer MongoDB version
    /// than this library was designed for.
    #[inline]
    pub fn is_unknown(&self) -> bool {
        matches!(self, OperationType::Unknown(_))
    }
}

/// MongoDB namespace (database + collection).
///
/// Identifies the specific collection where an operation occurred.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Namespace {
    /// Database name
    pub database: String,

    /// Collection name
    pub collection: String,
}

impl Namespace {
    /// Creates a new namespace from database and collection names.
    pub fn new(database: impl Into<String>, collection: impl Into<String>) -> Self {
        Self {
            database: database.into(),
            collection: collection.into(),
        }
    }

    /// Returns the fully qualified namespace as "database.collection".
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.database, self.collection)
    }
}

/// Update description for partial document updates.
///
/// When a document is updated (not replaced), this describes what changed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateDescription {
    /// Fields that were added or modified
    #[serde(rename = "updatedFields")]
    pub updated_fields: Document,

    /// Fields that were removed from the document
    #[serde(rename = "removedFields")]
    pub removed_fields: Vec<String>,

    /// Array modifications (if any)
    #[serde(rename = "truncatedArrays", skip_serializing_if = "Option::is_none")]
    pub truncated_arrays: Option<Vec<TruncatedArray>>,
}

/// Describes modifications to an array field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TruncatedArray {
    /// Field path to the array
    pub field: String,

    /// New size of the array after truncation
    #[serde(rename = "newSize")]
    pub new_size: u32,
}

/// A MongoDB change stream event.
///
/// This is the primary type that flows through the Rigatoni pipeline.
/// It represents a single change operation from MongoDB change streams.
///
/// # Memory Layout
///
/// The struct uses owned data for all fields to ensure safe transfer between
/// async tasks and threads. Fields that may not be present use `Option<T>`.
///
/// Approximate memory size: 200-500 bytes depending on document sizes.
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::event::{ChangeEvent, OperationType};
///
/// fn process_event(event: &ChangeEvent) {
///     match event.operation {
///         OperationType::Insert => {
///             println!("New document in {}", event.collection_name());
///             if let Some(doc) = &event.full_document {
///                 println!("Data: {:?}", doc);
///             }
///         }
///         OperationType::Update => {
///             println!("Document updated in {}", event.collection_name());
///             if let Some(desc) = &event.update_description {
///                 let keys: Vec<_> = desc.updated_fields.keys().collect();
///                 println!("Changed fields: {:?}", keys);
///             }
///         }
///         OperationType::Delete => {
///             println!("Document deleted from {}", event.collection_name());
///             println!("Key: {:?}", event.document_key);
///         }
///         _ => println!("Other operation: {:?}", event.operation),
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeEvent {
    /// Type of operation that occurred
    #[serde(rename = "operationType")]
    pub operation: OperationType,

    /// Namespace (database + collection) where the operation occurred
    #[serde(rename = "ns")]
    pub namespace: Namespace,

    /// Document key (_id and shard key if sharded)
    ///
    /// Present for all operations except invalidate.
    /// For invalidate events, this will be None.
    #[serde(rename = "documentKey", skip_serializing_if = "Option::is_none")]
    pub document_key: Option<Document>,

    /// Full document after the operation
    ///
    /// Present for: insert (always), replace (always), update (if configured),
    /// delete (never, unless configured for pre-images).
    #[serde(rename = "fullDocument", skip_serializing_if = "Option::is_none")]
    pub full_document: Option<Document>,

    /// Description of what changed in an update operation
    ///
    /// Present only for update operations.
    #[serde(rename = "updateDescription", skip_serializing_if = "Option::is_none")]
    pub update_description: Option<UpdateDescription>,

    /// Timestamp of the operation in the oplog
    #[serde(rename = "clusterTime")]
    pub cluster_time: DateTime<Utc>,

    /// Resume token for this event
    ///
    /// Can be used to resume the change stream from this point.
    #[serde(rename = "_id")]
    pub resume_token: Document,
}

impl ChangeEvent {
    /// Returns true if this is an insert operation.
    #[inline]
    pub fn is_insert(&self) -> bool {
        self.operation == OperationType::Insert
    }

    /// Returns true if this is an update operation.
    #[inline]
    pub fn is_update(&self) -> bool {
        self.operation == OperationType::Update
    }

    /// Returns true if this is a delete operation.
    #[inline]
    pub fn is_delete(&self) -> bool {
        self.operation == OperationType::Delete
    }

    /// Returns true if this is a replace operation.
    #[inline]
    pub fn is_replace(&self) -> bool {
        self.operation == OperationType::Replace
    }

    /// Returns true if this is an invalidate operation.
    #[inline]
    pub fn is_invalidate(&self) -> bool {
        self.operation == OperationType::Invalidate
    }

    /// Returns the collection name.
    #[inline]
    pub fn collection_name(&self) -> &str {
        &self.namespace.collection
    }

    /// Returns the database name.
    #[inline]
    pub fn database_name(&self) -> &str {
        &self.namespace.database
    }

    /// Returns the fully qualified namespace as "database.collection".
    #[inline]
    pub fn full_namespace(&self) -> String {
        self.namespace.full_name()
    }

    /// Returns the document ID if present in the document key.
    ///
    /// Most MongoDB documents have an `_id` field in the document key.
    /// Returns None if document_key is not present (e.g., invalidate events).
    pub fn document_id(&self) -> Option<&bson::Bson> {
        self.document_key.as_ref()?.get("_id")
    }

    /// Returns true if this event has a full document.
    ///
    /// Useful for checking if the document data is available.
    #[inline]
    pub fn has_full_document(&self) -> bool {
        self.full_document.is_some()
    }

    /// Returns true if this event has update description.
    ///
    /// Only present for update operations.
    #[inline]
    pub fn has_update_description(&self) -> bool {
        self.update_description.is_some()
    }

    /// Returns the size estimate of this event in bytes.
    ///
    /// Useful for batching and memory management.
    pub fn estimated_size_bytes(&self) -> usize {
        let mut size = std::mem::size_of::<Self>();

        // Add document sizes (rough estimate)
        if let Some(doc) = &self.full_document {
            size += estimate_document_size(doc);
        }

        if let Some(update_desc) = &self.update_description {
            size += estimate_document_size(&update_desc.updated_fields);
            size += update_desc
                .removed_fields
                .iter()
                .map(|s| s.len())
                .sum::<usize>();
        }

        if let Some(doc_key) = &self.document_key {
            size += estimate_document_size(doc_key);
        }
        size += estimate_document_size(&self.resume_token);

        size
    }
}

/// Estimates the serialized size of a BSON document in bytes.
fn estimate_document_size(doc: &Document) -> usize {
    // Simple estimation: each key + value pair ~= 50 bytes average
    // This is a rough heuristic; actual size varies widely
    doc.len() * 50
}

/// Conversion from MongoDB driver's ChangeStreamEvent.
///
/// This enables seamless integration with the official MongoDB Rust driver.
/// Returns an error if the resume token cannot be converted to a BSON document.
impl TryFrom<mongodb::change_stream::event::ChangeStreamEvent<Document>> for ChangeEvent {
    type Error = ConversionError;

    fn try_from(
        event: mongodb::change_stream::event::ChangeStreamEvent<Document>,
    ) -> Result<Self, Self::Error> {
        use mongodb::change_stream::event::OperationType as MongoOpType;

        // Convert operation type
        let operation = match event.operation_type {
            MongoOpType::Insert => OperationType::Insert,
            MongoOpType::Update => OperationType::Update,
            MongoOpType::Delete => OperationType::Delete,
            MongoOpType::Replace => OperationType::Replace,
            MongoOpType::Invalidate => OperationType::Invalidate,
            MongoOpType::Drop => OperationType::Drop,
            MongoOpType::DropDatabase => OperationType::DropDatabase,
            MongoOpType::Rename => OperationType::Rename,
            _ => {
                // For any unknown operation types, preserve the original type string
                // This ensures forward compatibility with new MongoDB versions
                let op_str = format!("{:?}", event.operation_type);
                eprintln!(
                    "Warning: Unknown MongoDB operation type encountered: {}. \
                     This may indicate a newer MongoDB version than supported.",
                    op_str
                );
                OperationType::Unknown(op_str)
            }
        };

        // Convert namespace
        let namespace = event
            .ns
            .map(|ns| Namespace {
                database: ns.db,
                collection: ns.coll.unwrap_or_default(),
            })
            .unwrap_or_else(|| Namespace {
                database: String::new(),
                collection: String::new(),
            });

        // Convert update description
        let update_description = event.update_description.map(|ud| UpdateDescription {
            updated_fields: ud.updated_fields,
            removed_fields: ud.removed_fields,
            truncated_arrays: ud.truncated_arrays.map(|arrays| {
                arrays
                    .into_iter()
                    .map(|ta| TruncatedArray {
                        field: ta.field,
                        new_size: ta.new_size as u32,
                    })
                    .collect()
            }),
        });

        // Convert cluster time to chrono DateTime, preserving increment as nanoseconds
        // MongoDB Timestamp has both time (seconds) and increment (counter within that second)
        // We map increment to nanoseconds to preserve ordering of events within the same second
        let cluster_time = event
            .cluster_time
            .map(|ts| {
                let seconds = ts.time as i64;
                // Map increment to nanoseconds for sub-second precision
                // This preserves event ordering within the same second
                let nanos = ts.increment * 1_000_000; // Scale increment to nanosecond range
                DateTime::from_timestamp(seconds, nanos)
                    .unwrap_or_else(|| {
                        // Log error in production - this should never happen with valid MongoDB data
                        eprintln!(
                            "Warning: Invalid MongoDB timestamp (time={}, increment={}), using current time",
                            ts.time, ts.increment
                        );
                        Utc::now()
                    })
            })
            .unwrap_or_else(|| {
                eprintln!("Warning: Missing cluster_time in ChangeStreamEvent, using current time");
                Utc::now()
            });

        // Convert resume token - this is critical for stream resumption
        let resume_token = bson::to_document(&event.id).map_err(|e| {
            ConversionError::ResumeTokenConversion(format!(
                "Failed to serialize resume token to BSON document: {}",
                e
            ))
        })?;

        Ok(Self {
            operation,
            namespace,
            document_key: event.document_key,
            full_document: event.full_document,
            update_description,
            cluster_time,
            resume_token,
        })
    }
}
