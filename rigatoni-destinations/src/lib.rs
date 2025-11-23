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

//! Rigatoni Destinations - ETL Destination Implementations
//!
//! This crate provides production-ready destination implementations for the Rigatoni ETL framework.
//! Destinations are the final stage in the ETL pipeline where processed events are written
//! to external systems.
//!
//! # Available Destinations
//!
//! - **S3**: AWS S3 and S3-compatible storage (MinIO, LocalStack)
//!
//! # Features
//!
//! Destinations and formats are enabled via Cargo features:
//!
//! - `s3` - AWS S3 destination (default)
//! - `json` - JSON serialization support (default)
//! - `csv` - CSV serialization support (default)
//! - `parquet` - Apache Parquet serialization support
//! - `avro` - Apache Avro serialization support
//! - `gzip` - Gzip compression support
//! - `zstandard` - Zstandard compression support
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use rigatoni_destinations::s3::{S3Destination, S3Config};
//! use rigatoni_core::Destination;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = S3Config::builder()
//!         .bucket("my-data-lake")
//!         .region("us-east-1")
//!         .prefix("mongodb/events")
//!         .build()?;
//!
//!     let mut destination = S3Destination::new(config).await?;
//!
//!     // Use with change stream listener
//!     // listener.pipe_to(destination).await?;
//!
//!     destination.close().await?;
//!     Ok(())
//! }
//! ```

// S3 destination module (enabled with "s3" feature)
#[cfg(feature = "s3")]
pub mod s3;
