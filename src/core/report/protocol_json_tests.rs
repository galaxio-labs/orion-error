    #[cfg(feature = "serde_json")]
    #[test]
    fn test_default_exposure_policy_maps_category_to_http_status_and_visibility() {
        let exposure_policy = DefaultExposurePolicy;
        let biz_identity = ErrorIdentity {
            code: "biz.validation_error".to_string(),
            category: ErrorCategory::Biz,
            reason: "validation error".to_string(),
            detail: None,
            position: None,
            path: None,
        };
        let sys_identity = ErrorIdentity {
            code: "sys.io_error".to_string(),
            category: ErrorCategory::Sys,
            reason: "system error".to_string(),
            detail: None,
            position: None,
            path: None,
        };

        assert_eq!(exposure_policy.http_status(&biz_identity), 400);
        assert_eq!(exposure_policy.http_status(&sys_identity), 500);
        assert_eq!(
            exposure_policy.visibility(&biz_identity),
            Visibility::Public
        );
        assert_eq!(
            exposure_policy.visibility(&sys_identity),
            Visibility::Internal
        );
        assert_eq!(
            exposure_policy.default_hints(&sys_identity),
            ["check filesystem state", "verify file permissions"]
        );
        assert_eq!(
            exposure_policy.decide(&sys_identity),
            ExposureDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec!["check filesystem state", "verify file permissions"],
                retryable: false,
            }
        );
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_exposure_snapshot_json_contains_identity_decision_and_report_sections() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("engine bootstrap failed")
            .with_context(OperationContext::doing("start engine"));

        let snapshot = err.exposure_snapshot(&DefaultExposurePolicy);

        assert_eq!(snapshot.identity.code, "sys.io_error");
        assert_eq!(snapshot.identity.category, ErrorCategory::Sys);
        assert_eq!(snapshot.decision.http_status, 500);
        assert_eq!(snapshot.decision.visibility, Visibility::Internal);
        assert_eq!(snapshot.identity.reason, "system error");
        assert!(snapshot.render().contains("reason: system error"));
        assert!(snapshot.render().contains("detail: engine bootstrap failed"));
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_exposure_snapshot_contains_identity_decision_and_report() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("engine bootstrap failed");

        let snapshot = err.exposure_snapshot(&DefaultExposurePolicy);

        assert_eq!(snapshot.identity.code, "sys.io_error");
        assert_eq!(snapshot.identity.category, ErrorCategory::Sys);
        assert!(snapshot.decision.http_status > 0);
        assert_eq!(snapshot.identity.reason, "system error");
        assert!(snapshot.render().contains("reason: system error"));
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_http_response_json_for_public_visibility_uses_detail() {
        let err = StructError::from(TestReason::Uvs(UvsReason::business_error()))
            .with_detail("order state invalid");
        let json = err
            .exposure_snapshot(&DefaultExposurePolicy)
            .to_http_error_json()
            .unwrap();

        assert_eq!(json["status"], serde_json::json!(400));
        assert_eq!(json["code"], serde_json::json!("biz.business_error"));
        assert_eq!(json["category"], serde_json::json!("biz"));
        assert_eq!(json["message"], serde_json::json!("order state invalid"));
        assert_eq!(json["visibility"], serde_json::json!("public"));
        assert_eq!(json["hints"], serde_json::json!([]));
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_http_response_json_for_internal_visibility_uses_reason() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("disk offline");
        let json = err
            .exposure_snapshot(&DefaultExposurePolicy)
            .to_http_error_json()
            .unwrap();

        assert_eq!(json["status"], serde_json::json!(500));
        assert_eq!(json["code"], serde_json::json!("sys.io_error"));
        assert_eq!(json["category"], serde_json::json!("sys"));
        assert_eq!(json["message"], serde_json::json!("system error"));
        assert_eq!(json["visibility"], serde_json::json!("internal"));
        assert_eq!(
            json["hints"],
            serde_json::json!(["check filesystem state", "verify file permissions"])
        );
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_http_error_json_keys_match_expected_shape() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("disk offline");

        let json_value = err
            .exposure_snapshot(&DefaultExposurePolicy)
            .to_http_error_json()
            .expect("serialize http error");

        let mut keys = json_value
            .as_object()
            .expect("http error object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();

        let mut expected = [
            "status",
            "code",
            "category",
            "message",
            "visibility",
            "hints",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(keys, expected);
        assert_eq!(json_value["status"], serde_json::json!(500));
        assert_eq!(json_value["code"], serde_json::json!("sys.io_error"));
        assert_eq!(json_value["message"], serde_json::json!("system error"));
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_cli_response_json_contains_summary_detail_and_hints() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("disk offline");
        let json = err
            .exposure_snapshot(&DefaultExposurePolicy)
            .to_cli_error_json()
            .unwrap();

        assert_eq!(json["code"], serde_json::json!("sys.io_error"));
        assert_eq!(json["category"], serde_json::json!("sys"));
        assert_eq!(json["summary"], serde_json::json!("system error: disk offline"));
        assert_eq!(
            json["detail"],
            serde_json::json!("reason: system error\ndetail: disk offline")
        );
        assert_eq!(json["visibility"], serde_json::json!("internal"));
        assert_eq!(
            json["hints"],
            serde_json::json!(["check filesystem state", "verify file permissions"])
        );
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_log_response_json_contains_machine_facing_diagnostics() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("disk offline")
            .with_std_source(std::io::Error::other("root cause"))
            .with_context(OperationContext::doing("load config").with_meta("tenant", "acme"));

        let json = err
            .exposure_snapshot(&DefaultExposurePolicy)
            .to_log_error_json()
            .unwrap();

        assert_eq!(json["code"], serde_json::json!("sys.io_error"));
        assert_eq!(json["category"], serde_json::json!("sys"));
        assert_eq!(json["reason"], serde_json::json!("system error"));
        assert_eq!(json["detail"], serde_json::json!("disk offline"));
        assert_eq!(json["operation"], serde_json::json!("load config"));
        assert_eq!(json["path"], serde_json::json!("load config"));
        assert_eq!(json["visibility"], serde_json::json!("internal"));
        assert_eq!(
            json["hints"],
            serde_json::json!(["check filesystem state", "verify file permissions"])
        );
        assert_eq!(json["root_metadata"]["tenant"], serde_json::json!("acme"));
        assert_eq!(json["source_frames"][0]["message"], serde_json::json!("root cause"));
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_rpc_response_json_hides_internal_detail_and_marks_retryable() {
        let err = StructError::from(TestReason::Uvs(UvsReason::timeout_error()))
            .with_detail("downstream rpc timeout");
        let json = err
            .exposure_snapshot(&DefaultExposurePolicy)
            .to_rpc_error_json()
            .unwrap();

        assert_eq!(json["status"], serde_json::json!(500));
        assert_eq!(json["code"], serde_json::json!("sys.timeout"));
        assert_eq!(json["category"], serde_json::json!("sys"));
        assert_eq!(json["reason"], serde_json::json!("timeout error"));
        assert_eq!(json["detail"], serde_json::json!(null));
        assert_eq!(json["visibility"], serde_json::json!("internal"));
        assert_eq!(
            json["hints"],
            serde_json::json!(["retry later", "inspect downstream service latency"])
        );
        assert_eq!(json["retryable"], serde_json::json!(true));
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_rpc_response_json_keeps_public_detail() {
        let err = StructError::from(TestReason::Uvs(UvsReason::business_error()))
            .with_detail("order state invalid");
        let json = err
            .exposure_snapshot(&DefaultExposurePolicy)
            .to_rpc_error_json()
            .unwrap();

        assert_eq!(json["status"], serde_json::json!(400));
        assert_eq!(json["code"], serde_json::json!("biz.business_error"));
        assert_eq!(json["category"], serde_json::json!("biz"));
        assert_eq!(json["reason"], serde_json::json!("business logic error"));
        assert_eq!(json["detail"], serde_json::json!("order state invalid"));
        assert_eq!(json["visibility"], serde_json::json!("public"));
        assert_eq!(json["hints"], serde_json::json!([]));
        assert_eq!(json["retryable"], serde_json::json!(false));
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_cli_error_json_keys_match_expected_shape() {
        let err = StructError::from(TestReason::Uvs(UvsReason::business_error()))
            .with_detail("order state invalid");

        let json_value = err
            .exposure_snapshot(&DefaultExposurePolicy)
            .to_cli_error_json()
            .expect("serialize cli error");

        let mut keys = json_value
            .as_object()
            .expect("cli error object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();

        let mut expected = [
            "code",
            "category",
            "summary",
            "detail",
            "visibility",
            "hints",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(keys, expected);
        assert_eq!(json_value["code"], serde_json::json!("biz.business_error"));
        assert_eq!(
            json_value["summary"],
            serde_json::json!("business logic error: order state invalid")
        );
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_log_error_json_keys_match_expected_shape() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("disk offline")
            .with_context(OperationContext::doing("load config"));

        let json_value = err
            .exposure_snapshot(&DefaultExposurePolicy)
            .to_log_error_json()
            .expect("serialize log error");

        let mut keys = json_value
            .as_object()
            .expect("log error object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();

        let mut expected = [
            "code",
            "category",
            "reason",
            "detail",
            "operation",
            "path",
            "visibility",
            "hints",
            "root_metadata",
            "context",
            "source_frames",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(keys, expected);
        assert_eq!(json_value["code"], serde_json::json!("sys.io_error"));
        assert_eq!(json_value["reason"], serde_json::json!("system error"));
        assert_eq!(json_value["operation"], serde_json::json!("load config"));
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_rpc_error_json_keys_match_expected_shape() {
        let err = StructError::from(TestReason::Uvs(UvsReason::timeout_error()))
            .with_detail("downstream rpc timeout");

        let json_value = err
            .exposure_snapshot(&DefaultExposurePolicy)
            .to_rpc_error_json()
            .expect("serialize rpc error");

        let mut keys = json_value
            .as_object()
            .expect("rpc error object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();

        let mut expected = [
            "status",
            "code",
            "category",
            "reason",
            "detail",
            "visibility",
            "hints",
            "retryable",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(keys, expected);
        assert_eq!(json_value["code"], serde_json::json!("sys.timeout"));
        assert_eq!(json_value["retryable"], serde_json::json!(true));
        assert_eq!(json_value["detail"], serde_json::Value::Null);
    }
