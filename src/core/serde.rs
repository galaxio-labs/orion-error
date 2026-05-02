//! All serde-dependent code: manual Serialize impls + serde-gated tests.
//!
//! This module is only compiled when `feature = "serde"` is enabled.
//! Keeping serde logic in one place makes feature gating and maintenance simpler.

use crate::core::{DomainReason, StructError};

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
        let path = self.target_path();

        let mut state = serializer.serialize_struct("StructError", 8)?;
        state.serialize_field("reason", self.imp().reason())?;
        state.serialize_field("detail", self.imp().detail())?;
        state.serialize_field("position", self.imp().position())?;
        state.serialize_field("context", self.imp().context())?;
        state.serialize_field("path", &path)?;
        state.serialize_field("source_frames", &source_frames)?;
        state.serialize_field("source_message", &source_message)?;
        state.serialize_field("source_chain", &source_chain)?;
        state.end()
    }
}

// ── Serde-gated tests ──

#[cfg(test)]
mod tests {
    use crate::core::{
        DomainReason, ErrorCategory, ErrorIdentity, ErrorMetadata, OperationContext, SourceFrame,
        StructError,
    };
    use crate::reason::{ErrorCode, ErrorIdentityProvider};
    use crate::report::RedactPolicy;
    use crate::UnifiedReason;

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
        General(UnifiedReason),
    }

    impl From<UnifiedReason> for TestReason {
        fn from(value: UnifiedReason) -> Self {
            Self::General(value)
        }
    }

    impl DomainReason for TestReason {}

    impl ErrorCode for TestReason {
        fn error_code(&self) -> i32 {
            match self {
                TestReason::TestError => 1001,
                TestReason::General(reason) => reason.error_code(),
            }
        }
    }

    impl ErrorIdentityProvider for TestReason {
        fn stable_code(&self) -> &'static str {
            match self {
                TestReason::TestError => "test.test_error",
                TestReason::General(reason) => reason.stable_code(),
            }
        }

        fn error_category(&self) -> ErrorCategory {
            match self {
                TestReason::TestError => ErrorCategory::Logic,
                TestReason::General(reason) => reason.error_category(),
            }
        }
    }

    // ── Tests from error.rs ──

    #[test]
    fn test_source_frame_serialization_skips_empty_metadata() {
        let frame = SourceFrame {
            index: 0,
            message: "message".into(),
            display: None,
            debug: Some("debug".into()),
            type_name: None,
            error_code: None,
            reason: None,
            path: None,
            detail: None,
            metadata: ErrorMetadata::default(),
            is_root_cause: true,
            context_fields: Vec::new(),
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
        let wrapped = StructError::from(TestReason::from(UnifiedReason::system_error()))
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
            path: Some("start engine".to_string()),
        };

        let value = serde_json::to_value(identity).unwrap();

        assert_eq!(value["code"], serde_json::json!("sys.io_error"));
        assert_eq!(value["category"], serde_json::json!("sys"));
        assert_eq!(value["reason"], serde_json::json!("system error"));
    }

    #[test]
    // ── Tests from report.rs ──
    #[test]
    fn test_report_serialization_supports_structured_export() {
        let source = StructError::from(TestReason::TestError).with_context(
            OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
        );
        let err = StructError::from(TestReason::from(UnifiedReason::system_error()))
            .with_context(
                OperationContext::doing("start engine").with_meta("component.name", "engine"),
            )
            .with_struct_source(source);

        let json_value = serde_json::to_value(err.report()).expect("serialize report");

        assert_eq!(json_value["reason"], serde_json::json!("system error"));
        // DiagnosticReport no longer includes root_metadata or source_frames.
        assert!(json_value.get("root_metadata").is_none());
        assert!(json_value.get("source_frames").is_none());
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
