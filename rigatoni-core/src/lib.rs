//! Rigatoni Core - ETL Framework Core Types and Traits
//!
//! This crate provides the foundational types and traits for the Rigatoni ETL framework.
//! It includes event definitions, pipeline orchestration, and MongoDB integration.
//!
//! # Key Components
//!
//! - **Events**: [`event`] module defines MongoDB change stream events
//! - **Pipeline**: Core pipeline orchestration (coming soon)
//! - **Traits**: Source, Transform, and Destination traits (coming soon)
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

pub mod event;
