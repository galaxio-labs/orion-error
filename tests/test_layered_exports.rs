use orion_error::{bridge, compat_prelude, conversion, reason, report, runtime, snapshot};

#[allow(deprecated)]
#[test]
fn test_runtime_snapshot_report_bridge_and_compat_exports_compile_and_interoperate() {
    let err = conversion::ErrorWith::attach_context(
        runtime::StructError::from(reason::UvsReason::system_error())
            .with_detail("engine bootstrap failed"),
        runtime::OperationContext::doing("start engine"),
    );

    let snapshot_value: snapshot::StructErrorSnapshot = err.snapshot();
    let stable: snapshot::StableStructErrorSnapshot = snapshot_value.stable_export();
    let report_value: report::ErrorReport = stable.report();
    let bridge_view: bridge::StdStructRef<'_, reason::UvsReason> = err.as_std();
    let owned_bridge: bridge::OwnedStdStructError<reason::UvsReason> = err.clone().into_std();
    let _compat_runtime = err.compat_serialize();

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
        serde_json::to_value(_compat_runtime).unwrap()["reason"],
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
