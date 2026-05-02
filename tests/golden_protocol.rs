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
    snapshot::StableErrorSnapshot,
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

fn make_deep_err() -> StructError<UnifiedReason> {
    let leaf = StructError::from(UnifiedReason::system_error())
        .with_detail("disk offline")
        .with_position("src/storage.rs:88");
    let mid = StructError::from(UnifiedReason::data_error())
        .with_detail("query failed")
        .with_source(leaf)
        .doing("load user");
    StructError::from(UnifiedReason::validation_error())
        .with_detail("invalid request")
        .with_source(mid)
        .doing("handle request")
        .at("POST /users")
        .with_context(
            OperationContext::doing("auth check")
                .with_meta("tenant", "acme")
                .with_meta("component.name", "auth"),
        )
}

// ---------------------------------------------------------------------------
// Golden tests
// ---------------------------------------------------------------------------

#[test]
fn golden_stable_snapshot_shallow() {
    let err = make_err();
    let stable: StableErrorSnapshot = err.snapshot().stable_export();

    let value = serde_json::to_value(&stable).unwrap();
    let obj = value.as_object().unwrap();

    // Schema version is stable
    assert_eq!(obj["schema_version"], "orion-error.snapshot.v3");
    assert_eq!(obj["reason"], "validation error");
    assert_eq!(obj["detail"], "field `email` is required");
    assert_eq!(obj["position"], "src/handler.rs:42");
    assert_eq!(obj["path"], "parse input / request.json");
    assert_eq!(obj["context"][0]["action"], "parse input");
    assert_eq!(obj["context"][1]["locator"], "request.json");

    // Stable export omits compat projection fields (fields, result, display, type_name)
    for ctx in obj["context"].as_array().unwrap() {
        assert!(
            ctx.get("fields").is_none(),
            "stable context must not contain fields"
        );
        assert!(
            ctx.get("result").is_none(),
            "stable context must not contain result"
        );
    }
}

#[test]
fn golden_stable_snapshot_deep_source() {
    let err = make_deep_err();
    let stable = err.snapshot().stable_export();
    let value = serde_json::to_value(&stable).unwrap();

    // Root fields
    assert_eq!(value["reason"], "validation error");
    assert_eq!(value["path"], "auth check / handle request / POST /users");
    assert_eq!(value["context"].as_array().unwrap().len(), 3);

    // Source frames = underlying source chain (top error reason is in snapshot.reason)
    let frames = value["source_frames"].as_array().unwrap();
    assert_eq!(frames.len(), 2, "data_error -> system_error");
    assert_eq!(frames[0]["message"], "data error");
    assert_eq!(frames[0]["is_root_cause"], false);

    // Root cause = deepest source
    let last = frames.last().unwrap();
    assert_eq!(last["message"], "system error");
    assert!(last["is_root_cause"] == true);
}

#[test]
fn golden_http_error_json_for_public_error() {
    let err = StructError::from(UnifiedReason::business_error())
        .with_detail("order state invalid")
        .doing("validate order");
    let json = err
        .exposure_snapshot(&DefaultExposurePolicy)
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
        .exposure_snapshot(&DefaultExposurePolicy)
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
        .exposure_snapshot(&DefaultExposurePolicy)
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
        .exposure_snapshot(&DefaultExposurePolicy)
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
        .exposure_snapshot(&DefaultExposurePolicy)
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
        .exposure_snapshot(&DefaultExposurePolicy)
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
    let err = StructError::from(UnifiedReason::system_error()).with_struct_source(inner);

    let json = err
        .exposure_snapshot(&DefaultExposurePolicy)
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
