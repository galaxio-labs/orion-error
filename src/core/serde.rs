//! All serde-dependent code: manual Serialize impls + serde-gated tests.
//!
//! This module is only compiled when `feature = "serde"` is enabled.
//! Keeping serde logic in one place makes feature gating and maintenance simpler.

use crate::core::{DomainReason, ErrorSnapshot, StructError};

// ── Manual Serialize impls ──

impl<T: DomainReason> serde::Serialize for StructError<T>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let source_frames = self.source_frames();
        let source_chain = self.source_chain();
        let source_message = source_chain.first().cloned();
        let want = self.target_main();
        let path = self.target_path();

        let mut state = serializer.serialize_struct("StructError", 9)?;
        state.serialize_field("reason", self.imp().reason())?;
        state.serialize_field("detail", self.imp().detail())?;
        state.serialize_field("position", self.imp().position())?;
        state.serialize_field("context", self.imp().context())?;
        state.serialize_field("want", &want)?;
        state.serialize_field("path", &path)?;
        state.serialize_field("source_frames", &source_frames)?;
        state.serialize_field("source_message", &source_message)?;
        state.serialize_field("source_chain", &source_chain)?;
        state.end()
    }
}

impl serde::Serialize for ErrorSnapshot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.stable_export().serialize(serializer)
    }
}

// ── Serde-gated tests ──

#[cfg(test)]
mod tests {
    use crate::core::context::ContextRecord;
    use crate::core::{
        context::OperationResult, DomainReason, ErrorCategory, ErrorIdentity, ErrorMetadata,
        ErrorSnapshot, OperationContext, SnapshotContextFrame, SnapshotSourceFrame, SourceFrame,
        StableErrorSnapshot, StructError, STABLE_SNAPSHOT_SCHEMA_VERSION,
    };
    use crate::report::RedactPolicy;
    use crate::{ErrorCode, ErrorIdentityProvider, UvsReason};

    // ── Test types ──

    #[derive(Debug, serde::Serialize)]
    struct TestPolicy;

    impl RedactPolicy for TestPolicy {
        fn redact_key(&self, key: &str) -> bool {
            matches!(key, "token" | "password" | "config.secret")
        }

        fn redact_value(&self, _key: Option<&str>, _value: &str) -> Option<String> {
            Some("<redacted>".to_string())
        }
    }

    #[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize)]
    enum TestReason {
        #[error("test error")]
        TestError,
        #[error("{0}")]
        Uvs(UvsReason),
    }

    impl From<UvsReason> for TestReason {
        fn from(value: UvsReason) -> Self {
            Self::Uvs(value)
        }
    }

    impl DomainReason for TestReason {}

    impl ErrorCode for TestReason {
        fn error_code(&self) -> i32 {
            match self {
                TestReason::TestError => 1001,
                TestReason::Uvs(reason) => reason.error_code(),
            }
        }
    }

    impl ErrorIdentityProvider for TestReason {
        fn stable_code(&self) -> &'static str {
            match self {
                TestReason::TestError => "test.test_error",
                TestReason::Uvs(reason) => reason.stable_code(),
            }
        }

        fn error_category(&self) -> ErrorCategory {
            match self {
                TestReason::TestError => ErrorCategory::Logic,
                TestReason::Uvs(reason) => reason.error_category(),
            }
        }
    }

    // ── Tests from error.rs ──

    #[test]
    fn test_source_frame_serialization_skips_empty_metadata() {
        let frame = SourceFrame {
            index: 0,
            message: "message".to_string(),
            display: None,
            debug: "debug".to_string(),
            type_name: None,
            error_code: None,
            reason: None,
            want: None,
            path: None,
            detail: None,
            metadata: ErrorMetadata::default(),
            is_root_cause: true,
        };

        let json_value = serde_json::to_value(&frame).unwrap();
        assert!(!json_value
            .as_object()
            .expect("object")
            .contains_key("metadata"));
    }

    #[test]
    fn test_source_frame_serialization_includes_metadata() {
        let error = StructError::from(TestReason::TestError).with_context(
            OperationContext::doing("load sink defaults")
                .with_meta("config.kind", "sink_defaults")
                .with_meta("parse.line", 1u32),
        );
        let wrapped = StructError::from(TestReason::from(UvsReason::system_error()))
            .with_struct_source(error);

        let json_value = serde_json::to_value(&wrapped).unwrap();
        assert_eq!(
            json_value["source_frames"][0]["metadata"]["config.kind"],
            serde_json::Value::String("sink_defaults".to_string())
        );
        assert_eq!(
            json_value["source_frames"][0]["metadata"]["parse.line"],
            serde_json::json!(1)
        );
    }

    // ── Tests from context.rs ──

    #[test]
    fn test_context_serialization() {
        let mut ctx = OperationContext::doing("serialization_test");
        ctx.with_doing("inner_step");
        ctx.record("key1", "value1");
        ctx.record("key2", "value2");

        let serialized = serde_json::to_string(&ctx).expect("序列化失败");
        assert!(serialized.contains("serialization_test"));
        assert!(serialized.contains("inner_step"));
        assert!(serialized.contains("key1"));
        assert!(serialized.contains("value1"));

        let deserialized: OperationContext =
            serde_json::from_str(&serialized).expect("反序列化失败");
        assert_eq!(ctx, deserialized);
    }

    #[test]
    fn test_context_serialization_skips_empty_metadata_and_reads_missing_field() {
        let ctx = OperationContext::doing("serialization_test");

        let serialized = serde_json::to_value(&ctx).expect("序列化失败");
        assert!(!serialized
            .as_object()
            .expect("object")
            .contains_key("metadata"));

        let deserialized: OperationContext =
            serde_json::from_value(serialized).expect("反序列化失败");
        assert!(deserialized.metadata().is_empty());
    }

    #[test]
    fn test_context_serialization_preserves_metadata() {
        let ctx = OperationContext::doing("serialization_test")
            .with_meta("config.kind", "sink_defaults")
            .with_meta("parse.line", 3u32);

        let serialized = serde_json::to_value(&ctx).expect("序列化失败");
        assert_eq!(
            serialized["metadata"]["config.kind"],
            serde_json::Value::String("sink_defaults".to_string())
        );
        assert_eq!(serialized["metadata"]["parse.line"], serde_json::json!(3));

        let deserialized: OperationContext =
            serde_json::from_value(serialized).expect("反序列化失败");
        assert_eq!(
            deserialized.metadata().get_str("config.kind"),
            Some("sink_defaults")
        );
    }

    // ── Tests from snapshot.rs ──

    #[test]
    fn test_identity_snapshot_serialization_includes_code_and_category() {
        let identity = ErrorIdentity {
            code: "sys.io_error".to_string(),
            category: ErrorCategory::Sys,
            reason: "system error".to_string(),
            detail: Some("engine bootstrap failed".to_string()),
            position: Some("src/main.rs:42".to_string()),
            want: Some("start engine".to_string()),
            path: Some("start engine".to_string()),
        };

        let value = serde_json::to_value(identity).unwrap();

        assert_eq!(value["code"], serde_json::json!("sys.io_error"));
        assert_eq!(value["category"], serde_json::json!("Sys"));
        assert_eq!(value["reason"], serde_json::json!("system error"));
    }

    #[test]
    fn test_snapshot_default_serialization_uses_stable_export_shape() {
        let source = StructError::from(TestReason::TestError)
            .with_detail("inner detail")
            .with_context(
                OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
            );
        let err = StructError::from(TestReason::from(UvsReason::system_error()))
            .with_detail("engine bootstrap failed")
            .with_position("src/main.rs:42")
            .with_context(
                OperationContext::doing("start engine").with_meta("component.name", "engine"),
            )
            .with_struct_source(source);

        let json_value = serde_json::to_value(err.snapshot()).unwrap();

        assert_eq!(
            json_value["schema_version"],
            serde_json::json!(STABLE_SNAPSHOT_SCHEMA_VERSION)
        );
        assert_eq!(json_value["reason"], serde_json::json!("system error"));
        assert_eq!(
            json_value["detail"],
            serde_json::json!("engine bootstrap failed")
        );
        assert_eq!(json_value["position"], serde_json::json!("src/main.rs:42"));
        assert_eq!(json_value["want"], serde_json::json!("start engine"));
        assert_eq!(json_value["path"], serde_json::json!("start engine"));
        assert_eq!(
            json_value["root_metadata"]["component.name"],
            serde_json::json!("engine")
        );
        assert_eq!(
            json_value["context"][0]["target"],
            serde_json::json!("start engine")
        );
        assert_eq!(
            json_value["context"][0]["action"],
            serde_json::json!("start engine")
        );
        assert_eq!(json_value["context"][0]["locator"], serde_json::Value::Null);
        assert_eq!(
            json_value["context"][0]["path"],
            serde_json::json!(["start engine"])
        );
        assert_eq!(
            json_value["context"][0]["metadata"]["component.name"],
            serde_json::json!("engine")
        );
        assert!(json_value["context"][0].get("fields").is_none());
        assert!(json_value["context"][0].get("result").is_none());
        assert_eq!(
            json_value["source_frames"][0]["message"],
            serde_json::json!("test error")
        );
        assert_eq!(
            json_value["source_frames"][0]["reason"],
            serde_json::json!("test error")
        );
        assert_eq!(
            json_value["source_frames"][0]["want"],
            serde_json::json!("load defaults")
        );
        assert_eq!(
            json_value["source_frames"][0]["path"],
            serde_json::json!("load defaults")
        );
        assert_eq!(
            json_value["source_frames"][0]["detail"],
            serde_json::json!("inner detail")
        );
        assert_eq!(
            json_value["source_frames"][0]["metadata"]["config.kind"],
            serde_json::json!("sink_defaults")
        );
        assert_eq!(
            json_value["source_frames"][0]["is_root_cause"],
            serde_json::json!(true)
        );
        assert!(json_value["source_frames"][0].get("debug").is_none());
        assert!(json_value["source_frames"][0].get("display").is_none());
        assert!(json_value["source_frames"][0].get("type_name").is_none());
        assert!(json_value.get("source_message").is_none());
        assert!(json_value.get("source_chain").is_none());
    }

    #[test]
    fn test_snapshot_stable_export_serialization_omits_compat_projection_fields() {
        let snapshot = ErrorSnapshot {
            reason: "system error".to_string(),
            detail: Some("outer detail".to_string()),
            position: Some("src/main.rs:42".to_string()),
            want: Some("start engine".to_string()),
            path: Some("start engine".to_string()),
            context: vec![SnapshotContextFrame {
                target: Some("start engine".to_string()),
                action: Some("start engine".to_string()),
                locator: Some("engine.toml".to_string()),
                path: vec!["start engine".to_string()],
                metadata: {
                    let mut metadata = ErrorMetadata::new();
                    metadata.insert("component.name", "engine");
                    metadata
                },
                fields: vec![("tenant".to_string(), "alpha".to_string())],
                result: OperationResult::Fail,
            }],
            root_metadata: {
                let mut metadata = ErrorMetadata::new();
                metadata.insert("component.name", "engine");
                metadata
            },
            source_frames: vec![SnapshotSourceFrame {
                index: 0,
                message: "db unavailable".to_string(),
                display: Some("db unavailable".to_string()),
                type_name: Some("std::io::Error".to_string()),
                error_code: None,
                reason: None,
                want: Some("load config".to_string()),
                path: Some("load config / read".to_string()),
                detail: Some("inner detail".to_string()),
                metadata: {
                    let mut metadata = ErrorMetadata::new();
                    metadata.insert("config.kind", "sink_defaults");
                    metadata
                },
                is_root_cause: true,
            }],
            category: ErrorCategory::Sys,
        };

        let stable = snapshot.stable_export();
        let json_value = serde_json::to_value(&stable).unwrap();

        assert_eq!(StableErrorSnapshot::clone(&stable), stable);
        assert_eq!(
            json_value["schema_version"],
            serde_json::json!(STABLE_SNAPSHOT_SCHEMA_VERSION)
        );
        assert_eq!(json_value["reason"], serde_json::json!("system error"));
        assert_eq!(
            json_value["context"][0]["target"],
            serde_json::json!("start engine")
        );
        assert_eq!(
            json_value["context"][0]["action"],
            serde_json::json!("start engine")
        );
        assert_eq!(
            json_value["context"][0]["locator"],
            serde_json::json!("engine.toml")
        );
        assert!(json_value["context"][0].get("fields").is_none());
        assert!(json_value["context"][0].get("result").is_none());
        assert_eq!(
            json_value["source_frames"][0]["index"],
            serde_json::json!(0)
        );
        assert!(json_value["source_frames"][0].get("debug").is_none());
        assert!(json_value["source_frames"][0].get("display").is_none());
        assert!(json_value["source_frames"][0].get("type_name").is_none());
    }

    // ── Tests from report.rs ──

    #[test]
    fn test_report_serialization_supports_structured_export() {
        let source = StructError::from(TestReason::TestError).with_context(
            OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
        );
        let err = StructError::from(TestReason::from(UvsReason::system_error()))
            .with_context(
                OperationContext::doing("start engine").with_meta("component.name", "engine"),
            )
            .with_struct_source(source);

        let json_value = serde_json::to_value(err.report()).expect("serialize report");

        assert_eq!(json_value["reason"], serde_json::json!("system error"));
        assert_eq!(
            json_value["root_metadata"]["component.name"],
            serde_json::json!("engine")
        );
        assert_eq!(
            json_value["source_frames"][0]["metadata"]["config.kind"],
            serde_json::json!("sink_defaults")
        );
    }

    #[test]
    fn test_report_redacted_supports_structured_export() {
        let err = StructError::from(TestReason::TestError)
            .with_detail("token=abc")
            .with_std_source(std::io::Error::other("token=abc"))
            .with_context(OperationContext::doing("load").with_meta("config.secret", "abc"));

        let json_value =
            serde_json::to_value(err.report_redacted(&TestPolicy)).expect("serialize redacted");

        let encoded = serde_json::to_string(&json_value).expect("json string");
        assert!(encoded.contains("<redacted>"));
        assert!(!encoded.contains("token=abc"));
        assert!(!encoded.contains("\"abc\""));
    }
}
