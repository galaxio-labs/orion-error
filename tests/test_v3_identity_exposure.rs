use orion_error::dev::testing::{assert_err_identity, assert_err_operation, assert_err_path};
use orion_error::prelude::*;
use orion_error::protocol::DefaultExposurePolicy;
use orion_error::protocol::Visibility as ReportVisibility;
use orion_error::reason::ErrorCategory;
use orion_error::{OperationContext, StructError, UnifiedReason};

#[test]
fn test_snapshot_exposure_flow_for_system_error() {
    let err = std::fs::read_to_string("missing-config.toml")
        .source_err(UnifiedReason::system_error(), "read config failed")
        .doing("read config")
        .unwrap_err();

    assert_err_identity(&err, "sys.io_error", ErrorCategory::Sys);

    let identity = err.identity_snapshot();
    let snapshot = err.exposure(&DefaultExposurePolicy);
    let rendered = snapshot.render_user_debug();

    assert_eq!(identity.code, "sys.io_error");
    assert_eq!(identity.category, ErrorCategory::Sys);
    assert_err_operation(&err, "read config");
    assert_err_path(&err, "read config");
    assert_eq!(snapshot.identity, identity);
    assert_eq!(snapshot.decision.http_status, 500);
    assert_eq!(snapshot.decision.visibility, ReportVisibility::Internal);
    assert_eq!(
        snapshot.decision.default_hints,
        vec!["check filesystem state", "verify file permissions"]
    );
    assert!(!snapshot.decision.retryable);
    assert_eq!(snapshot.identity.reason, "system error");
    assert!(snapshot.render().contains("reason: system error"));
    assert!(rendered.contains("sys.io_error"));
    assert!(rendered.contains("read config failed"));
}

#[test]
fn test_snapshot_exposure_flow_for_business_error() {
    let err = ErrorWith::with_context(
        StructError::from(UnifiedReason::business_error()).with_detail("order state invalid"),
        OperationContext::doing("validate order"),
    );

    let identity = err.identity_snapshot();
    let snapshot = err.exposure(&DefaultExposurePolicy);
    let rendered = err.report().render();

    assert_eq!(identity.code, "biz.business_error");
    assert_eq!(identity.category, ErrorCategory::Biz);
    assert_err_operation(&err, "validate order");
    assert_err_path(&err, "validate order");
    assert_eq!(snapshot.identity, identity);
    assert_eq!(snapshot.decision.http_status, 400);
    assert_eq!(snapshot.decision.visibility, ReportVisibility::Public);
    assert!(snapshot.decision.default_hints.is_empty());
    assert!(!snapshot.decision.retryable);
    assert_eq!(snapshot.identity.reason, "business logic error");
    assert!(snapshot.render().contains("reason: business logic error"));
    assert!(rendered.contains("reason: business logic error"));
    assert!(rendered.contains("detail: order state invalid"));
    assert!(rendered.contains("validate order"));
}

#[cfg(feature = "serde_json")]
#[test]
fn test_exposure_json_projection_for_business_error() {
    let err = StructError::from(UnifiedReason::business_error())
        .with_detail("order state invalid")
        .with_context(OperationContext::doing("validate order"));

    let http = err
        .exposure(&DefaultExposurePolicy)
        .to_http_error_json()
        .unwrap();
    let cli = err
        .exposure(&DefaultExposurePolicy)
        .to_cli_error_json()
        .unwrap();

    assert_eq!(http["status"], serde_json::json!(400));
    assert_eq!(http["code"], serde_json::json!("biz.business_error"));
    assert_eq!(http["message"], serde_json::json!("order state invalid"));
    assert_eq!(cli["code"], serde_json::json!("biz.business_error"));
    assert_eq!(
        cli["summary"],
        serde_json::json!("business logic error: order state invalid")
    );
}
