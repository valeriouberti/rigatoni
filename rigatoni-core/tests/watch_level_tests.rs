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

//! Tests for the WatchLevel enum and its functionality.

use rigatoni_core::watch_level::WatchLevel;

#[test]
fn test_watch_level_default() {
    assert_eq!(WatchLevel::default(), WatchLevel::Database);
}

#[test]
fn test_watch_level_collection() {
    let level = WatchLevel::Collection(vec!["users".to_string(), "orders".to_string()]);
    assert!(level.is_collection());
    assert!(!level.is_database());
    assert!(!level.is_deployment());
    assert_eq!(
        level.collections(),
        Some(&vec!["users".to_string(), "orders".to_string()])
    );
}

#[test]
fn test_watch_level_database() {
    let level = WatchLevel::Database;
    assert!(!level.is_collection());
    assert!(level.is_database());
    assert!(!level.is_deployment());
    assert_eq!(level.collections(), None);
}

#[test]
fn test_watch_level_deployment() {
    let level = WatchLevel::Deployment;
    assert!(!level.is_collection());
    assert!(!level.is_database());
    assert!(level.is_deployment());
    assert_eq!(level.collections(), None);
}

#[test]
fn test_watch_level_description() {
    let level = WatchLevel::Collection(vec![]);
    assert_eq!(level.description(), "no collections");

    let level = WatchLevel::Collection(vec!["users".to_string()]);
    assert_eq!(level.description(), "1 collection (users)");

    let level = WatchLevel::Collection(vec!["users".to_string(), "orders".to_string()]);
    assert_eq!(level.description(), "2 collections");

    let level = WatchLevel::Database;
    assert_eq!(level.description(), "database");

    let level = WatchLevel::Deployment;
    assert_eq!(level.description(), "deployment");
}

#[test]
fn test_resume_token_key() {
    let level = WatchLevel::Collection(vec!["users".to_string()]);
    assert_eq!(
        level.resume_token_key("mydb", Some("users")),
        "resume_token:mydb:users"
    );

    let level = WatchLevel::Database;
    assert_eq!(
        level.resume_token_key("mydb", None),
        "resume_token:database:mydb"
    );

    let level = WatchLevel::Deployment;
    assert_eq!(
        level.resume_token_key("mydb", None),
        "resume_token:deployment"
    );
}

#[test]
fn test_watch_level_display() {
    let level = WatchLevel::Collection(vec!["users".to_string()]);
    assert_eq!(format!("{}", level), "Collection([\"users\"])");

    let level = WatchLevel::Database;
    assert_eq!(format!("{}", level), "Database");

    let level = WatchLevel::Deployment;
    assert_eq!(format!("{}", level), "Deployment");
}

#[test]
fn test_watch_level_equality() {
    assert_eq!(WatchLevel::Database, WatchLevel::Database);
    assert_eq!(WatchLevel::Deployment, WatchLevel::Deployment);
    assert_eq!(
        WatchLevel::Collection(vec!["a".to_string()]),
        WatchLevel::Collection(vec!["a".to_string()])
    );
    assert_ne!(WatchLevel::Database, WatchLevel::Deployment);
    assert_ne!(
        WatchLevel::Collection(vec!["a".to_string()]),
        WatchLevel::Collection(vec!["b".to_string()])
    );
}

#[test]
fn test_watch_level_clone() {
    let level = WatchLevel::Collection(vec!["users".to_string()]);
    let cloned = level.clone();
    assert_eq!(level, cloned);
}

#[test]
fn test_watch_level_empty_collection() {
    let level = WatchLevel::Collection(vec![]);
    assert!(level.is_collection());
    assert_eq!(level.collections(), Some(&vec![]));
    assert_eq!(level.description(), "no collections");
}

#[test]
fn test_resume_token_key_collection_without_name() {
    let level = WatchLevel::Collection(vec!["users".to_string()]);
    // When collection name is not provided, it falls back to database-only key
    assert_eq!(level.resume_token_key("mydb", None), "resume_token:mydb");
}

#[test]
fn test_watch_level_debug() {
    let level = WatchLevel::Collection(vec!["users".to_string()]);
    let debug_str = format!("{:?}", level);
    assert!(debug_str.contains("Collection"));
    assert!(debug_str.contains("users"));

    let level = WatchLevel::Database;
    assert_eq!(format!("{:?}", level), "Database");

    let level = WatchLevel::Deployment;
    assert_eq!(format!("{:?}", level), "Deployment");
}
