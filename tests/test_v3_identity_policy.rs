use orion_error::{
    assert_err_identity, assert_err_operation, assert_err_path, v2, DefaultErrorPolicy,
    ErrorCategory, ErrorRenderer, ErrorWith, IntoAs, RenderMode, TextReportRenderer, UvsReason,
};

#[test]
fn test_root_exports_support_v3_identity_and_policy_flow() {
    let err = std::fs::read_to_string("missing-config.toml")
        .into_as(UvsReason::system_error(), "read config failed")
        .doing("read config")
        .unwrap_err();

    assert_err_identity(&err, "sys.io_error", ErrorCategory::Sys);

    let identity = err.identity_snapshot();
    let view = err.policy_report();
    let policy = DefaultErrorPolicy;
    let decision = view.decision(&policy);
    let snapshot = err.policy_snapshot(&policy);
    let http = err.http_response(&policy);
    let cli = err.cli_response(&policy);
    let log = err.log_response(&policy);
    let rpc = err.rpc_response(&policy);
    let rendered = TextReportRenderer::new(RenderMode::Compact).render(view.report());

    assert_eq!(identity.code, "sys.io_error");
    assert_eq!(identity.category, ErrorCategory::Sys);
    assert_err_operation(&err, "read config");
    assert_err_path(&err, "read config");
    assert_eq!(view.identity(), &identity);
    assert_eq!(decision.http_status, 500);
    assert_eq!(decision.visibility, orion_error::Visibility::Internal);
    assert_eq!(
        decision.default_hints,
        vec!["check filesystem state", "verify file permissions"]
    );
    assert!(!decision.retryable);
    assert_eq!(snapshot.identity, identity);
    assert_eq!(snapshot.decision, decision);
    assert_eq!(snapshot.report.reason, "system error");
    assert_eq!(http.status, 500);
    assert_eq!(http.code, "sys.io_error");
    assert_eq!(http.message, "system error");
    assert_eq!(cli.code, "sys.io_error");
    assert_eq!(cli.summary, "system error: read config failed");
    assert_eq!(log.code, "sys.io_error");
    assert_eq!(log.reason, "system error");
    assert_eq!(log.operation.as_deref(), Some("read config"));
    assert_eq!(rpc.code, "sys.io_error");
    assert_eq!(rpc.reason, "system error");
    assert_eq!(rpc.detail, None);
    assert!(!rpc.retryable);
    assert!(rendered.contains("system error"));
    assert!(rendered.contains("read config failed"));
}

#[test]
fn test_v2_namespace_exports_support_v3_identity_and_policy_flow() {
    let err = v2::conversion::ErrorWith::with_context(
        v2::runtime::StructError::from(v2::reason::UvsReason::business_error())
            .with_detail("order state invalid"),
        v2::runtime::OperationContext::doing("validate order"),
    );

    let identity = err.identity_snapshot();
    let view = err.policy_report();
    let decision = view.decision(&v2::report::DefaultErrorPolicy);
    let snapshot = err.policy_snapshot(&v2::report::DefaultErrorPolicy);
    let http = err.http_response(&v2::report::DefaultErrorPolicy);
    let cli = err.cli_response(&v2::report::DefaultErrorPolicy);
    let log = err.log_response(&v2::report::DefaultErrorPolicy);
    let rpc = err.rpc_response(&v2::report::DefaultErrorPolicy);
    let renderer = v2::report::TextReportRenderer::new(v2::report::RenderMode::Verbose);
    let rendered = view.render_with(renderer);

    assert_eq!(identity.code, "biz.business_error");
    assert_eq!(identity.category, v2::reason::ErrorCategory::Biz);
    assert_err_operation(&err, "validate order");
    assert_err_path(&err, "validate order");
    assert_eq!(view.identity(), &identity);
    assert_eq!(decision.http_status, 400);
    assert_eq!(decision.visibility, v2::report::Visibility::Public);
    assert!(decision.default_hints.is_empty());
    assert!(!decision.retryable);
    assert_eq!(snapshot.identity, identity);
    assert_eq!(snapshot.decision, decision);
    assert_eq!(snapshot.report.reason, "business logic error");
    assert_eq!(http.status, 400);
    assert_eq!(http.code, "biz.business_error");
    assert_eq!(http.message, "order state invalid");
    assert_eq!(cli.code, "biz.business_error");
    assert_eq!(cli.summary, "business logic error: order state invalid");
    assert_eq!(log.code, "biz.business_error");
    assert_eq!(log.reason, "business logic error");
    assert_eq!(log.operation.as_deref(), Some("validate order"));
    assert_eq!(rpc.code, "biz.business_error");
    assert_eq!(rpc.detail.as_deref(), Some("order state invalid"));
    assert!(!rpc.retryable);
    assert!(rendered.contains("reason: business logic error"));
    assert!(rendered.contains("detail: order state invalid"));
    assert!(rendered.contains("validate order"));
}
