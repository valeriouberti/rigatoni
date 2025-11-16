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

//! S3 destination for streaming data from MongoDB to AWS S3.
//!
//! This module provides a production-ready S3 destination that implements the
//! `Destination` trait. It supports multiple serialization formats, compression
//! options, and key generation strategies.
//!
//! # Features
//!
//! - **Multiple formats**: JSON, Parquet, CSV, Avro (with feature gates)
//! - **Compression**: gzip, zstd (with feature gates)
//! - **Flexible partitioning**: Hive-style, date-based, collection-based
//! - **Retry logic**: Automatic retry with exponential backoff
//! - **S3-compatible storage**: Works with AWS S3, MinIO, LocalStack
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use rigatoni_core::Destination;
//! use rigatoni_destinations::s3::{S3Destination, S3Config};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = S3Config::builder()
//!         .bucket("my-data-lake")
//!         .region("us-east-1")
//!         .prefix("mongodb/events")
//!         .build()?;
//!
//!     let destination = S3Destination::new(config).await?;
//!
//!     // Use with change stream listener
//!     // listener.pipe_to(destination).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Examples
//!
//! ## With compression and Hive partitioning
//!
//! ```rust,ignore
//! use rigatoni_destinations::s3::{
//!     S3Config, S3Destination, Compression, KeyGenerationStrategy,
//! };
//!
//! let config = S3Config::builder()
//!     .bucket("analytics-data")
//!     .region("us-west-2")
//!     .prefix("events")
//!     .compression(Compression::Zstd)
//!     .key_strategy(KeyGenerationStrategy::HivePartitioned)
//!     .build()?;
//!
//! let destination = S3Destination::new(config).await?;
//! ```
//!
//! ## Using LocalStack for testing
//!
//! ```rust,ignore
//! let config = S3Config::builder()
//!     .bucket("test-bucket")
//!     .region("us-east-1")
//!     .endpoint_url("http://localhost:4566")
//!     .force_path_style(true)
//!     .build()?;
//!
//! let destination = S3Destination::new(config).await?;
//! ```

pub mod config;
mod destination;
pub mod key_gen;

pub use config::{Compression, S3Config, S3ConfigBuilder, S3ConfigError, SerializationFormat};
pub use destination::S3Destination;
pub use key_gen::KeyGenerationStrategy;
