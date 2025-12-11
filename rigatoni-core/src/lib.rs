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

//! Rigatoni Core - CDC/Data Replication Framework Core Types and Traits
//!
//! This crate provides the foundational types and traits for the Rigatoni CDC/Data Replication framework.
//! It includes event definitions, pipeline orchestration, and `MongoDB` integration.
//!
//! # Key Components
//!
//! - **Events**: [`event`] module defines `MongoDB` change stream events
//! - **Stream**: [`stream`] module provides `MongoDB` change stream listener with auto-reconnection
//! - **Destination**: [`destination`] module defines the destination trait for writing events
//! - **State**: [`state`] module provides state storage for resume tokens
//! - **Pipeline**: [`pipeline`] module provides the core pipeline orchestration
//! - **Traits**: Source and Transform traits (coming soon)
//!
//! # Example
//!
//! ```rust
//! use rigatoni_core::event::{ChangeEvent, OperationType};
//!
//! fn process_event(event: ChangeEvent) {
//!     match event.operation {
//!         OperationType::Insert => println!("New document inserted"),
//!         OperationType::Update => println!("Document updated"),
//!         OperationType::Delete => println!("Document deleted"),
//!         _ => println!("Other operation"),
//!     }
//! }
//! ```

pub mod destination;
pub mod event;
pub mod metrics;
pub mod pipeline;
pub mod state;
pub mod stream;
pub mod watch_level;
