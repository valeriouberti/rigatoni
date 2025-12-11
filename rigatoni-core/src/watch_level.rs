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

//! Watch level configuration for MongoDB change streams.
//!
//! This module defines the different scopes at which Rigatoni can watch
//! for changes in MongoDB: collection level, database level, or deployment
//! (cluster) level.
//!
//! # Watch Levels
//!
//! ## Collection Level
//!
//! Watch specific collections only. Use this when you know exactly which
//! collections to monitor and want fine-grained control.
//!
//! ```rust
//! use rigatoni_core::watch_level::WatchLevel;
//!
//! let level = WatchLevel::Collection(vec!["users".to_string(), "orders".to_string()]);
//! ```
//!
//! ## Database Level
//!
//! Watch all collections in a database. New collections are automatically
//! included as they're created. This is the default and recommended mode
//! for most use cases.
//!
//! ```rust
//! use rigatoni_core::watch_level::WatchLevel;
//!
//! let level = WatchLevel::Database;
//! ```
//!
//! ## Deployment Level
//!
//! Watch all databases in the deployment (cluster-wide). Use with caution
//! as this can generate high event volume. Requires MongoDB 4.0+ and
//! appropriate permissions.
//!
//! ```rust
//! use rigatoni_core::watch_level::WatchLevel;
//!
//! let level = WatchLevel::Deployment;
//! ```
//!
//! # Performance Considerations
//!
//! | Level | Streams | Auto-Discovery | Throughput | Recommended For |
//! |-------|---------|----------------|------------|-----------------|
//! | Collection | N (one per collection) | No | Highest (parallel) | Large DBs with known collections |
//! | Database | 1 | Yes | Good | Most use cases (< 50 collections) |
//! | Deployment | 1 | Yes | Variable | Monitoring/audit use cases |

/// Defines the scope of collections to watch for changes.
///
/// This enum controls whether Rigatoni watches specific collections,
/// an entire database, or all databases in a deployment.
///
/// # Examples
///
/// ```rust
/// use rigatoni_core::watch_level::WatchLevel;
///
/// // Watch specific collections
/// let collections = WatchLevel::Collection(vec!["users".to_string(), "orders".to_string()]);
///
/// // Watch entire database (default)
/// let database = WatchLevel::Database;
///
/// // Watch entire deployment (cluster-wide)
/// let deployment = WatchLevel::Deployment;
///
/// // Default is Database level
/// assert_eq!(WatchLevel::default(), WatchLevel::Database);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchLevel {
    /// Watch specific collections only.
    ///
    /// This is the most granular option, allowing you to specify exactly
    /// which collections to monitor. Each collection gets its own change
    /// stream, enabling parallel processing.
    ///
    /// **Advantages**:
    /// - Maximum parallelism (one worker per collection)
    /// - Can apply different batching/retry settings per collection
    /// - Lower latency for specific collections
    ///
    /// **Disadvantages**:
    /// - Must update configuration when adding new collections
    /// - More MongoDB connections
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_core::watch_level::WatchLevel;
    ///
    /// let level = WatchLevel::Collection(vec![
    ///     "users".to_string(),
    ///     "orders".to_string(),
    ///     "products".to_string(),
    /// ]);
    /// ```
    Collection(Vec<String>),

    /// Watch all collections in the database.
    ///
    /// Automatically picks up new collections as they are created.
    /// Uses MongoDB's `db.watch()` API to create a single change stream
    /// for the entire database.
    ///
    /// **Advantages**:
    /// - Automatic discovery of new collections
    /// - Single change stream (simpler architecture)
    /// - No configuration updates needed
    ///
    /// **Disadvantages**:
    /// - Single stream may become bottleneck for high-volume databases
    /// - Cannot apply per-collection settings
    ///
    /// **Requirements**:
    /// - MongoDB replica set
    ///
    /// **Recommended for**: Databases with < 50 collections
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_core::pipeline::PipelineConfig;
    ///
    /// let config = PipelineConfig::builder()
    ///     .mongodb_uri("mongodb://localhost:27017/?replicaSet=rs0")
    ///     .database("mydb")
    ///     .watch_database()  // Use Database watch level
    ///     .build();
    /// ```
    Database,

    /// Watch all databases in the deployment (cluster-wide).
    ///
    /// Uses MongoDB's `client.watch()` API to monitor changes across
    /// all databases in the deployment. This is the most comprehensive
    /// option but also the most resource-intensive.
    ///
    /// **Advantages**:
    /// - Complete visibility into all changes
    /// - Single stream for entire cluster
    /// - Useful for audit logging and compliance
    ///
    /// **Disadvantages**:
    /// - High event volume
    /// - Requires cluster-wide permissions
    /// - Not suitable for multi-tenant environments (unless intended)
    ///
    /// **Requirements**:
    /// - MongoDB 4.0+
    /// - Cluster-wide read permissions
    ///
    /// **Recommended for**: Monitoring, audit logging, compliance use cases
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_core::pipeline::PipelineConfig;
    ///
    /// let config = PipelineConfig::builder()
    ///     .mongodb_uri("mongodb://localhost:27017/?replicaSet=rs0")
    ///     .database("mydb")  // Still needed for state storage keys
    ///     .watch_deployment()  // Watch all databases
    ///     .build();
    /// ```
    Deployment,
}

impl Default for WatchLevel {
    /// Returns the default watch level: [`WatchLevel::Database`].
    ///
    /// Database-level watching is the recommended default as it provides
    /// automatic collection discovery without the complexity of deployment-level
    /// watching.
    fn default() -> Self {
        Self::Database
    }
}

impl WatchLevel {
    /// Returns `true` if this is collection-level watching.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_core::watch_level::WatchLevel;
    ///
    /// let level = WatchLevel::Collection(vec!["users".to_string()]);
    /// assert!(level.is_collection());
    ///
    /// let level = WatchLevel::Database;
    /// assert!(!level.is_collection());
    /// ```
    #[must_use]
    pub fn is_collection(&self) -> bool {
        matches!(self, Self::Collection(_))
    }

    /// Returns `true` if this is database-level watching.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_core::watch_level::WatchLevel;
    ///
    /// let level = WatchLevel::Database;
    /// assert!(level.is_database());
    /// ```
    #[must_use]
    pub fn is_database(&self) -> bool {
        matches!(self, Self::Database)
    }

    /// Returns `true` if this is deployment-level (cluster-wide) watching.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_core::watch_level::WatchLevel;
    ///
    /// let level = WatchLevel::Deployment;
    /// assert!(level.is_deployment());
    /// ```
    #[must_use]
    pub fn is_deployment(&self) -> bool {
        matches!(self, Self::Deployment)
    }

    /// Returns the collections if this is collection-level watching.
    ///
    /// Returns `None` for database or deployment level watching.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_core::watch_level::WatchLevel;
    ///
    /// let level = WatchLevel::Collection(vec!["users".to_string()]);
    /// assert_eq!(level.collections(), Some(&vec!["users".to_string()]));
    ///
    /// let level = WatchLevel::Database;
    /// assert_eq!(level.collections(), None);
    /// ```
    #[must_use]
    pub fn collections(&self) -> Option<&Vec<String>> {
        match self {
            Self::Collection(collections) => Some(collections),
            _ => None,
        }
    }

    /// Returns a human-readable description of the watch level.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_core::watch_level::WatchLevel;
    ///
    /// let level = WatchLevel::Collection(vec!["users".to_string(), "orders".to_string()]);
    /// assert_eq!(level.description(), "2 collections");
    ///
    /// let level = WatchLevel::Database;
    /// assert_eq!(level.description(), "database");
    ///
    /// let level = WatchLevel::Deployment;
    /// assert_eq!(level.description(), "deployment");
    /// ```
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::Collection(collections) => {
                if collections.is_empty() {
                    "no collections".to_string()
                } else if collections.len() == 1 {
                    format!("1 collection ({})", collections[0])
                } else {
                    format!("{} collections", collections.len())
                }
            }
            Self::Database => "database".to_string(),
            Self::Deployment => "deployment".to_string(),
        }
    }

    /// Returns the resume token key prefix for this watch level.
    ///
    /// Different watch levels use different resume token keys to avoid
    /// conflicts when switching between levels.
    ///
    /// # Arguments
    ///
    /// * `database` - The database name (used for collection and database levels)
    /// * `collection` - Optional collection name (used for collection level)
    ///
    /// # Example
    ///
    /// ```rust
    /// use rigatoni_core::watch_level::WatchLevel;
    ///
    /// let level = WatchLevel::Collection(vec!["users".to_string()]);
    /// assert_eq!(
    ///     level.resume_token_key("mydb", Some("users")),
    ///     "resume_token:mydb:users"
    /// );
    ///
    /// let level = WatchLevel::Database;
    /// assert_eq!(
    ///     level.resume_token_key("mydb", None),
    ///     "resume_token:database:mydb"
    /// );
    ///
    /// let level = WatchLevel::Deployment;
    /// assert_eq!(
    ///     level.resume_token_key("mydb", None),
    ///     "resume_token:deployment"
    /// );
    /// ```
    #[must_use]
    pub fn resume_token_key(&self, database: &str, collection: Option<&str>) -> String {
        match self {
            Self::Collection(_) => {
                if let Some(coll) = collection {
                    format!("resume_token:{}:{}", database, coll)
                } else {
                    format!("resume_token:{}", database)
                }
            }
            Self::Database => {
                format!("resume_token:database:{}", database)
            }
            Self::Deployment => "resume_token:deployment".to_string(),
        }
    }
}

impl std::fmt::Display for WatchLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Collection(collections) => {
                if collections.is_empty() {
                    write!(f, "Collection([])")
                } else {
                    write!(f, "Collection({:?})", collections)
                }
            }
            Self::Database => write!(f, "Database"),
            Self::Deployment => write!(f, "Deployment"),
        }
    }
}
