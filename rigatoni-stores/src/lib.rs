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

//! State store implementations for Rigatoni ETL framework.
//!
//! This crate provides various backend implementations of the
//! [`StateStore`](rigatoni_core::state::StateStore) trait for persisting
//! `MongoDB` change stream resume tokens.
//!
//! # Available Stores
//!
//! - **Redis** (`redis-store` feature): Distributed state storage with Redis
//!
//! # Feature Flags
//!
//! - `redis-store`: Enables Redis-backed state store (requires Redis server)
//! - `memory`: In-memory state store (coming soon)
//! - `file`: File-based state store (coming soon)
//!
//! # Example: Redis Store
//!
//! ```rust,ignore
//! use rigatoni_stores::redis::{RedisStore, RedisConfig};
//! use rigatoni_core::state::StateStore;
//! use mongodb::bson::doc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Configure Redis connection
//! let config = RedisConfig::builder()
//!     .url("redis://localhost:6379")
//!     .pool_size(10)
//!     .build()?;
//!
//! // Create store
//! let store = RedisStore::new(config).await?;
//!
//! // Save resume token
//! let token = doc! { "_data": "token123" };
//! store.save_resume_token("users", &token).await?;
//!
//! // Retrieve token
//! let retrieved = store.get_resume_token("users").await?;
//! assert!(retrieved.is_some());
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

#[cfg(feature = "redis-store")]
pub mod redis;
