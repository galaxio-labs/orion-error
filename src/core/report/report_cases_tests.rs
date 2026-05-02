#[test]
fn test_report_contains_root_and_source_data() {
    let source = StructError::from(TestReason::TestError).with_context(
        OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
    );
    let err = StructError::from(TestReason::General(UnifiedReason::system_error()))
        .with_context(OperationContext::doing("start engine").with_meta("component.name", "engine"))
        .with_struct_source(source);

    let report = err.report();
    let rendered = report.render();

    assert_eq!(report.reason, "system error");
    assert_eq!(report.reason(), "system error");
    assert!(rendered.contains("reason: system error"));
    assert!(rendered.contains("context:"));
    assert!(rendered.contains("start engine"));
}

#[test]
fn test_struct_error_into_report_matches_borrowed_report() {
    let source = StructError::from(TestReason::TestError).with_context(
        OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
    );
    let err = StructError::from(TestReason::General(UnifiedReason::system_error()))
        .with_context(OperationContext::doing("start engine").with_meta("component.name", "engine"))
        .with_struct_source(source);

    let via_borrowed = err.report();
    let via_owned = err.into_report();

    assert_eq!(via_owned, via_borrowed);
}

#[test]
fn test_report_from_struct_error_matches_report_methods() {
    let source = StructError::from(TestReason::TestError).with_context(
        OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
    );
    let err = StructError::from(TestReason::General(UnifiedReason::system_error()))
        .with_context(OperationContext::doing("start engine").with_meta("component.name", "engine"))
        .with_struct_source(source);

    let via_method = err.report();
    let via_borrowed = DiagnosticReport::from(&err);
    let via_owned = DiagnosticReport::from(err);

    assert_eq!(via_borrowed, via_method);
    assert_eq!(via_owned, via_method);
}

#[test]
fn test_report_verbose_render_includes_metadata() {
    let report = DiagnosticReport::from_parts(
        "test error".to_string(),
        Some("failed".to_string()),
        None,
        std::sync::Arc::new(vec![OperationContext::doing("load")]),
    );

    let rendered = report.render();

    assert!(rendered.contains("reason: test error"));
    assert!(rendered.contains("detail: failed"));
    assert!(rendered.contains("context:"));
}

#[test]
fn test_struct_error_exposure_uses_real_stable_identity() {
    let err = StructError::from(TestReason::General(UnifiedReason::system_error()))
        .with_detail("engine bootstrap failed")
        .with_context(OperationContext::doing("start engine"));

    let snapshot = err.exposure(&DefaultExposurePolicy);

    assert_eq!(snapshot.identity.code, "sys.io_error");
    assert_eq!(snapshot.identity.category, ErrorCategory::Sys);
    assert_eq!(snapshot.decision.http_status, 500);
    assert_eq!(
        snapshot.decision.default_hints,
        vec!["check filesystem state", "verify file permissions"]
    );
    assert_eq!(
        snapshot.decision,
        ExposureDecision {
            http_status: 500,
            visibility: Visibility::Internal,
            default_hints: vec!["check filesystem state", "verify file permissions"],
            retryable: false,
        }
    );
    assert_eq!(snapshot.identity.reason, "system error");
    assert!(snapshot.render().contains("reason: system error"));
}

#[test]
fn test_report_decision_uses_exposure_identity_fallback() {
    let report = DiagnosticReport::from_parts(
        "configuration error".to_string(),
        Some("invalid config".to_string()),
        None,
        std::sync::Arc::new(vec![]),
    );

    let identity = ErrorIdentity {
        code: "test.error".to_string(),
        category: ErrorCategory::Sys,
        reason: "configuration error".to_string(),
        detail: None,
        position: None,
        path: None,
    };

    let snapshot =
        ErrorProtocolSnapshot::from_report_skeleton(report, identity, &DefaultExposurePolicy);

    assert_eq!(
        snapshot.decision,
        ExposureDecision {
            http_status: 500,
            visibility: Visibility::Internal,
            default_hints: vec![],
            retryable: false,
        }
    );
}
