//! Golden tests for stable protocol output shapes.
//!
//! These tests create a known error and assert the full JSON output against
//! expected values. A change in output shape or field content means either
//! the golden value needs updating or the change broke the contract.

#![cfg(all(test, feature = "serde_json", feature = "serde"))]

use orion_error::{
    prelude::*,
    protocol::DefaultExposurePolicy,
    reason::UnifiedReason,
    runtime::{OperationContext, StructError},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_err() -> StructError<UnifiedReason> {
    StructError::from(UnifiedReason::validation_error())
        .with_detail("field `email` is required")
        .with_position("src/handler.rs:42")
        .doing("parse input")
        .at("request.json")
}

// ---------------------------------------------------------------------------
// Golden tests
// ---------------------------------------------------------------------------

#[test]
fn golden_http_error_json_for_public_error() {
    let err = StructError::from(UnifiedReason::business_error())
        .with_detail("order state invalid")
        .doing("validate order");
    let json = err
        .exposure(&DefaultExposurePolicy)
        .to_http_error_json()
        .unwrap();

    assert_eq!(json["status"], 400);
    assert_eq!(json["code"], "biz.business_error");
    assert_eq!(json["category"], "biz");
    assert_eq!(json["message"], "order state invalid");
    assert_eq!(json["visibility"], "public");
    assert_eq!(json["hints"], serde_json::json!([]));
}

#[test]
fn golden_http_error_json_for_internal_error() {
    let err = StructError::from(UnifiedReason::system_error())
        .with_detail("disk offline")
        .doing("write file");
    let json = err
        .exposure(&DefaultExposurePolicy)
        .to_http_error_json()
        .unwrap();

    assert_eq!(json["status"], 500);
    assert_eq!(json["code"], "sys.io_error");
    assert_eq!(json["category"], "sys");
    assert_eq!(json["message"], "system error"); // internal: uses reason, not detail
    assert_eq!(json["visibility"], "internal");
    assert_eq!(
        json["hints"],
        serde_json::json!(["check filesystem state", "verify file permissions"])
    );
}

#[test]
fn golden_http_error_json_keys() {
    let err = make_err();
    let json = err
        .exposure(&DefaultExposurePolicy)
        .to_http_error_json()
        .unwrap();
    let keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();

    assert!(keys.contains(&"status"));
    assert!(keys.contains(&"code"));
    assert!(keys.contains(&"category"));
    assert!(keys.contains(&"message"));
    assert!(keys.contains(&"visibility"));
    assert!(keys.contains(&"hints"));
    assert_eq!(keys.len(), 6);
}

#[test]
fn golden_rpc_error_json_for_timeout() {
    let err =
        StructError::from(UnifiedReason::timeout_error()).with_detail("downstream rpc timeout");
    let json = err
        .exposure(&DefaultExposurePolicy)
        .to_rpc_error_json()
        .unwrap();

    assert_eq!(json["status"], 500);
    assert_eq!(json["code"], "sys.timeout");
    assert_eq!(json["retryable"], true);
    assert_eq!(json["detail"], serde_json::Value::Null); // internal: hides detail
    assert_eq!(
        json["hints"],
        serde_json::json!(["retry later", "inspect downstream service latency"])
    );
}

#[test]
fn golden_rpc_error_json_for_business() {
    let err = StructError::from(UnifiedReason::business_error()).with_detail("order state invalid");
    let json = err
        .exposure(&DefaultExposurePolicy)
        .to_rpc_error_json()
        .unwrap();

    assert_eq!(json["status"], 400);
    assert_eq!(json["code"], "biz.business_error");
    assert_eq!(json["detail"], "order state invalid"); // public: shows detail
    assert_eq!(json["retryable"], false);
}

#[test]
fn golden_redacted_protocol_masks_detail() {
    let err = StructError::from(UnifiedReason::validation_error()).with_detail("token=abc");
    let policy = TestRedactPolicy;
    let redacted = err
        .exposure(&DefaultExposurePolicy)
        .redacted(&policy)
        .render_user_debug_redacted(&policy);

    assert!(redacted.contains("<redacted>"));
    assert!(!redacted.contains("token=abc"));
}

#[test]
fn golden_source_frame_metadata() {
    let inner = StructError::from(UnifiedReason::validation_error()).with_context(
        OperationContext::doing("parse")
            .with_meta("parse.line", 42u32)
            .with_meta("parse.file", "config.toml"),
    );
    let err = StructError::from(UnifiedReason::system_error()).with_source(inner);

    let json = err
        .exposure(&DefaultExposurePolicy)
        .to_log_error_json()
        .unwrap();

    // Source frame carries metadata
    let frame = &json["source_frames"][0];
    assert_eq!(frame["message"], "validation error");
    assert_eq!(frame["metadata"]["parse.line"], 42);
    assert_eq!(frame["metadata"]["parse.file"], "config.toml");
}

// ---------------------------------------------------------------------------
// Test utilities
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct TestRedactPolicy;

impl orion_error::report::RedactPolicy for TestRedactPolicy {
    fn redact_value(&self, _key: Option<&str>, _value: &str) -> Option<String> {
        Some("<redacted>".to_string())
    }
}
