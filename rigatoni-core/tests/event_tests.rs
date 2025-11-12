//! Integration tests for the event module.
//!
//! These tests verify the complete functionality of ChangeEvent and related types.

use rigatoni_core::event::{
    ChangeEvent, Namespace, OperationType, TruncatedArray, UpdateDescription,
};
use bson::{doc, Bson};
use chrono::Utc;

#[test]
fn test_operation_type_serialization() {
    let op = OperationType::Insert;
    let json = serde_json::to_string(&op).unwrap();
    assert_eq!(json, r#""insert""#);

    let deserialized: OperationType = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, OperationType::Insert);
}

#[test]
fn test_operation_type_predicates() {
    assert!(OperationType::Insert.is_data_modification());
    assert!(OperationType::Update.is_data_modification());
    assert!(OperationType::Replace.is_data_modification());

    assert!(OperationType::Delete.is_data_removal());
    assert!(OperationType::Drop.is_data_removal());
    assert!(OperationType::DropDatabase.is_data_removal());

    assert!(OperationType::Drop.is_ddl());
    assert!(OperationType::Rename.is_ddl());
    assert!(OperationType::DropDatabase.is_ddl());
}

#[test]
fn test_namespace_creation() {
    let ns = Namespace::new("testdb", "users");
    assert_eq!(ns.database, "testdb");
    assert_eq!(ns.collection, "users");
    assert_eq!(ns.full_name(), "testdb.users");
}

#[test]
fn test_change_event_insert() {
    let event = ChangeEvent {
        operation: OperationType::Insert,
        namespace: Namespace::new("testdb", "users"),
        document_key: Some(doc! { "_id": 123 }),
        full_document: Some(doc! {
            "_id": 123,
            "name": "Alice",
            "email": "alice@example.com"
        }),
        update_description: None,
        cluster_time: Utc::now(),
        resume_token: doc! { "_data": "token123" },
    };

    assert!(event.is_insert());
    assert!(!event.is_update());
    assert!(!event.is_delete());
    assert_eq!(event.collection_name(), "users");
    assert_eq!(event.database_name(), "testdb");
    assert_eq!(event.full_namespace(), "testdb.users");
    assert!(event.has_full_document());
    assert!(!event.has_update_description());

    let doc_id = event.document_id().unwrap();
    assert_eq!(doc_id, &Bson::Int32(123));
}

#[test]
fn test_change_event_update() {
    let event = ChangeEvent {
        operation: OperationType::Update,
        namespace: Namespace::new("testdb", "users"),
        document_key: Some(doc! { "_id": 123 }),
        full_document: None,
        update_description: Some(UpdateDescription {
            updated_fields: doc! { "email": "newemail@example.com" },
            removed_fields: vec!["oldField".to_string()],
            truncated_arrays: None,
        }),
        cluster_time: Utc::now(),
        resume_token: doc! { "_data": "token456" },
    };

    assert!(!event.is_insert());
    assert!(event.is_update());
    assert!(!event.is_delete());
    assert!(!event.has_full_document());
    assert!(event.has_update_description());

    let update_desc = event.update_description.as_ref().unwrap();
    assert_eq!(
        update_desc.updated_fields.get("email").unwrap(),
        &Bson::String("newemail@example.com".to_string())
    );
    assert_eq!(update_desc.removed_fields.len(), 1);
}

#[test]
fn test_change_event_delete() {
    let event = ChangeEvent {
        operation: OperationType::Delete,
        namespace: Namespace::new("testdb", "users"),
        document_key: Some(doc! { "_id": 123 }),
        full_document: None,
        update_description: None,
        cluster_time: Utc::now(),
        resume_token: doc! { "_data": "token789" },
    };

    assert!(!event.is_insert());
    assert!(!event.is_update());
    assert!(event.is_delete());
    assert!(!event.has_full_document());
}

#[test]
fn test_change_event_serialization_roundtrip() {
    let original = ChangeEvent {
        operation: OperationType::Insert,
        namespace: Namespace::new("testdb", "users"),
        document_key: Some(doc! { "_id": 123 }),
        full_document: Some(doc! {
            "_id": 123,
            "name": "Bob",
            "age": 30
        }),
        update_description: None,
        cluster_time: Utc::now(),
        resume_token: doc! { "_data": "token123" },
    };

    // Serialize to JSON
    let json = serde_json::to_string(&original).unwrap();

    // Deserialize back
    let deserialized: ChangeEvent = serde_json::from_str(&json).unwrap();

    // Verify equality
    assert_eq!(original.operation, deserialized.operation);
    assert_eq!(original.namespace, deserialized.namespace);
    assert_eq!(original.document_key, deserialized.document_key);
    assert_eq!(original.full_document, deserialized.full_document);
}

#[test]
fn test_change_event_with_truncated_arrays() {
    let event = ChangeEvent {
        operation: OperationType::Update,
        namespace: Namespace::new("testdb", "items"),
        document_key: Some(doc! { "_id": 456 }),
        full_document: None,
        update_description: Some(UpdateDescription {
            updated_fields: doc! {},
            removed_fields: vec![],
            truncated_arrays: Some(vec![TruncatedArray {
                field: "tags".to_string(),
                new_size: 5,
            }]),
        }),
        cluster_time: Utc::now(),
        resume_token: doc! { "_data": "token999" },
    };

    assert!(event.is_update());
    let update_desc = event.update_description.as_ref().unwrap();
    let arrays = update_desc.truncated_arrays.as_ref().unwrap();
    assert_eq!(arrays.len(), 1);
    assert_eq!(arrays[0].field, "tags");
    assert_eq!(arrays[0].new_size, 5);
}

#[test]
fn test_estimated_size_bytes() {
    let event = ChangeEvent {
        operation: OperationType::Insert,
        namespace: Namespace::new("testdb", "users"),
        document_key: Some(doc! { "_id": 123 }),
        full_document: Some(doc! {
            "_id": 123,
            "name": "Alice",
            "email": "alice@example.com",
            "metadata": doc! {
                "created_at": "2024-01-01",
                "tags": ["user", "active"]
            }
        }),
        update_description: None,
        cluster_time: Utc::now(),
        resume_token: doc! { "_data": "token123" },
    };

    let size = event.estimated_size_bytes();

    // Should be greater than the base struct size
    assert!(size > std::mem::size_of::<ChangeEvent>());

    // Should account for the full document
    assert!(size > 100); // At least 100 bytes with the document
}

#[test]
fn test_bson_serialization_roundtrip() {
    let original = ChangeEvent {
        operation: OperationType::Replace,
        namespace: Namespace::new("testdb", "products"),
        document_key: Some(doc! { "_id": "prod123" }),
        full_document: Some(doc! {
            "_id": "prod123",
            "name": "Widget",
            "price": 29.99
        }),
        update_description: None,
        cluster_time: Utc::now(),
        resume_token: doc! { "_data": "tokenABC" },
    };

    // Serialize to BSON
    let bson_doc = bson::to_document(&original).unwrap();

    // Deserialize back
    let deserialized: ChangeEvent = bson::from_document(bson_doc).unwrap();

    // Verify equality
    assert_eq!(original.operation, deserialized.operation);
    assert_eq!(original.namespace, deserialized.namespace);
    assert!(original.is_replace());
    assert!(deserialized.is_replace());
}

#[test]
fn test_change_event_invalidate_without_document_key() {
    let event = ChangeEvent {
        operation: OperationType::Invalidate,
        namespace: Namespace::new("testdb", "users"),
        document_key: None, // Invalidate events don't have document keys
        full_document: None,
        update_description: None,
        cluster_time: Utc::now(),
        resume_token: doc! { "_data": "tokenInvalidate" },
    };

    assert!(event.is_invalidate());
    assert_eq!(event.document_id(), None);
    assert_eq!(event.document_key, None);

    // Test serialization roundtrip
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: ChangeEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.document_key, None);
}

#[test]
fn test_operation_type_unknown_variant() {
    let unknown_op = OperationType::Unknown("futureOperation".to_string());

    assert!(unknown_op.is_unknown());
    assert!(!unknown_op.is_data_modification());
    assert!(!unknown_op.is_data_removal());
    assert!(!unknown_op.is_ddl());

    // Test that it can be serialized/deserialized
    let json = serde_json::to_string(&unknown_op).unwrap();
    let deserialized: OperationType = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, unknown_op);
}

#[test]
fn test_operation_type_all_predicates() {
    // Test is_data_modification
    assert!(OperationType::Insert.is_data_modification());
    assert!(OperationType::Update.is_data_modification());
    assert!(OperationType::Replace.is_data_modification());
    assert!(!OperationType::Delete.is_data_modification());

    // Test is_data_removal
    assert!(OperationType::Delete.is_data_removal());
    assert!(OperationType::Drop.is_data_removal());
    assert!(OperationType::DropDatabase.is_data_removal());
    assert!(!OperationType::Insert.is_data_removal());

    // Test is_ddl
    assert!(OperationType::Drop.is_ddl());
    assert!(OperationType::DropDatabase.is_ddl());
    assert!(OperationType::Rename.is_ddl());
    assert!(!OperationType::Insert.is_ddl());
    assert!(!OperationType::Update.is_ddl());

    // Test is_unknown
    assert!(OperationType::Unknown("test".to_string()).is_unknown());
    assert!(!OperationType::Insert.is_unknown());
}
