use orion_error::{v2, ErrorCode, ErrorWith, IntoAs, UvsReason};

#[test]
fn test_v2_namespace_layered_modules_and_prelude_compile() {
    let err = v2::conversion::ErrorWith::with_context(
        v2::runtime::StructError::from(v2::reason::UvsReason::system_error())
            .with_detail("bootstrap failed"),
        v2::runtime::OperationContext::doing("start engine"),
    );

    let snapshot = err.snapshot();
    let stable = snapshot.stable_export();
    let report = stable.report();
    let bridge = err.as_std();
    let cli_fields = v2::report::CLI_ERROR_RESPONSE_FIELDS;
    let http_fields = v2::report::HTTP_ERROR_RESPONSE_FIELDS;

    assert_eq!(v2::reason::ErrorCode::error_code(&err), 201);
    assert_eq!(snapshot.reason, "system error");
    assert_eq!(
        stable.schema_version,
        v2::snapshot::STABLE_SNAPSHOT_SCHEMA_VERSION
    );
    assert_eq!(report.reason, "system error");
    assert_eq!(cli_fields, orion_error::CLI_ERROR_RESPONSE_FIELDS);
    assert_eq!(http_fields, orion_error::HTTP_ERROR_RESPONSE_FIELDS);
    assert!(std::error::Error::source(&bridge).is_none());

    let io_result: Result<(), std::io::Error> = Err(std::io::Error::other("disk offline"));
    let structured = v2::conversion::IntoAs::into_as(
        io_result,
        v2::reason::UvsReason::system_error(),
        "load config failed",
    )
    .unwrap_err();
    assert_eq!(structured.source_ref().unwrap().to_string(), "disk offline");

    fn build_with_v2_prelude() -> Result<(), v2::runtime::StructError<UvsReason>> {
        use orion_error::v2::prelude::*;

        let mut ctx = OperationContext::doing("load config");
        ctx.record("path", "config.toml");

        std::fs::read_to_string("missing-config.toml")
            .into_as(UvsReason::system_error(), "read config failed")
            .doing("read config")
            .with_context(&ctx)
            .map(|_| ())
    }

    let err = build_with_v2_prelude().unwrap_err();
    assert_eq!(err.error_code(), UvsReason::system_error().error_code());
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

#[test]
fn test_v2_prelude_exports_cli_projection_types_and_constants() {
    use orion_error::v2::prelude::*;

    let cli = ErrorCliResponse {
        code: "sys.io_error".to_string(),
        category: ErrorCategory::Sys,
        summary: "system error".to_string(),
        detail: "disk offline".to_string(),
        visibility: Visibility::Internal,
        hints: vec!["check filesystem state".to_string()],
    };
    let log = ErrorLogResponse {
        code: "sys.io_error".to_string(),
        category: ErrorCategory::Sys,
        reason: "system error".to_string(),
        detail: Some("disk offline".to_string()),
        operation: Some("load config".to_string()),
        path: Some("load config".to_string()),
        visibility: Visibility::Internal,
        hints: vec!["check filesystem state".to_string()],
        root_metadata: orion_error::ErrorMetadata::new(),
        context: vec![],
        source_frames: vec![],
    };
    let rpc = ErrorRpcResponse {
        status: 500,
        code: "sys.io_error".to_string(),
        category: ErrorCategory::Sys,
        reason: "system error".to_string(),
        detail: None,
        visibility: Visibility::Internal,
        hints: vec!["check filesystem state".to_string()],
        retryable: false,
    };

    assert_eq!(
        CLI_ERROR_RESPONSE_FIELDS,
        orion_error::CLI_ERROR_RESPONSE_FIELDS
    );
    assert_eq!(
        HTTP_ERROR_RESPONSE_FIELDS,
        orion_error::HTTP_ERROR_RESPONSE_FIELDS
    );
    assert_eq!(
        LOG_ERROR_RESPONSE_FIELDS,
        orion_error::LOG_ERROR_RESPONSE_FIELDS
    );
    assert_eq!(
        RPC_ERROR_RESPONSE_FIELDS,
        orion_error::RPC_ERROR_RESPONSE_FIELDS
    );
    assert_eq!(cli.code, "sys.io_error");
    assert_eq!(cli.visibility, Visibility::Internal);
    assert_eq!(log.reason, "system error");
    assert_eq!(rpc.status, 500);
}

#[allow(deprecated)]
#[test]
fn test_v1_namespace_exposes_primary_and_compat_imports() {
    fn build_with_v1_prelude() -> Result<(), orion_error::StructError<UvsReason>> {
        use orion_error::v1::prelude::*;

        let mut ctx = OperationContext::doing("load config");
        ctx.record("path", "config.toml");

        std::fs::read_to_string("missing-config.toml")
            .into_as(UvsReason::system_error(), "read config failed")
            .doing("read config")
            .with_context(&ctx)
            .map(|_| ())
    }

    fn build_with_v1_compat() -> Result<(), orion_error::StructError<UvsReason>> {
        // Compat coverage is intentional here: V1 explicit namespace must keep
        // exposing the legacy helper group until that surface is removed.
        use orion_error::v1::compat_prelude::*;

        let legacy: Result<(), &str> = Err("legacy failure");
        legacy.owe(UvsReason::business_error())
    }

    let err = build_with_v1_prelude().unwrap_err();
    assert_eq!(err.error_code(), UvsReason::system_error().error_code());

    let err = build_with_v1_compat().unwrap_err();
    assert_eq!(err.error_code(), UvsReason::business_error().error_code());
}
