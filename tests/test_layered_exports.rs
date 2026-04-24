use orion_error::ErrorWith;
use orion_error::{bridge, compat_prelude, conversion, reason, report, runtime, snapshot};

#[test]
fn test_runtime_snapshot_report_bridge_and_legacy_exports_compile_and_interoperate() {
    let err = conversion::ErrorWith::with_context(
        runtime::StructError::from(reason::UvsReason::system_error())
            .with_detail("engine bootstrap failed"),
        runtime::OperationContext::doing("start engine"),
    );

    let snapshot_value: snapshot::ErrorSnapshot = err.snapshot();
    let stable: snapshot::StableErrorSnapshot = snapshot_value.stable_export();
    let report_value: report::DiagnosticReport = stable.report();
    let bridge_view: bridge::StdStructRef<'_, reason::UvsReason> = err.as_std();
    let owned_bridge: bridge::OwnedStdStructError<reason::UvsReason> = err.clone().into_std();

    assert_eq!(
        reason::ErrorCode::error_code(&err),
        reason::ErrorCode::error_code(&reason::UvsReason::system_error())
    );
    assert_eq!(snapshot_value.reason, "system error");
    assert_eq!(
        stable.schema_version,
        snapshot::STABLE_SNAPSHOT_SCHEMA_VERSION
    );
    assert_eq!(report_value.reason, "system error");
    #[cfg(feature = "serde")]
    assert_eq!(
        serde_json::to_value(&err).unwrap()["reason"],
        serde_json::json!("SystemError")
    );
    assert!(std::error::Error::source(&bridge_view).is_none());
    assert_eq!(owned_bridge.into_struct(), err);

    let legacy: Result<(), &str> = Err("legacy failure");
    let compat_result: Result<(), runtime::StructError<reason::UvsReason>> =
        compat_prelude::ErrorOweBase::owe(legacy, reason::UvsReason::business_error());

    assert_eq!(
        reason::ErrorCode::error_code(&compat_result.unwrap_err()),
        reason::ErrorCode::error_code(&reason::UvsReason::business_error())
    );

    let io_result: Result<(), std::io::Error> = Err(std::io::Error::other("disk offline"));
    let structured = conversion::IntoAs::into_as(
        io_result,
        reason::UvsReason::system_error(),
        "load config failed",
    )
    .unwrap_err();
    assert_eq!(structured.source_ref().unwrap().to_string(), "disk offline");
}

#[test]
fn test_testcase_module_exports_assert_helpers() {
    let err = runtime::StructError::from(reason::UvsReason::business_error())
        .with_detail("order state invalid")
        .doing("validate order");

    orion_error::testcase::assert_err_identity(
        &err,
        "biz.business_error",
        reason::ErrorCategory::Biz,
    );
    orion_error::testcase::assert_err_operation(&err, "validate order");
    orion_error::testcase::assert_err_path(&err, "validate order");
}

#[test]
fn test_advanced_prelude_types_and_report_exports_include_cli_projection() {
    let cli = orion_error::report::ErrorCliResponse {
        code: "biz.business_error".to_string(),
        category: orion_error::reason::ErrorCategory::Biz,
        summary: "business logic error".to_string(),
        detail: "order state invalid".to_string(),
        visibility: orion_error::report::Visibility::Public,
        hints: vec!["fix order state".to_string()],
    };
    let log = orion_error::report::ErrorLogResponse {
        code: "biz.business_error".to_string(),
        category: orion_error::reason::ErrorCategory::Biz,
        reason: "business logic error".to_string(),
        detail: Some("order state invalid".to_string()),
        operation: Some("validate order".to_string()),
        path: Some("validate order".to_string()),
        visibility: orion_error::report::Visibility::Public,
        hints: vec!["fix order state".to_string()],
        root_metadata: orion_error::runtime::ErrorMetadata::new(),
        context: vec![],
        source_frames: vec![],
    };
    let rpc = orion_error::report::ErrorRpcResponse {
        status: 400,
        code: "biz.business_error".to_string(),
        category: orion_error::reason::ErrorCategory::Biz,
        reason: "business logic error".to_string(),
        detail: Some("order state invalid".to_string()),
        visibility: orion_error::report::Visibility::Public,
        hints: vec!["fix order state".to_string()],
        retryable: false,
    };

    assert_eq!(cli.code, "biz.business_error");
    assert_eq!(cli.summary, "business logic error");
    assert_eq!(log.reason, "business logic error");
    assert_eq!(rpc.status, 400);
}
