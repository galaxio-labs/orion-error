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

#[cfg(feature = "serde_json")]
#[test]
fn test_advanced_prelude_types_and_report_exports_include_cli_projection() {
    use orion_error::{DefaultExposurePolicy, StructError, UvsReason};

    let snapshot = StructError::from(UvsReason::business_error())
        .with_detail("order state invalid")
        .exposure_snapshot(&DefaultExposurePolicy);

    let http = snapshot.to_http_error_json().unwrap();
    let cli = snapshot.to_cli_error_json().unwrap();
    let rpc = snapshot.to_rpc_error_json().unwrap();

    assert_eq!(http["code"], serde_json::json!("biz.business_error"));
    assert_eq!(http["status"], serde_json::json!(400));
    assert_eq!(cli["code"], serde_json::json!("biz.business_error"));
    assert_eq!(
        cli["summary"],
        serde_json::json!("business logic error: order state invalid")
    );
    assert_eq!(rpc["status"], serde_json::json!(400));
    assert_eq!(rpc["code"], serde_json::json!("biz.business_error"));
    assert_eq!(rpc["detail"], serde_json::json!("order state invalid"));
}
