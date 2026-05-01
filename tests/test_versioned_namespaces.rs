use orion_error::reason::ErrorCode;
use orion_error::{conversion, reason, runtime, snapshot};
use orion_error::prelude::*;
use orion_error::UvsReason;

#[test]
fn test_layered_modules_and_root_prelude_compile() {
    let err = conversion::ErrorWith::with_context(
        runtime::StructError::from(reason::UvsReason::system_error())
            .with_detail("bootstrap failed"),
        runtime::OperationContext::doing("start engine"),
    );

    let snapshot = err.snapshot();
    let stable = snapshot.stable_export();
    let report = stable.report();
    let bridge = err.as_std();

    assert_eq!(reason::ErrorCode::error_code(err.reason()), 201);
    assert_eq!(snapshot.reason, "system error");
    assert_eq!(
        stable.schema_version(),
        snapshot::STABLE_SNAPSHOT_SCHEMA_VERSION
    );
    assert_eq!(report.reason(), "system error");
    assert!(std::error::Error::source(&bridge).is_none());

    let io_result: Result<(), std::io::Error> = Err(std::io::Error::other("disk offline"));
    let structured = conversion::IntoAs::into_as(
        io_result,
        reason::UvsReason::system_error(),
        "load config failed",
    )
    .unwrap_err();
    assert_eq!(structured.source_ref().unwrap().to_string(), "disk offline");

    fn build_with_root_prelude() -> Result<(), runtime::StructError<UvsReason>> {
        use orion_error::prelude::*;
        use orion_error::reason::UvsReason;
        use orion_error::runtime::OperationContext;

        let mut ctx = OperationContext::doing("load config");
        ctx.record_field("path", "config.toml");

        std::fs::read_to_string("missing-config.toml")
            .into_as(UvsReason::system_error(), "read config failed")
            .doing("read config")
            .with_context(&ctx)
            .map(|_| ())
    }

    let err = build_with_root_prelude().unwrap_err();
    assert_eq!(err.reason().error_code(), UvsReason::system_error().error_code());
    assert_eq!(err.action_main().as_deref(), Some("load config"));
    assert_eq!(err.locator_main(), None);
    assert_eq!(err.contexts()[0].action().as_deref(), Some("read config"));
    assert_eq!(err.contexts()[1].action().as_deref(), Some("load config"));

    let at_err = std::fs::read_to_string("missing-config.toml")
        .into_as(UvsReason::system_error(), "read config failed")
        .at("missing-config.toml")
        .unwrap_err();
    assert_eq!(
        at_err.locator_main().as_deref(),
        Some("missing-config.toml")
    );
}

#[cfg(feature = "serde_json")]
#[test]
fn test_dev_prelude_exports_cli_projection_types() {
    use orion_error::protocol::DefaultExposurePolicy;
    use orion_error::{StructError, UvsReason};

    let http = StructError::from(UvsReason::system_error())
        .exposure_snapshot(&DefaultExposurePolicy)
        .to_http_error_json()
        .unwrap();
    let cli = StructError::from(UvsReason::system_error())
        .exposure_snapshot(&DefaultExposurePolicy)
        .to_cli_error_json()
        .unwrap();

    assert_eq!(http["code"], serde_json::json!("sys.io_error"));
    assert_eq!(http["status"], serde_json::json!(500));
    assert_eq!(cli["code"], serde_json::json!("sys.io_error"));
    assert!(StructError::from(UvsReason::system_error())
        .exposure_snapshot(&DefaultExposurePolicy)
        .render()
        .contains("reason: system error"));
}

#[test]
fn test_root_prelude_imports_compile() {
    fn build_with_prelude() -> Result<(), orion_error::StructError<UvsReason>> {
        use orion_error::prelude::*;
        use orion_error::reason::UvsReason;
        use orion_error::runtime::OperationContext;

        let mut ctx = OperationContext::doing("load config");
        ctx.record_field("path", "config.toml");

        std::fs::read_to_string("missing-config.toml")
            .into_as(UvsReason::system_error(), "read config failed")
            .doing("read config")
            .with_context(&ctx)
            .map(|_| ())
    }

    let err = build_with_prelude().unwrap_err();
    assert_eq!(err.reason().error_code(), UvsReason::system_error().error_code());
}
