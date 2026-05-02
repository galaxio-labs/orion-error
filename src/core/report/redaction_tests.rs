    #[test]
    fn test_render_user_debug_redacted_masks_sensitive_fields() {
        let err = StructError::from(TestReason::TestError)
            .with_detail("token=abc")
            .with_context({
                let mut ctx = OperationContext::doing("load");
                ctx.record_field("token", "abc");
                ctx.record_meta("component.name", "order_service");
                ctx.record_meta("config.secret", "abc");
                ctx
            });

        let rendered = err.exposure_snapshot(&DefaultExposurePolicy).render_user_debug_redacted(&TestPolicy);

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("token=abc"));
        assert!(!rendered.contains("token=\"abc\""));
        assert!(!rendered.contains("config.secret"));
    }

    #[test]
    fn test_report_redaction_masks_sensitive_fields() {
        let err = StructError::from(TestReason::TestError)
            .with_detail("token=abc")
            .with_std_source(std::io::Error::other("token=abc"))
            .with_context(OperationContext::doing("load").with_meta("config.secret", "abc"));

        let rendered = err.render_redacted(&TestPolicy);
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("abc"));
    }

    #[test]
    fn test_report_redaction_masks_source_frame_message() {
        let err = StructError::from(TestReason::TestError)
            .with_std_source(std::io::Error::other("https://svc.local?token=abc"));

        let rendered = err.render_redacted(&TestPolicy);

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("svc.local"));
        assert!(!rendered.contains("token=abc"));
    }

    #[test]
    fn test_report_redaction_masks_source_frame_display() {
        let snapshot = test_proto(
            DiagnosticReport::from_parts(
                "test error".to_string(),
                None,
                None,
                std::sync::Arc::new(vec![]),
            ),
            ReportProjectionParts {
                path: None,
                root_metadata: ErrorMetadata::new(),
                source_frames: vec![SourceFrame {
                    index: 0,
                    message: "inner".into(),
                    display: Some("inner token=abc".into()),
                    debug: Some("debug".into()),
                    type_name: None,
                    error_code: None,
                    reason: None,
                    path: None,
                    detail: None,
                    metadata: ErrorMetadata::new(),
                    is_root_cause: true,
            context_fields: Vec::new(),
                }],
            },
            test_identity("test.error", ErrorCategory::Logic, "test error", None, None),
            ExposureDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec![],
                retryable: false,
            },
        );

        let redacted = snapshot.redacted(&TestPolicy);
        assert_eq!(
            redacted.projection.source_frames[0].display.as_deref(),
            Some("<redacted>")
        );
        assert!(!redacted.projection.source_frames[0]
            .display
            .as_deref()
            .unwrap()
            .contains("token=abc"));
    }

    #[test]
    fn test_report_redaction_masks_source_frame_debug() {
        let snapshot = test_proto(
            DiagnosticReport::from_parts(
                "test error".to_string(),
                None,
                None,
                std::sync::Arc::new(vec![]),
            ),
            ReportProjectionParts {
                path: None,
                root_metadata: ErrorMetadata::new(),
                source_frames: vec![SourceFrame {
                    index: 0,
                    message: "inner".into(),
                    display: None,
                    debug: Some("debug token=abc".into()),
                    type_name: None,
                    error_code: None,
                    reason: None,
                    path: None,
                    detail: None,
                    metadata: ErrorMetadata::new(),
                    is_root_cause: true,
            context_fields: Vec::new(),
                }],
            },
            test_identity("test.error", ErrorCategory::Logic, "test error", None, None),
            ExposureDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec![],
                retryable: false,
            },
        );

        let redacted = snapshot.redacted(&TestPolicy);
        assert_eq!(
            redacted.projection.source_frames[0].debug.as_deref(),
            Some("<redacted>")
        );
        assert!(!redacted.projection.source_frames[0]
            .debug
            .as_deref()
            .unwrap_or("")
            .contains("token=abc"));
    }

    #[test]
    fn test_report_redaction_masks_root_and_frame_paths() {
        let report = DiagnosticReport::from_parts(
            "test error".to_string(),
            None,
            Some("/srv/app/config.toml:10".to_string()),
            std::sync::Arc::new(vec![OperationContext::at("/srv/app/config.toml")]),
        );

        #[derive(Debug)]
        struct PathPolicy;

        impl RedactPolicy for PathPolicy {
            fn redact_value(&self, key: Option<&str>, value: &str) -> Option<String> {
                match key {
                    Some("position") | Some("want") | Some("path") | Some("locator") => {
                        Some(value.replace("/srv/app/config.toml", "<path-redacted>"))
                    }
                    _ => Some(value.to_string()),
                }
            }
        }

        let rendered = report.render_redacted(&PathPolicy);
        assert!(rendered.contains("<path-redacted>"));
        assert!(!rendered.contains("/srv/app/config.toml"));
    }

    #[test]
    fn test_report_redaction_masks_reason_fields() {
        let report = DiagnosticReport::from_parts(
            "tenant secret error".to_string(),
            None,
            None,
            std::sync::Arc::new(vec![]),
        );

        #[derive(Debug)]
        struct ReasonPolicy;

        impl RedactPolicy for ReasonPolicy {
            fn redact_value(&self, key: Option<&str>, value: &str) -> Option<String> {
                match key {
                    Some("reason") | Some("source.reason") => {
                        Some(value.replace("secret", "<redacted>"))
                    }
                    _ => Some(value.to_string()),
                }
            }
        }

        let redacted = report.redacted(&ReasonPolicy);
        assert_eq!(redacted.reason(), "tenant <redacted> error");
    }

    #[test]
    fn test_report_redaction_applies_value_hook_without_redact_key() {
        #[derive(Debug)]
        struct ValueOnlyPolicy;

        impl RedactPolicy for ValueOnlyPolicy {
            fn redact_value(&self, key: Option<&str>, value: &str) -> Option<String> {
                match key {
                    Some("detail") => Some("<detail-redacted>".to_string()),
                    Some("token") => Some("<token-redacted>".to_string()),
                    Some("config.secret") => Some("<secret-redacted>".to_string()),
                    _ => Some(value.to_string()),
                }
            }
        }

        let err = StructError::from(TestReason::TestError)
            .with_detail("token=abc")
            .with_context({
                let mut ctx = OperationContext::doing("load");
                ctx.record("token", "abc");
                ctx.record_meta("config.secret", "abc");
                ctx
            });

        let rendered = err.render_redacted(&ValueOnlyPolicy);
        assert!(rendered.contains("<detail-redacted>"));
        assert!(rendered.contains("<token-redacted>"));
        assert!(!rendered.contains("token=abc"));
        assert!(!rendered.contains("token: abc"));
        assert!(!rendered.contains("config.secret"));
    }
