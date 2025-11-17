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

#![cfg(feature = "redis-store")]

use mongodb::bson::doc;
use rigatoni_core::state::StateStore;
use rigatoni_stores::redis::{RedisConfig, RedisStore};
use std::time::Duration;
use testcontainers::{runners::AsyncRunner, ImageExt};
use testcontainers_modules::redis::Redis;

/// Helper to create a Redis store connected to a test container.
async fn create_test_store(url: String) -> RedisStore {
    let config = RedisConfig::builder()
        .url(url)
        .pool_size(5)
        .build()
        .expect("valid config");

    RedisStore::new(config)
        .await
        .expect("failed to create store")
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_save_and_get_resume_token() {
    // Start Redis container
    let redis_container = Redis::default()
        .start()
        .await
        .expect("failed to start Redis container");

    let host_port = redis_container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");

    let url = format!("redis://127.0.0.1:{}", host_port);
    let store = create_test_store(url).await;

    // Create test token
    let token = doc! {
        "_data": "test_token_123",
        "clusterTime": 1234567890_i64,
    };

    // Save token
    store
        .save_resume_token("users", &token)
        .await
        .expect("failed to save token");

    // Retrieve token
    let retrieved = store
        .get_resume_token("users")
        .await
        .expect("failed to get token");

    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), token);

    // Close store
    store.close().await.expect("failed to close store");
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_get_nonexistent_token() {
    let redis_container = Redis::default()
        .start()
        .await
        .expect("failed to start Redis container");

    let host_port = redis_container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");

    let url = format!("redis://127.0.0.1:{}", host_port);
    let store = create_test_store(url).await;

    // Try to get non-existent token
    let result = store
        .get_resume_token("nonexistent")
        .await
        .expect("failed to get token");

    assert!(result.is_none());
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_delete_resume_token() {
    let redis_container = Redis::default()
        .start()
        .await
        .expect("failed to start Redis container");

    let host_port = redis_container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");

    let url = format!("redis://127.0.0.1:{}", host_port);
    let store = create_test_store(url).await;

    let token = doc! { "_data": "to_be_deleted" };

    // Save token
    store
        .save_resume_token("temp", &token)
        .await
        .expect("failed to save token");

    // Verify it exists
    let retrieved = store
        .get_resume_token("temp")
        .await
        .expect("failed to get token");
    assert!(retrieved.is_some());

    // Delete token
    store
        .delete_resume_token("temp")
        .await
        .expect("failed to delete token");

    // Verify it's gone
    let after_delete = store
        .get_resume_token("temp")
        .await
        .expect("failed to get token");
    assert!(after_delete.is_none());
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_list_resume_tokens() {
    let redis_container = Redis::default()
        .start()
        .await
        .expect("failed to start Redis container");

    let host_port = redis_container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");

    let url = format!("redis://127.0.0.1:{}", host_port);
    let store = create_test_store(url).await;

    // Save multiple tokens
    let token1 = doc! { "_data": "token1" };
    let token2 = doc! { "_data": "token2" };
    let token3 = doc! { "_data": "token3" };

    store
        .save_resume_token("users", &token1)
        .await
        .expect("failed to save token1");
    store
        .save_resume_token("orders", &token2)
        .await
        .expect("failed to save token2");
    store
        .save_resume_token("products", &token3)
        .await
        .expect("failed to save token3");

    // List all tokens
    let all_tokens = store
        .list_resume_tokens()
        .await
        .expect("failed to list tokens");

    assert_eq!(all_tokens.len(), 3);
    assert_eq!(all_tokens.get("users"), Some(&token1));
    assert_eq!(all_tokens.get("orders"), Some(&token2));
    assert_eq!(all_tokens.get("products"), Some(&token3));
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_update_existing_token() {
    let redis_container = Redis::default()
        .start()
        .await
        .expect("failed to start Redis container");

    let host_port = redis_container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");

    let url = format!("redis://127.0.0.1:{}", host_port);
    let store = create_test_store(url).await;

    // Save initial token
    let token1 = doc! { "_data": "version1" };
    store
        .save_resume_token("users", &token1)
        .await
        .expect("failed to save token1");

    // Update with new token
    let token2 = doc! { "_data": "version2", "updated": true };
    store
        .save_resume_token("users", &token2)
        .await
        .expect("failed to save token2");

    // Retrieve and verify it's the new token
    let retrieved = store
        .get_resume_token("users")
        .await
        .expect("failed to get token");

    assert_eq!(retrieved.unwrap(), token2);
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_with_ttl() {
    let redis_container = Redis::default()
        .start()
        .await
        .expect("failed to start Redis container");

    let host_port = redis_container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");

    let url = format!("redis://127.0.0.1:{}", host_port);

    // Create store with 2-second TTL
    let config = RedisConfig::builder()
        .url(url)
        .pool_size(5)
        .ttl(Duration::from_secs(2))
        .build()
        .expect("valid config");

    let store = RedisStore::new(config)
        .await
        .expect("failed to create store");

    let token = doc! { "_data": "expires_soon" };

    // Save token
    store
        .save_resume_token("temp", &token)
        .await
        .expect("failed to save token");

    // Verify it exists immediately
    let retrieved = store
        .get_resume_token("temp")
        .await
        .expect("failed to get token");
    assert!(retrieved.is_some());

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Verify it's gone
    let after_ttl = store
        .get_resume_token("temp")
        .await
        .expect("failed to get token");
    assert!(after_ttl.is_none());
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_concurrent_operations() {
    let redis_container = Redis::default()
        .start()
        .await
        .expect("failed to start Redis container");

    let host_port = redis_container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");

    let url = format!("redis://127.0.0.1:{}", host_port);
    let store = create_test_store(url).await;

    // Spawn multiple concurrent writes
    let mut handles = vec![];
    for i in 0..10 {
        let store_clone = store.clone();
        let handle = tokio::spawn(async move {
            let collection = format!("collection_{}", i);
            let token = doc! { "_data": format!("token_{}", i) };
            store_clone
                .save_resume_token(&collection, &token)
                .await
                .expect("failed to save token");
        });
        handles.push(handle);
    }

    // Wait for all writes to complete
    for handle in handles {
        handle.await.expect("task failed");
    }

    // Verify all tokens were saved
    let all_tokens = store
        .list_resume_tokens()
        .await
        .expect("failed to list tokens");

    assert_eq!(all_tokens.len(), 10);
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_large_token() {
    let redis_container = Redis::default()
        .start()
        .await
        .expect("failed to start Redis container");

    let host_port = redis_container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");

    let url = format!("redis://127.0.0.1:{}", host_port);
    let store = create_test_store(url).await;

    // Create a large token with nested documents and arrays
    let token = doc! {
        "_data": "x".repeat(1000), // 1KB string
        "nested": {
            "field1": "value1",
            "field2": "value2",
            "deep": {
                "field3": "value3",
            }
        },
        "array": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    };

    // Save large token
    store
        .save_resume_token("large", &token)
        .await
        .expect("failed to save large token");

    // Retrieve and verify
    let retrieved = store
        .get_resume_token("large")
        .await
        .expect("failed to get token");

    assert_eq!(retrieved.unwrap(), token);
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_connection_pooling() {
    let redis_container = Redis::default()
        .start()
        .await
        .expect("failed to start Redis container");

    let host_port = redis_container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");

    let url = format!("redis://127.0.0.1:{}", host_port);

    // Create store with small pool size
    let config = RedisConfig::builder()
        .url(url)
        .pool_size(2)
        .build()
        .expect("valid config");

    let store = RedisStore::new(config)
        .await
        .expect("failed to create store");

    // Perform many concurrent operations with limited pool
    let mut handles = vec![];
    for i in 0..20 {
        let store_clone = store.clone();
        let handle = tokio::spawn(async move {
            let collection = format!("pool_test_{}", i);
            let token = doc! { "_data": format!("token_{}", i) };

            // Save
            store_clone
                .save_resume_token(&collection, &token)
                .await
                .expect("failed to save");

            // Get
            let retrieved = store_clone
                .get_resume_token(&collection)
                .await
                .expect("failed to get");

            assert_eq!(retrieved.unwrap(), token);
        });
        handles.push(handle);
    }

    // All operations should succeed despite limited pool
    for handle in handles {
        handle.await.expect("task failed");
    }
}
