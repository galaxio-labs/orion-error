use crate::reason::{ErrorCategory, ErrorCode, ErrorIdentityProvider};
use crate::{
    core::{DomainReason, ErrorMetadata, SourceFrame},
    OperationContext, StructError, UvsReason,
};

use super::{
    DiagnosticReport, ErrorSnapshot, SnapshotContextFrame, SnapshotSourceFrame,
    StableErrorSnapshot, StableSnapshotContextFrame, STABLE_SNAPSHOT_SCHEMA_VERSION,
};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
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

#[test]
fn test_snapshot_captures_runtime_fields_and_source_frames() {
    let source = StructError::from(TestReason::TestError).with_context(
        OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
    );
    let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
        .with_detail("engine bootstrap failed")
        .with_position("src/main.rs:42")
        .with_context(OperationContext::doing("start engine").with_meta("component.name", "engine"))
        .with_struct_source(source);

    let snapshot = err.snapshot();

    assert_eq!(snapshot.reason, "system error");
    assert_eq!(snapshot.detail.as_deref(), Some("engine bootstrap failed"));
    assert_eq!(snapshot.position.as_deref(), Some("src/main.rs:42"));
    assert_eq!(snapshot.context[0].target.as_deref(), Some("start engine"));
    assert_eq!(
        snapshot.root_metadata.get_str("component.name"),
        Some("engine")
    );
    assert_eq!(
        snapshot.source_frames[0].metadata.get_str("config.kind"),
        Some("sink_defaults")
    );
}

#[test]
fn test_identity_snapshot_captures_stable_identity_fields() {
    let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
        .with_detail("engine bootstrap failed")
        .with_position("src/main.rs:42")
        .with_context(OperationContext::doing("start engine"));

    let identity = err.identity_snapshot();

    assert_eq!(identity.code, "sys.io_error");
    assert_eq!(identity.category, ErrorCategory::Sys);
    assert_eq!(identity.reason, "system error");
    assert_eq!(identity.detail.as_deref(), Some("engine bootstrap failed"));
    assert_eq!(identity.position.as_deref(), Some("src/main.rs:42"));
    assert_eq!(identity.path.as_deref(), Some("start engine"));
}

#[test]
fn test_snapshot_preserves_action_and_locator_context_fields() {
    let mut ctx = OperationContext::at("config.toml");
    ctx.with_doing("parse config");

    let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
        .with_context(OperationContext::doing("load config").with_meta("component.name", "engine"))
        .with_context(ctx);

    let snapshot = err.snapshot();

    assert_eq!(snapshot.context[0].action.as_deref(), Some("load config"));
    assert_eq!(snapshot.context[1].action.as_deref(), Some("parse config"));
    assert_eq!(snapshot.context[1].locator.as_deref(), Some("config.toml"));

    let report = snapshot.into_report();
    assert_eq!(
        report.context()[1].action().as_deref(),
        Some("parse config")
    );
    assert_eq!(
        report.context()[1].locator().as_deref(),
        Some("config.toml")
    );
}

#[test]
fn test_snapshot_report_conversion_preserves_payload() {
    let snapshot = ErrorSnapshot {
        reason: "system error".to_string(),
        detail: Some("engine bootstrap failed".to_string()),
        position: Some("src/main.rs:42".to_string()),
        path: Some("start engine / load defaults".to_string()),
        context: vec![SnapshotContextFrame {
            target: Some("start engine".to_string()),
            action: None,
            locator: None,
            path: vec!["start engine".to_string()],
            metadata: ErrorMetadata::new(),
            fields: vec![],
            result: crate::core::context::OperationResult::Fail,
        }],
        root_metadata: {
            let mut metadata = ErrorMetadata::new();
            metadata.insert("component.name", "engine");
            metadata
        },
        source_frames: vec![],
        category: ErrorCategory::Sys,
        code: "sys.test_error".to_string(),
    };

    let report = snapshot.report();

    assert_eq!(report.reason(), snapshot.reason);
    assert_eq!(report.detail(), snapshot.detail.as_deref());
    assert_eq!(report.position(), snapshot.position.as_deref());
    assert_eq!(
        report.context(),
        snapshot
            .context
            .clone()
            .into_iter()
            .map(Into::into)
            .collect::<Vec<OperationContext>>()
            .as_slice()
    );
}

#[test]
fn test_snapshot_from_struct_error_matches_snapshot_method() {
    let err = StructError::from(TestReason::TestError)
        .with_detail("engine bootstrap failed")
        .with_context(OperationContext::doing("start engine"));

    let via_method = err.snapshot();
    let via_from = ErrorSnapshot::from(&err);

    assert_eq!(via_from, via_method);
}

#[test]
fn test_snapshot_from_owned_struct_error_matches_snapshot_method() {
    let err = StructError::from(TestReason::TestError)
        .with_detail("engine bootstrap failed")
        .with_context(OperationContext::doing("start engine"));

    let via_method = err.snapshot();
    let via_from = ErrorSnapshot::from(err);

    assert_eq!(via_from, via_method);
}

#[test]
fn test_struct_error_into_snapshot_matches_snapshot_method() {
    let err = StructError::from(TestReason::TestError)
        .with_detail("engine bootstrap failed")
        .with_context(OperationContext::doing("start engine"));

    let via_method = err.snapshot();
    let via_into = err.into_snapshot();

    assert_eq!(via_into, via_method);
}

#[test]
fn test_snapshot_into_report_matches_borrowed_report() {
    let snapshot = ErrorSnapshot {
        reason: "system error".to_string(),
        detail: Some("engine bootstrap failed".to_string()),
        position: Some("src/main.rs:42".to_string()),
        path: Some("start engine".to_string()),
        context: vec![SnapshotContextFrame {
            target: Some("start engine".to_string()),
            action: None,
            locator: None,
            path: vec!["start engine".to_string()],
            metadata: ErrorMetadata::new(),
            fields: vec![("tenant".to_string(), "alpha".to_string())],
            result: crate::core::context::OperationResult::Fail,
        }],
        root_metadata: ErrorMetadata::new(),
        source_frames: vec![SnapshotSourceFrame {
            index: 0,
            message: "db unavailable".to_string(),
            display: Some("db unavailable".to_string()),
            type_name: Some("std::io::Error".to_string()),
            error_code: None,
            reason: None,
            path: Some("load config / read".to_string()),
            detail: Some("inner detail".to_string()),
            metadata: ErrorMetadata::new(),
            is_root_cause: true,
        }],
        category: ErrorCategory::Sys,
        code: "sys.test_error".to_string(),
    };

    let via_borrowed = snapshot.report();
    let via_owned = snapshot.clone().into_report();
    let via_from = DiagnosticReport::from(snapshot);

    assert_eq!(via_owned, via_borrowed);
    assert_eq!(via_from, via_borrowed);
}

#[test]
fn test_snapshot_stable_helpers_prefer_snapshot_native_frames() {
    let source = StructError::from(TestReason::TestError)
        .with_detail("inner detail")
        .with_context(
            OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
        );
    let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
        .with_detail("outer detail")
        .with_context(OperationContext::doing("start engine"))
        .with_struct_source(source);

    let snapshot = err.snapshot();

    assert_eq!(snapshot.stable_context(), snapshot.context.as_slice());
    assert_eq!(
        snapshot.stable_source_frames(),
        snapshot.source_frames.as_slice()
    );
    assert_eq!(snapshot.root_source_frame().unwrap().message, "test error");
    assert_eq!(
        snapshot
            .root_source_frame()
            .unwrap()
            .metadata
            .get_str("config.kind"),
        Some("sink_defaults")
    );
}

#[test]
fn test_snapshot_stable_export_strips_compat_projection_fields() {
    let source = StructError::from(TestReason::TestError)
        .with_detail("inner detail")
        .with_context(
            OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
        );
    let mut outer = OperationContext::at("engine.toml");
    outer.with_doing("start engine");
    let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
        .with_detail("outer detail")
        .with_context(outer)
        .with_struct_source(source);

    let snapshot = err.snapshot();
    let stable = snapshot.stable_export();

    assert_eq!(stable.schema_version(), STABLE_SNAPSHOT_SCHEMA_VERSION);
    assert_eq!(stable.reason(), snapshot.reason);
    assert_eq!(stable.path(), Some("start engine / engine.toml"));
    assert_eq!(stable.root_metadata().get_str("component.name"), None);
    let report = stable.report();
    assert_eq!(
        report.context()[0].compat_target().as_deref(),
        Some("start engine")
    );
    assert_eq!(
        report.context()[0].action().as_deref(),
        Some("start engine")
    );
    assert_eq!(
        report.context()[0].locator().as_deref(),
        Some("engine.toml")
    );
}

#[test]
fn test_snapshot_into_stable_export_matches_borrowed_stable_export() {
    let snapshot = ErrorSnapshot {
        reason: "system error".to_string(),
        detail: Some("outer detail".to_string()),
        position: Some("src/main.rs:42".to_string()),
        path: Some("start engine".to_string()),
        context: vec![SnapshotContextFrame {
            target: Some("start engine".to_string()),
            action: None,
            locator: None,
            path: vec!["start engine".to_string()],
            metadata: ErrorMetadata::new(),
            fields: vec![("tenant".to_string(), "alpha".to_string())],
            result: crate::core::context::OperationResult::Fail,
        }],
        root_metadata: ErrorMetadata::new(),
        source_frames: vec![SnapshotSourceFrame {
            index: 0,
            message: "db unavailable".to_string(),
            display: Some("db unavailable".to_string()),
            type_name: Some("std::io::Error".to_string()),
            error_code: None,
            reason: None,
            path: Some("load config / read".to_string()),
            detail: Some("inner detail".to_string()),
            metadata: ErrorMetadata::new(),
            is_root_cause: true,
        }],
        category: ErrorCategory::Sys,
        code: "sys.test_error".to_string(),
    };

    let via_borrowed = snapshot.stable_export();
    let via_owned = snapshot.clone().into_stable_export();
    let via_from_borrowed = StableErrorSnapshot::from(&snapshot);
    let via_from_owned = StableErrorSnapshot::from(snapshot);

    assert_eq!(via_owned, via_borrowed);
    assert_eq!(via_from_borrowed, via_borrowed);
    assert_eq!(via_from_owned, via_borrowed);
    assert_eq!(
        via_borrowed.schema_version(),
        STABLE_SNAPSHOT_SCHEMA_VERSION
    );
}

#[test]
fn test_stable_snapshot_from_struct_error_matches_snapshot_stable_export() {
    let source = StructError::from(TestReason::TestError)
        .with_detail("inner detail")
        .with_context(
            OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
        );
    let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
        .with_detail("outer detail")
        .with_context(OperationContext::doing("start engine"))
        .with_struct_source(source);

    let via_method = err.snapshot().stable_export();
    let via_borrowed = StableErrorSnapshot::from(&err);
    let via_owned = StableErrorSnapshot::from(err);

    assert_eq!(via_borrowed, via_method);
    assert_eq!(via_owned, via_method);
}

#[test]
fn test_stable_snapshot_into_report_matches_report() {
    let stable = ErrorSnapshot {
        reason: "system error".to_string(),
        detail: Some("outer detail".to_string()),
        position: None,
        path: Some("start engine".to_string()),
        context: vec![SnapshotContextFrame {
            target: Some("start engine".to_string()),
            action: None,
            locator: None,
            path: vec!["start engine".to_string()],
            metadata: ErrorMetadata::new(),
            fields: vec![],
            result: crate::core::context::OperationResult::Fail,
        }],
        root_metadata: ErrorMetadata::new(),
        source_frames: vec![SnapshotSourceFrame {
            index: 0,
            message: "db unavailable".to_string(),
            display: None,
            type_name: None,
            error_code: None,
            reason: None,
            path: Some("load config / read".to_string()),
            detail: Some("inner detail".to_string()),
            metadata: ErrorMetadata::new(),
            is_root_cause: true,
        }],
        category: ErrorCategory::Sys,
        code: "sys.test_error".to_string(),
    }
    .stable_export();

    let via_method = stable.report();
    let via_owned = stable.clone().into_report();

    assert_eq!(via_owned, via_method);
}

#[test]
fn test_stable_snapshot_accessors_expose_only_top_level_contract() {
    let stable = ErrorSnapshot {
        reason: "system error".to_string(),
        detail: Some("outer detail".to_string()),
        position: Some("src/main.rs:42".to_string()),
        path: Some("start engine / engine.toml".to_string()),
        context: vec![],
        root_metadata: {
            let mut metadata = ErrorMetadata::new();
            metadata.insert("component.name", "engine");
            metadata
        },
        source_frames: vec![],
        category: ErrorCategory::Sys,
        code: "sys.test_error".to_string(),
    }
    .stable_export();

    assert_eq!(stable.schema_version(), STABLE_SNAPSHOT_SCHEMA_VERSION);
    assert_eq!(stable.reason(), "system error");
    assert_eq!(stable.detail(), Some("outer detail"));
    assert_eq!(stable.position(), Some("src/main.rs:42"));
    assert_eq!(stable.path(), Some("start engine / engine.toml"));
    assert_eq!(stable.category(), ErrorCategory::Sys);
    assert_eq!(stable.code(), "sys.test_error");
    assert_eq!(
        stable.root_metadata().get_str("component.name"),
        Some("engine")
    );
}

#[test]
fn test_snapshot_context_frame_roundtrip_to_operation_context() {
    let mut ctx = OperationContext::doing("start engine");
    ctx.with_doing("load defaults");
    ctx.record("tenant", "alpha");
    ctx.record_meta("component.name", "engine");

    let snapshot_frame = SnapshotContextFrame::from(ctx.clone());
    let roundtrip: OperationContext = snapshot_frame.clone().into();

    assert_eq!(snapshot_frame.target.as_deref(), Some("start engine"));
    assert_eq!(
        snapshot_frame.path,
        vec!["start engine".to_string(), "load defaults".to_string()]
    );
    assert_eq!(roundtrip.compat_target().as_deref(), Some("start engine"));
    assert_eq!(
        roundtrip.path(),
        vec!["start engine".to_string(), "load defaults".to_string()]
    );
    assert_eq!(
        roundtrip.metadata().get_str("component.name"),
        Some("engine")
    );
    assert_eq!(
        roundtrip.context().items,
        vec![("tenant".to_string(), "alpha".to_string())]
    );
}

#[test]
fn test_snapshot_context_frame_roundtrip_normalizes_action_locator_path() {
    let mut ctx = OperationContext::at("engine.toml");
    ctx.with_doing("start engine");

    let snapshot_frame = SnapshotContextFrame::from(ctx);
    let roundtrip: OperationContext = snapshot_frame.clone().into();

    assert_eq!(snapshot_frame.target.as_deref(), Some("start engine"));
    assert_eq!(snapshot_frame.action.as_deref(), Some("start engine"));
    assert_eq!(snapshot_frame.locator.as_deref(), Some("engine.toml"));
    assert_eq!(
        snapshot_frame.path,
        vec!["start engine".to_string(), "engine.toml".to_string()]
    );
    assert_eq!(
        roundtrip.path(),
        vec!["start engine".to_string(), "engine.toml".to_string()]
    );
    assert_eq!(
        roundtrip.path_string().as_deref(),
        Some("start engine / engine.toml")
    );
}

#[test]
fn test_snapshot_source_frame_roundtrip_to_report_frame() {
    let frame = SnapshotSourceFrame {
        index: 0,
        message: "db unavailable".to_string(),
        display: Some("db unavailable".to_string()),
        type_name: Some("std::io::Error".to_string()),
        error_code: None,
        reason: None,
        path: Some("load config / read".to_string()),
        detail: Some("inner detail".to_string()),
        metadata: {
            let mut metadata = ErrorMetadata::new();
            metadata.insert("config.kind", "sink_defaults");
            metadata
        },
        is_root_cause: true,
    };

    let report_frame: SourceFrame = frame.clone().into();
    let roundtrip = SnapshotSourceFrame::from(report_frame);

    assert_eq!(roundtrip, frame);
}

#[test]
fn test_stable_snapshot_source_frame_to_source_frame_roundtrip() {
    let stable = super::StableSnapshotSourceFrame {
        index: 0,
        message: "db unavailable".to_string(),
        error_code: Some(200),
        reason: Some("test error".to_string()),
        path: Some("load config / read".to_string()),
        detail: Some("inner detail".to_string()),
        metadata: {
            let mut metadata = ErrorMetadata::new();
            metadata.insert("config.kind", "sink_defaults");
            metadata
        },
        is_root_cause: true,
    };

    let source_frame: SourceFrame = stable.clone().into();
    let roundtrip: SnapshotSourceFrame = source_frame.into();

    assert_eq!(roundtrip.index, stable.index);
    assert_eq!(roundtrip.message, stable.message);
    assert_eq!(roundtrip.error_code, stable.error_code);
    assert_eq!(roundtrip.reason, stable.reason);
    assert_eq!(roundtrip.path, stable.path);
    assert_eq!(roundtrip.detail, stable.detail);
    assert_eq!(
        roundtrip.metadata.get_str("config.kind"),
        Some("sink_defaults")
    );
    assert!(roundtrip.is_root_cause);
}

#[test]
fn test_stable_snapshot_context_frame_to_operation_context_roundtrip() {
    let stable = StableSnapshotContextFrame {
        target: Some("start engine".to_string()),
        action: Some("start engine".to_string()),
        locator: Some("engine.toml".to_string()),
        path: vec!["start engine".to_string(), "engine.toml".to_string()],
        metadata: {
            let mut metadata = ErrorMetadata::new();
            metadata.insert("component.name", "engine");
            metadata
        },
    };

    let ctx: OperationContext = stable.into();

    assert_eq!(ctx.action().as_deref(), Some("start engine"));
    assert_eq!(ctx.locator().as_deref(), Some("engine.toml"));
    assert_eq!(
        ctx.path(),
        &["start engine".to_string(), "engine.toml".to_string()]
    );
    assert_eq!(ctx.metadata().get_str("component.name"), Some("engine"));
}

#[cfg(feature = "serde_json")]
#[test]
fn test_to_stable_snapshot_json_uses_stable_export_shape() {
    let snapshot = ErrorSnapshot {
        reason: "system error".to_string(),
        detail: Some("outer detail".to_string()),
        position: None,
        path: Some("start engine".to_string()),
        context: vec![SnapshotContextFrame {
            target: Some("start engine".to_string()),
            action: Some("start engine".to_string()),
            locator: Some("engine.toml".to_string()),
            path: vec!["start engine".to_string()],
            metadata: ErrorMetadata::new(),
            fields: vec![("tenant".to_string(), "alpha".to_string())],
            result: crate::core::context::OperationResult::Fail,
        }],
        root_metadata: ErrorMetadata::new(),
        source_frames: vec![SnapshotSourceFrame {
            index: 0,
            message: "db unavailable".to_string(),
            display: Some("db unavailable".to_string()),
            type_name: Some("std::io::Error".to_string()),
            error_code: None,
            reason: None,
            path: Some("load config / read".to_string()),
            detail: None,
            metadata: ErrorMetadata::new(),
            is_root_cause: true,
        }],
        category: ErrorCategory::Sys,
        code: "sys.test_error".to_string(),
    };

    let json_value = snapshot.to_stable_snapshot_json().unwrap();

    assert_eq!(
        json_value,
        serde_json::to_value(snapshot.stable_export()).unwrap()
    );
    assert_eq!(
        json_value["schema_version"],
        serde_json::json!(STABLE_SNAPSHOT_SCHEMA_VERSION)
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
    assert!(json_value["source_frames"][0].get("display").is_none());
}

#[cfg(feature = "serde_json")]
#[test]
fn test_stable_snapshot_json_fields_match_schema_constants() {
    let snapshot = ErrorSnapshot {
        reason: "system error".to_string(),
        detail: Some("outer detail".to_string()),
        position: Some("src/main.rs:42".to_string()),
        path: Some("start engine".to_string()),
        context: vec![SnapshotContextFrame {
            target: Some("start engine".to_string()),
            action: None,
            locator: None,
            path: vec!["start engine".to_string()],
            metadata: ErrorMetadata::new(),
            fields: vec![("tenant".to_string(), "alpha".to_string())],
            result: crate::core::context::OperationResult::Fail,
        }],
        root_metadata: ErrorMetadata::new(),
        source_frames: vec![SnapshotSourceFrame {
            index: 0,
            message: "db unavailable".to_string(),
            display: Some("db unavailable".to_string()),
            type_name: Some("std::io::Error".to_string()),
            error_code: None,
            reason: None,
            path: Some("load config / read".to_string()),
            detail: Some("inner detail".to_string()),
            metadata: ErrorMetadata::new(),
            is_root_cause: true,
        }],
        category: ErrorCategory::Sys,
        code: "sys.test_error".to_string(),
    };

    let json_value = snapshot.to_stable_snapshot_json().unwrap();
    let top_level = json_value.as_object().unwrap();
    let context = json_value["context"][0].as_object().unwrap();
    let source_frame = json_value["source_frames"][0].as_object().unwrap();

    assert_eq!(
        sorted_keys(top_level),
        sorted_strings(&[
            "schema_version",
            "reason",
            "detail",
            "position",
            "path",
            "context",
            "root_metadata",
            "source_frames",
        ])
    );
    assert_eq!(
        sorted_keys(context),
        sorted_strings(&["target", "action", "locator", "path", "metadata"])
    );
    assert_eq!(
        sorted_keys(source_frame),
        sorted_strings(&[
            "index",
            "message",
            "error_code",
            "reason",
            "path",
            "detail",
            "metadata",
            "is_root_cause",
        ])
    );
}

#[cfg(feature = "serde_json")]
fn sorted_keys(map: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let mut keys = map.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    keys
}

#[cfg(feature = "serde_json")]
fn sorted_strings(values: &[&str]) -> Vec<String> {
    let mut values = values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    values.sort();
    values
}
