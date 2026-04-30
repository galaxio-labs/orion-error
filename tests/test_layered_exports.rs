use orion_error::{cli, conversion, interop, protocol, reason, report, runtime, snapshot};
use orion_error::prelude::ErrorWith;

#[test]
fn test_layered_surfaces_compile_and_interoperate() {
    let err = conversion::ErrorWith::with_context(
        runtime::StructError::from(reason::UvsReason::system_error())
            .with_detail("engine bootstrap failed"),
        runtime::OperationContext::doing("start engine"),
    );

    let snapshot_value: snapshot::ErrorSnapshot = err.snapshot();
    let stable: snapshot::StableErrorSnapshot = snapshot_value.stable_export();
    let report_value: report::DiagnosticReport = stable.report();
    let bridge_view: interop::StdStructRef<'_, reason::UvsReason> = err.as_std();
    let owned_bridge: interop::OwnedStdStructError<reason::UvsReason> = err.clone().into_std();

    assert_eq!(
        reason::ErrorCode::error_code(err.reason()),
        reason::ErrorCode::error_code(&reason::UvsReason::system_error())
    );
    assert_eq!(snapshot_value.reason, "system error");
    assert_eq!(
        stable.schema_version(),
        snapshot::STABLE_SNAPSHOT_SCHEMA_VERSION
    );
    assert_eq!(report_value.reason(), "system error");
    #[cfg(feature = "serde")]
    assert_eq!(
        serde_json::to_value(&err).unwrap()["reason"],
        serde_json::json!("SystemError")
    );
    assert!(std::error::Error::source(&bridge_view).is_none());
    assert_eq!(owned_bridge.into_struct(), err);

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
fn test_dev_testing_module_exports_assert_helpers() {
    let err = runtime::StructError::from(reason::UvsReason::business_error())
        .with_detail("order state invalid")
        .doing("validate order");

    orion_error::dev::testing::assert_err_identity(
        &err,
        "biz.business_error",
        reason::ErrorCategory::Biz,
    );
    orion_error::dev::testing::assert_err_operation(&err, "validate order");
    orion_error::dev::testing::assert_err_path(&err, "validate order");
}

#[test]
fn test_root_surface_stays_on_primary_runtime_path() {
    use orion_error::prelude::*;
    use orion_error::{OperationContext, UvsReason};
    use orion_error::protocol::DefaultExposurePolicy;

    let mut ctx = OperationContext::doing("load config");
    ctx.record_field("path", "config.toml");

    let err = std::fs::read_to_string("missing-config.toml")
        .into_as(UvsReason::system_error(), "read config failed")
        .doing("read config")
        .with_context(&ctx)
        .unwrap_err();

    let snapshot = err.snapshot();
    let report = err.report();
    let proto = err.exposure_snapshot(&DefaultExposurePolicy);

    assert_eq!(snapshot.reason, "system error");
    assert_eq!(report.reason(), "system error");
    assert_eq!(proto.identity.code, "sys.io_error");
    assert_eq!(err.action_main().as_deref(), Some("load config"));
    assert_eq!(err.target_path().as_deref(), Some("load config / read config"));
    assert_eq!(err.contexts()[0].action().as_deref(), Some("read config"));
    assert_eq!(err.contexts()[1].action().as_deref(), Some("load config"));
}

#[test]
fn test_root_conversion_traits_now_live_under_prelude_or_conversion() {
    fn build_with_prelude() -> Result<(), runtime::StructError<reason::UvsReason>> {
        use orion_error::prelude::*;
        use orion_error::reason::UvsReason;

        std::fs::read_to_string("missing-config.toml")
            .into_as(UvsReason::system_error(), "read config failed")
            .map(|_| ())
    }

    let _ = build_with_prelude().unwrap_err();
    let _: fn(
        Result<(), std::io::Error>,
        reason::UvsReason,
        &'static str,
    ) -> Result<(), runtime::StructError<reason::UvsReason>> = conversion::IntoAs::into_as;
}

#[test]
fn test_layered_modules_remain_the_official_home_for_non_root_types() {
    let err = runtime::StructError::builder(reason::UvsReason::system_error())
        .detail("engine bootstrap failed")
        .finish();

    let snapshot_value: snapshot::ErrorSnapshot = err.snapshot();
    let report_value: report::DiagnosticReport = snapshot_value.report();
    let std_bridge: interop::OwnedStdStructError<reason::UvsReason> = err.clone().into_std();

    let _: runtime::StructErrorBuilder<reason::UvsReason> =
        runtime::StructError::builder(reason::UvsReason::system_error());
    let _: reason::ErrorCategory = reason::ErrorCategory::Sys;
    let _: protocol::Visibility = protocol::Visibility::Internal;
    let _: snapshot::StableErrorSnapshot = snapshot_value.stable_export();
    let _: interop::OwnedStdStructError<reason::UvsReason> = err.clone().into_std();

    assert_eq!(report_value.reason(), "system error");
    assert_eq!(std_bridge.into_struct(), err);
}

#[test]
fn test_reason_module_is_the_trait_home_for_identity_and_code() {
    fn assert_reason_traits<R>(reason: &R) -> (&'static str, reason::ErrorCategory, i32)
    where
        R: reason::DomainReason + reason::ErrorIdentityProvider + reason::ErrorCode,
    {
        (
            reason::ErrorIdentityProvider::stable_code(reason),
            reason::ErrorIdentityProvider::error_category(reason),
            reason::ErrorCode::error_code(reason),
        )
    }

    let (stable_code, category, error_code) = assert_reason_traits(&reason::UvsReason::system_error());
    assert_eq!(stable_code, "sys.io_error");
    assert_eq!(category, reason::ErrorCategory::Sys);
    assert_eq!(error_code, 201);
}

#[test]
fn test_removed_root_type_aliases_do_not_return() {
    let identity: snapshot::ErrorIdentity =
        runtime::StructError::from(reason::UvsReason::system_error()).identity_snapshot();
    let _: runtime::ErrorMetadata = Default::default();
    let _: runtime::MetadataValue = "demo".into();

    assert_eq!(identity.code, "sys.io_error");
}

#[test]
fn test_runtime_source_observation_surface_lives_under_runtime_source_module() {
    let err = runtime::StructError::from(reason::UvsReason::system_error())
        .with_source(std::io::Error::other("disk offline"));

    let payload = err.source_payload().expect("source payload");
    let _: runtime::source::SourcePayloadRef<'_> = payload;
    let _: runtime::source::SourcePayloadKind = payload.kind();
    let _: &[runtime::source::SourceFrame] = payload.frames();

    assert_eq!(
        err.source_payload_kind(),
        Some(runtime::source::SourcePayloadKind::Std)
    );
}

#[cfg(feature = "serde_json")]
#[test]
fn test_dev_prelude_types_and_report_exports_include_cli_projection() {
    use orion_error::dev::prelude::{ErrorProtocolSnapshot, ExposureDecision, Visibility};
    use orion_error::protocol::DefaultExposurePolicy;
    use orion_error::{StructError, UvsReason};

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

    let _: &ErrorProtocolSnapshot = &snapshot;
    let _: Visibility = Visibility::Internal;
    let _: ExposureDecision = snapshot.decision.clone();
    assert!(snapshot.render().contains("reason: business logic error"));
}

#[test]
fn test_report_and_protocol_modules_have_distinct_roles() {
    let report_value: report::DiagnosticReport =
        runtime::StructError::from(reason::UvsReason::system_error()).report();
    let _: protocol::Visibility = protocol::Visibility::Internal;
    let _: protocol::ExposureDecision = protocol::ExposureDecision {
        http_status: 500,
        visibility: protocol::Visibility::Internal,
        default_hints: vec![],
        retryable: false,
    };

    assert_eq!(report_value.reason(), "system error");
}

#[test]
fn test_cli_module_is_the_public_home_for_print_error() {
    let fn_ptr: fn(&runtime::StructError<reason::UvsReason>) = cli::print_error;
    let err = runtime::StructError::from(reason::UvsReason::system_error())
        .with_detail("disk offline");

    let _ = fn_ptr;
    assert!(err.display_chain().contains("disk offline"));
}

#[test]
fn test_public_surface_grading_stays_split_between_primary_observation_and_secondary_paths() {
    let err = runtime::StructError::from(reason::UvsReason::system_error())
        .with_detail("disk offline")
        .with_source(std::io::Error::other("root cause"));

    // Primary path stays centered on report/snapshot/protocol entry points.
    let _: report::DiagnosticReport = err.report();
    let _: snapshot::ErrorSnapshot = err.snapshot();
    let _: protocol::ErrorProtocolSnapshot =
        err.exposure_snapshot(&protocol::DefaultExposurePolicy);

    // Observation surface remains explicitly readable but separate.
    let _: &[runtime::source::SourceFrame] = err.source_frames();
    let _: Option<runtime::source::SourcePayloadRef<'_>> = err.source_payload();
    let _: Option<runtime::source::SourcePayloadKind> = err.source_payload_kind();

    // Secondary protocol assembly path remains available without becoming root/runtime API.
    let report_value = err.report();
    let identity = err.identity_snapshot();
    let _: protocol::ErrorProtocolSnapshot =
        protocol::ErrorProtocolSnapshot::from_report_skeleton(
            report_value,
            identity,
            &protocol::DefaultExposurePolicy,
        );
}
