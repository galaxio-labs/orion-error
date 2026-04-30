    #[test]
    fn test_exposure_snapshot_render_debug_summary_prefers_detail_path_context_and_component() {
        let snapshot = test_proto(
            DiagnosticReport::from_parts(
                "invalid order".to_string(),
                Some("order text must not be empty".to_string()),
                None,
                std::sync::Arc::new(vec![{
                    let mut ctx = OperationContext::doing("place_order");
                    ctx.record_field("user_id", "42");
                    ctx.record_field("order.raw", "");
                    ctx.record_meta("component.name", "order_service");
                    ctx
                }]),
            ),
            ReportProjectionParts {
                path: Some("place_order / parse order".to_string()),
                root_metadata: {
                    let mut metadata = ErrorMetadata::new();
                    metadata.insert("component.name", "order_service");
                    metadata.insert("trace.secret", "prod-token");
                    metadata
                },
                source_frames: vec![],
            },
            test_identity(
                "biz.order_invalid",
                ErrorCategory::Biz,
                "invalid order",
                Some("order text must not be empty"),
                Some("place_order / parse order"),
            ),
            ExposureDecision {
                http_status: 400,
                visibility: Visibility::Public,
                default_hints: vec![],
                retryable: false,
            },
        );

        let rendered = snapshot.render_user_debug();

        assert!(rendered.contains("code          : biz.order_invalid (biz)"));
        assert!(rendered.contains("detail        : order text must not be empty"));
        assert!(rendered.contains("http          : 400 public retryable=false"));
        assert!(rendered.contains("path          : place_order / parse order"));
        assert!(rendered.contains("context       : user_id=\"42\", order.raw=\"\""));
        assert!(rendered.contains("component     : order_service"));
        assert!(!rendered.contains("trace.secret"));
    }

    #[test]
    fn test_exposure_snapshot_render_debug_summary_falls_back_to_reason_and_source() {
        let snapshot = test_proto(
            DiagnosticReport::from_parts(
                "storage full".to_string(),
                None,
                None,
                std::sync::Arc::new(vec![]),
            ),
            ReportProjectionParts {
                path: Some("place_order / save order".to_string()),
                root_metadata: ErrorMetadata::new(),
                source_frames: vec![SourceFrame {
                    index: 0,
                    message: "storage full".to_string(),
                    display: None,
                    debug: None,
                    type_name: None,
                    error_code: None,
                    reason: None,
                    path: None,
                    detail: None,
                    metadata: ErrorMetadata::new(),
                    is_root_cause: true,
                }],
            },
            test_identity(
                "sys.storage_full",
                ErrorCategory::Sys,
                "storage full",
                None,
                Some("place_order / save order"),
            ),
            ExposureDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec![],
                retryable: false,
            },
        );

        let rendered = snapshot.render_user_debug();

        assert!(rendered.contains("detail        : storage full"));
        assert!(rendered.contains("source        : storage full"));
    }

    #[test]
    fn test_exposure_snapshot_render_debug_summary_prefers_root_cause_source_frame() {
        let snapshot = test_proto(
            DiagnosticReport::from_parts(
                "system error".to_string(),
                Some("save order failed".to_string()),
                None,
                std::sync::Arc::new(vec![]),
            ),
            ReportProjectionParts {
                path: Some("place_order / save order".to_string()),
                root_metadata: ErrorMetadata::new(),
                source_frames: vec![
                    SourceFrame {
                        index: 0,
                        message: "storage layer failed".to_string(),
                        display: None,
                        debug: None,
                        type_name: None,
                        error_code: None,
                        reason: None,
                        path: None,
                        detail: None,
                        metadata: ErrorMetadata::new(),
                        is_root_cause: false,
                    },
                    SourceFrame {
                        index: 1,
                        message: "disk offline".to_string(),
                        display: None,
                        debug: None,
                        type_name: None,
                        error_code: None,
                        reason: None,
                        path: None,
                        detail: None,
                        metadata: ErrorMetadata::new(),
                        is_root_cause: true,
                    },
                ],
            },
            test_identity(
                "sys.io_error",
                ErrorCategory::Sys,
                "system error",
                Some("save order failed"),
                Some("place_order / save order"),
            ),
            ExposureDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec![],
                retryable: false,
            },
        );

        let rendered = snapshot.render_user_debug();

        assert!(rendered.contains("source        : disk offline"));
        assert!(!rendered.contains("source        : storage layer failed"));
    }
