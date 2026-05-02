use std::{error::Error as StdError, fmt};

use crate::core::error::source_chain::InternalSourcePayload;
use crate::core::error::std_bridge::internal_into_std_bridge;
use crate::{
    core::context::CallContext,
    reason::{DomainReason, ErrorCode},
    traits::ErrorWith,
    OperationContext, UvsReason,
};

use super::*;
use derive_more::From;
use thiserror::Error;

// Define a simple DomainReason for testing
#[derive(Debug, Clone, PartialEq, Error, From)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
enum TestDomainReason {
    #[error("test error")]
    TestError,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl ErrorCode for TestDomainReason {
    fn error_code(&self) -> i32 {
        match self {
            TestDomainReason::TestError => 1001,
            TestDomainReason::Uvs(uvs_reason) => uvs_reason.error_code(),
        }
    }
}

impl DomainReason for TestDomainReason {}

#[derive(Debug)]
struct InnerError;

impl fmt::Display for InnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "inner source")
    }
}

impl StdError for InnerError {}

#[derive(Debug)]
struct OuterError {
    source: InnerError,
}

impl fmt::Display for OuterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "outer source")
    }
}

impl StdError for OuterError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.source)
    }
}

#[test]
fn test_struct_error_serialization() {
    // Create a context
    let mut context = CallContext::default();
    context
        .items
        .push(("key1".to_string(), "value1".to_string()));
    context
        .items
        .push(("key2".to_string(), "value2".to_string()));

    // Create a StructError
    let error = StructError::new(
        TestDomainReason::TestError,
        Some("Detailed error description".to_string()),
        Some("file.rs:10:5".to_string()),
        vec![OperationContext::from(context)],
    );

    // Serialize to JSON
    let json_value = serde_json::to_value(&error).unwrap();
    println!("{json_value:#}");
    assert!(json_value.get("reason").is_some());
    assert!(json_value.get("detail").is_some());
    assert!(json_value.get("position").is_some());
    assert!(json_value.get("context").is_some());
    assert_eq!(json_value.get("path"), Some(&serde_json::Value::Null));
    assert_eq!(
        json_value.get("source_frames"),
        Some(&serde_json::Value::Array(Vec::new()))
    );
    assert_eq!(
        json_value.get("source_message"),
        Some(&serde_json::Value::Null)
    );
    assert_eq!(
        json_value.get("source_chain"),
        Some(&serde_json::Value::Array(Vec::new()))
    );
}

#[test]
fn test_struct_error_source_tracking() {
    let error = StructError::builder(TestDomainReason::TestError)
        .detail("high-level detail")
        .source_std(OuterError { source: InnerError })
        .finish();

    assert_eq!(error.source_ref().unwrap().to_string(), "outer source");
    assert_eq!(error.root_cause().unwrap().to_string(), "inner source");
    assert_eq!(error.root_cause_frame().unwrap().message, "inner source");

}

#[test]
fn test_struct_error_uses_outer_action_and_full_path() {
    let mut outer = OperationContext::doing("place_order");
    outer.with_doing("read_order_payload");
    outer.with_doing("parse_order");
    outer.record("order_id", "42");

    let error = StructError::from(TestDomainReason::TestError).with_context(outer);

    assert_eq!(error.action_main().as_deref(), Some("place_order"));
    assert_eq!(error.action_main().as_deref(), Some("place_order"));
    assert_eq!(
        error.target_path().as_deref(),
        Some("place_order / read_order_payload / parse_order")
    );
    assert_eq!(
        error.path_segments(),
        vec![
            "place_order".to_string(),
            "read_order_payload".to_string(),
            "parse_order".to_string()
        ]
    );

    let display_output = format!("{error}");
    assert!(display_output.contains("-> Call: place_order / read_order_payload / parse_order"));
}

#[test]
fn test_errorwith_doing_and_at_write_structured_context_semantics() {
    let error = StructError::from(TestDomainReason::TestError)
        .doing("parse config")
        .at("config.toml");

    assert_eq!(error.action_main().as_deref(), Some("parse config"));
    assert_eq!(error.locator_main().as_deref(), Some("config.toml"));
    assert_eq!(error.action_main().as_deref(), Some("parse config"));
    assert_eq!(
        error.target_path().as_deref(),
        Some("parse config / config.toml")
    );
    assert_eq!(
        error.contexts()[0].action().as_deref(),
        Some("parse config")
    );
    assert_eq!(
        error.contexts()[1].locator().as_deref(),
        Some("config.toml")
    );
}

#[test]
fn test_errorwith_doing_and_at_match_single_context_compat_path_order() {
    let split = StructError::from(TestDomainReason::TestError)
        .doing("parse config")
        .at("config.toml");

    let mut combined_ctx = OperationContext::doing("parse config");
    combined_ctx.with_at("config.toml");
    let combined = StructError::from(TestDomainReason::TestError).with_context(combined_ctx);

    assert_eq!(split.target_path(), combined.target_path());
    assert_eq!(split.path_segments(), combined.path_segments());
}

#[test]
fn test_errorwith_multiple_at_segments_preserve_locator_chain() {
    let error = StructError::from(TestDomainReason::TestError)
        .doing("parse config")
        .at("tenant-a")
        .at("config.toml");

    assert_eq!(
        error.target_path().as_deref(),
        Some("parse config / tenant-a / config.toml")
    );
    assert_eq!(
        error.path_segments(),
        vec![
            "parse config".to_string(),
            "tenant-a".to_string(),
            "config.toml".to_string()
        ]
    );
}

#[test]
fn test_struct_error_display_chain() {
    let error = StructError::builder(TestDomainReason::TestError)
        .detail("high-level detail")
        .source_std(OuterError { source: InnerError })
        .finish();

    assert_eq!(
        error.source_chain(),
        vec!["outer source".to_string(), "inner source".to_string()]
    );
    assert_eq!(error.source_frames().len(), 2);
    assert_eq!(error.source_frames()[0].index, 0);
    assert_eq!(error.source_frames()[0].message, "outer source");
    assert_eq!(error.source_frames()[1].index, 1);
    assert_eq!(error.source_frames()[1].message, "inner source");
    assert!(error.source_frames()[1].is_root_cause);
    assert_eq!(
        error.source_frames()[0].type_name.as_deref(),
        Some(concat!(module_path!(), "::OuterError"))
    );
    assert_eq!(error.source_frames()[1].type_name, None);

    let display_chain = error.display_chain();
    assert!(display_chain.contains("-> Caused by:"));
    assert!(display_chain.contains("1. outer source"));
    assert!(display_chain.contains("2. inner source"));
}

#[test]
fn test_struct_error_serialization_includes_source_summary() {
    let error = StructError::builder(TestDomainReason::TestError)
        .detail("high-level detail")
        .source_std(OuterError { source: InnerError })
        .finish();

    let json_value = serde_json::to_value(&error).unwrap();
    assert_eq!(
        json_value.get("source_message"),
        Some(&serde_json::Value::String("outer source".to_string()))
    );
    assert_eq!(
        json_value.get("source_chain"),
        Some(&serde_json::json!(["outer source", "inner source"]))
    );
    assert_eq!(
        json_value.get("source_frames"),
        Some(&serde_json::json!([
            {
                "index": 0,
                "message": "outer source",
                "type_name": concat!(module_path!(), "::OuterError"),
                "is_root_cause": false
            },
            {
                "index": 1,
                "message": "inner source",
                "is_root_cause": true
            }
        ]))
    );
    assert!(!json_value["source_frames"][0]
        .as_object()
        .unwrap()
        .contains_key("debug"));
}

#[test]
fn test_struct_error_serialization_includes_path() {
    let mut outer = OperationContext::doing("place_order");
    outer.with_doing("read_order_payload");
    outer.record("order_id", "42");

    let error = StructError::from(TestDomainReason::TestError).with_context(outer);

    let json_value = serde_json::to_value(&error).unwrap();
    assert_eq!(
        json_value.get("path"),
        Some(&serde_json::Value::String(
            "place_order / read_order_payload".to_string()
        ))
    );
}

#[test]
fn test_struct_error_context_metadata_prefers_inner_context() {
    let inner = OperationContext::doing("load sink defaults")
        .with_meta("config.kind", "sink_defaults")
        .with_meta("file.path", "/tmp/defaults.toml");
    let outer = OperationContext::doing("load infra sink routes")
        .with_meta("config.kind", "sink_route")
        .with_meta("config.group", "infra");

    let error = StructError::from(TestDomainReason::TestError)
        .with_context(inner)
        .with_context(outer);

    let metadata = error.context_metadata();
    assert_eq!(metadata.get_str("config.kind"), Some("sink_defaults"));
    assert_eq!(metadata.get_str("file.path"), Some("/tmp/defaults.toml"));
    assert_eq!(metadata.get_str("config.group"), Some("infra"));
}

#[test]
fn test_with_struct_source_preserves_source_context_metadata() {
    let source = StructError::from(TestDomainReason::TestError).with_context(
        OperationContext::doing("load sink defaults").with_meta("config.kind", "sink_defaults"),
    );

    let error = StructError::from(TestDomainReason::Uvs(UvsReason::system_error()))
        .with_struct_source(source);

    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Struct));
    assert_eq!(
        error.source_frames()[0].metadata.get_str("config.kind"),
        Some("sink_defaults")
    );
}

#[test]
fn test_with_std_source_marks_internal_source_kind() {
    let error = StructError::from(TestDomainReason::TestError)
        .with_std_source(std::io::Error::other("disk offline"));

    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Std));
}

#[test]
fn test_with_source_auto_routes_std_source_kind() {
    let error = StructError::from(TestDomainReason::TestError)
        .with_source(std::io::Error::other("disk offline"));

    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Std));
}

#[test]
fn test_with_source_auto_routes_struct_source_kind() {
    let source = StructError::from(TestDomainReason::TestError).with_context(
        OperationContext::doing("load sink defaults").with_meta("config.kind", "sink_defaults"),
    );

    let error =
        StructError::from(TestDomainReason::Uvs(UvsReason::system_error())).with_source(source);

    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Struct));
    assert_eq!(
        error.source_frames()[0].metadata.get_str("config.kind"),
        Some("sink_defaults")
    );
}

#[test]
fn test_builder_source_struct_preserves_source_context_metadata() {
    let source = StructError::from(TestDomainReason::TestError).with_context(
        OperationContext::doing("load sink defaults").with_meta("config.kind", "sink_defaults"),
    );

    let error = StructError::builder(TestDomainReason::Uvs(UvsReason::system_error()))
        .source_struct(source)
        .finish();

    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Struct));
    assert_eq!(
        error.source_frames()[0].metadata.get_str("config.kind"),
        Some("sink_defaults")
    );
}

#[test]
fn test_builder_source_std_marks_internal_source_kind() {
    let error = StructError::builder(TestDomainReason::TestError)
        .source_std(std::io::Error::other("disk offline"))
        .finish();

    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Std));
}

#[test]
fn test_builder_source_auto_routes_std_source_kind() {
    let error = StructError::builder(TestDomainReason::TestError)
        .source(std::io::Error::other("disk offline"))
        .finish();

    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Std));
}

#[test]
fn test_builder_source_auto_routes_struct_source_kind() {
    let source = StructError::from(TestDomainReason::TestError).with_context(
        OperationContext::doing("load sink defaults").with_meta("config.kind", "sink_defaults"),
    );

    let error = StructError::builder(TestDomainReason::Uvs(UvsReason::system_error()))
        .source(source)
        .finish();

    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Struct));
    assert_eq!(
        error.source_frames()[0].metadata.get_str("config.kind"),
        Some("sink_defaults")
    );
}

#[test]
fn test_internal_source_payload_uses_distinct_std_and_struct_variants() {
    let std_error = StructError::from(TestDomainReason::TestError)
        .with_std_source(std::io::Error::other("disk offline"));
    assert!(matches!(
        std_error.imp.source_payload.as_ref().unwrap(),
        InternalSourcePayload::Std { .. }
    ));

    let source = StructError::from(TestDomainReason::TestError)
        .with_detail("inner detail")
        .with_context(
            OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
        );
    let struct_error = StructError::from(TestDomainReason::Uvs(UvsReason::system_error()))
        .with_struct_source(source);
    assert!(matches!(
        struct_error.imp.source_payload.as_ref().unwrap(),
        InternalSourcePayload::Struct { .. }
    ));
}

#[test]
fn test_public_source_payload_ref_exposes_std_source_read_only() {
    let error = StructError::from(TestDomainReason::TestError)
        .with_std_source(std::io::Error::other("disk offline"));

    let payload = error.source_payload().expect("expected source payload");

    assert_eq!(payload.kind(), SourcePayloadKind::Std);
    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Std));
    assert_eq!(payload.source().to_string(), "disk offline");
    assert_eq!(payload.root_cause().to_string(), "disk offline");
    assert_eq!(payload.frames()[0].message, "disk offline");
    assert_eq!(payload.source_chain(), vec!["disk offline".to_string()]);
}

#[test]
fn test_public_source_payload_ref_exposes_struct_source_read_only() {
    let source = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));
    let error = StructError::from(TestDomainReason::Uvs(UvsReason::system_error()))
        .with_struct_source(source);

    let payload = error.source_payload().expect("expected source payload");

    assert_eq!(payload.kind(), SourcePayloadKind::Struct);
    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Struct));
    assert_eq!(
        payload.source().to_string(),
        "test error\n  -> Info: repo layer failed"
    );
    assert_eq!(payload.root_cause().to_string(), "db unavailable");
    assert_eq!(
        payload.source_chain(),
        vec!["test error".to_string(), "db unavailable".to_string()]
    );
    assert_eq!(payload.frames()[0].reason.as_deref(), Some("test error"));
    assert_eq!(
        payload.frames()[0].detail.as_deref(),
        Some("repo layer failed")
    );
}

#[test]
fn test_with_source_routes_std_source_payload() {
    let error = StructError::from(TestDomainReason::TestError)
        .with_source(std::io::Error::other("disk offline"));

    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Std));
    assert_eq!(error.source_ref().unwrap().to_string(), "disk offline");
    assert_eq!(error.source_frames()[0].message, "disk offline");
}

#[test]
fn test_with_source_routes_struct_source_payload() {
    let source = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));
    let error =
        StructError::from(TestDomainReason::Uvs(UvsReason::system_error())).with_source(source);

    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Struct));
    assert_eq!(
        error.source_frames()[0].reason.as_deref(),
        Some("test error")
    );
    assert_eq!(error.root_cause().unwrap().to_string(), "db unavailable");
}

#[test]
fn test_builder_source_routes_std_source_payload() {
    let error = StructError::builder(TestDomainReason::TestError)
        .source(std::io::Error::other("disk offline"))
        .finish();

    assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Std));
    assert_eq!(error.source_ref().unwrap().to_string(), "disk offline");
}

#[test]
fn test_internal_into_std_bridge_preserves_display_and_source_chain() {
    let structured = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));

    let bridged = internal_into_std_bridge(structured);

    assert_eq!(
        bridged.to_string(),
        "test error\n  -> Info: repo layer failed"
    );
    assert_eq!(
        StdError::source(bridged.as_ref()).unwrap().to_string(),
        "db unavailable"
    );
}

#[test]
fn test_internal_as_std_bridge_matches_struct_error_std_view() {
    let structured = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));

    let bridged = structured.as_std();
    let bridged_std: &dyn StdError = &bridged;

    assert_eq!(bridged.to_string(), structured.to_string());
    assert_eq!(
        StdError::source(bridged_std).unwrap().to_string(),
        "db unavailable"
    );
}

#[test]
fn test_public_owned_std_bridge_preserves_display_source_and_inner() {
    let structured = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));

    let bridged = structured.clone().into_std();

    assert_eq!(bridged.to_string(), structured.to_string());
    assert_eq!(
        StdError::source(&bridged).unwrap().to_string(),
        "db unavailable"
    );
    assert_eq!(bridged.inner().detail(), structured.detail());
    assert_eq!(bridged.into_struct(), structured);
}

#[test]
fn test_owned_std_bridge_from_struct_error_matches_into_std() {
    let structured = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));

    let via_from = OwnedStdStructError::from(structured.clone());
    let via_method = structured.into_std();

    assert_eq!(via_from.to_string(), via_method.to_string());
    assert_eq!(
        StdError::source(&via_from).unwrap().to_string(),
        StdError::source(&via_method).unwrap().to_string()
    );
}

#[test]
fn test_owned_std_bridge_into_boxed_preserves_display_and_source() {
    let structured = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));

    let boxed = structured.clone().into_std().into_boxed();

    assert_eq!(boxed.to_string(), structured.to_string());
    assert_eq!(boxed.source().unwrap().to_string(), "db unavailable");
}

#[test]
fn test_struct_error_into_boxed_std_preserves_display_and_source() {
    let structured = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));

    let boxed = structured.clone().into_boxed_std();

    assert_eq!(boxed.to_string(), structured.to_string());
    assert_eq!(boxed.source().unwrap().to_string(), "db unavailable");
}

#[test]
fn test_dyn_std_bridge_preserves_display_source_and_structured_frames() {
    let structured = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));

    let bridged = structured.clone().into_dyn_std();

    assert_eq!(bridged.to_string(), structured.to_string());
    assert_eq!(
        StdError::source(&bridged).unwrap().to_string(),
        "db unavailable"
    );
    assert_eq!(bridged.source_frames()[0].message, "test error");
    assert_eq!(
        bridged.source_frames()[0].detail.as_deref(),
        structured.detail().as_deref()
    );
}

#[test]
fn test_dyn_std_bridge_from_owned_std_bridge_matches_struct_error_conversion() {
    let structured = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));

    let via_owned = OwnedDynStdStructError::from(structured.clone().into_std());
    let via_struct = structured.into_dyn_std();

    assert_eq!(via_owned.to_string(), via_struct.to_string());
    assert_eq!(
        StdError::source(&via_owned).unwrap().to_string(),
        StdError::source(&via_struct).unwrap().to_string()
    );
    assert_eq!(via_owned.source_frames(), via_struct.source_frames());
}

#[test]
fn test_dyn_std_bridge_into_boxed_preserves_display_and_source() {
    let structured = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));

    let boxed = structured.clone().into_dyn_std().into_boxed();

    assert_eq!(boxed.to_string(), structured.to_string());
    assert_eq!(boxed.source().unwrap().to_string(), "db unavailable");
}

#[test]
fn test_public_std_ref_bridge_preserves_display_and_source() {
    let structured = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));

    let bridged = structured.as_std();
    let bridged_std: &dyn StdError = &bridged;

    assert_eq!(bridged.to_string(), structured.to_string());
    assert_eq!(
        StdError::source(bridged_std).unwrap().to_string(),
        "db unavailable"
    );
}

#[test]
fn test_std_ref_bridge_from_struct_error_matches_as_std_and_exposes_inner() {
    let structured = StructError::from(TestDomainReason::TestError)
        .with_detail("repo layer failed")
        .with_std_source(std::io::Error::other("db unavailable"));

    let via_from = StdStructRef::from(&structured);
    let via_method = structured.as_std();

    assert_eq!(via_from.to_string(), via_method.to_string());
    assert_eq!(
        StdError::source(&via_from).unwrap().to_string(),
        StdError::source(&via_method).unwrap().to_string()
    );
    assert_eq!(via_from.inner().detail(), structured.detail());
    assert_eq!(via_from.inner().reason(), structured.reason());
}

#[test]
fn test_with_struct_source_keeps_nested_source_frame_metadata() {
    let leaf = StructError::from(TestDomainReason::TestError).with_context(
        OperationContext::doing("parse route")
            .with_meta("config.kind", "sink_route")
            .with_meta("config.group", "infra"),
    );
    let middle = StructError::from(TestDomainReason::Uvs(UvsReason::validation_error()))
        .with_struct_source(leaf);

    let error = StructError::from(TestDomainReason::Uvs(UvsReason::system_error()))
        .with_struct_source(middle);

    assert_eq!(
        error.source_frames()[1].metadata.get_str("config.kind"),
        Some("sink_route")
    );
    assert_eq!(
        error.source_frames()[1].metadata.get_str("config.group"),
        Some("infra")
    );
}

#[test]
fn test_root_and_source_metadata_can_be_read_separately() {
    let source = StructError::from(TestDomainReason::TestError).with_context(
        OperationContext::doing("load sink defaults").with_meta("config.kind", "sink_defaults"),
    );

    let error = StructError::from(TestDomainReason::Uvs(UvsReason::system_error()))
        .with_context(OperationContext::doing("start engine").with_meta("component.name", "engine"))
        .with_struct_source(source);

    assert_eq!(
        error.context_metadata().get_str("component.name"),
        Some("engine")
    );
    assert_eq!(
        error.source_frames()[0].metadata.get_str("config.kind"),
        Some("sink_defaults")
    );
}

#[test]
fn test_display_does_not_include_metadata() {
    let error = StructError::from(TestDomainReason::TestError).with_context(
        OperationContext::doing("load sink defaults")
            .with_meta("config.kind", "sink_defaults")
            .with_meta("config.group", "infra"),
    );

    let display_output = format!("{error}");
    assert!(!display_output.contains("config.kind"));
    assert!(!display_output.contains("sink_defaults"));
}

#[test]
fn test_inherent_with_context_accepts_call_context() {
    let mut ctx = CallContext::default();
    ctx.items.push(("tenant".to_string(), "acme".to_string()));

    let error = StructError::from(TestDomainReason::TestError).with_context(ctx);

    assert_eq!(error.contexts().len(), 1);
    assert_eq!(error.contexts()[0].context().items.len(), 1);
    assert_eq!(error.contexts()[0].context().items[0].0, "tenant");
    assert_eq!(error.contexts()[0].context().items[0].1, "acme");
}

// -----------------------------------------------------------------------
// path_segments edge cases
// -----------------------------------------------------------------------

#[test]
fn test_path_segments_empty_context_chain() {
    let error = StructError::from(TestDomainReason::TestError);
    assert!(error.path_segments().is_empty());
    assert!(error.target_path().is_none());
}

#[test]
fn test_path_segments_only_at_no_doing() {
    let error = StructError::from(TestDomainReason::TestError).at("config.toml");

    assert_eq!(error.path_segments(), vec!["config.toml"]);
    assert_eq!(error.target_path().as_deref(), Some("config.toml"));
}

#[test]
fn test_path_segments_multiple_at_no_doing() {
    let error = StructError::from(TestDomainReason::TestError)
        .at("tenant-a")
        .at("config.toml");

    assert_eq!(error.path_segments(), vec!["tenant-a", "config.toml"]);
    assert_eq!(
        error.target_path().as_deref(),
        Some("tenant-a / config.toml")
    );
}

#[test]
fn test_path_segments_repeated_segment_is_deduplicated() {
    let error = StructError::from(TestDomainReason::TestError)
        .doing("load config")
        .at("load config");

    assert_eq!(error.path_segments(), vec!["load config"]);
}

#[test]
fn test_path_segments_repeated_segment_in_merge() {
    let mut inner = OperationContext::doing("service");
    inner.with_doing("parse");
    let mut outer = OperationContext::doing("service");
    outer.with_doing("validate");

    let error = StructError::from(TestDomainReason::TestError)
        .with_context(inner)
        .with_context(outer);

    // Dedup only removes adjacent duplicates; "service" appears twice
    // because it's the root of two separate context layers.
    assert_eq!(
        error.path_segments(),
        vec!["service", "validate", "service", "parse"]
    );
}

#[test]
fn test_path_segments_at_only_inner_outer_mixed() {
    let inner = OperationContext::doing("inner task");
    let mut outer = OperationContext::doing("outer task");
    outer.with_at("locator-a");

    let error = StructError::from(TestDomainReason::TestError)
        .with_context(inner)
        .at("locator-b")
        .with_context(outer);

    assert_eq!(
        error.path_segments(),
        vec!["outer task", "locator-a", "inner task", "locator-b"]
    );
}

#[test]
fn test_path_segments_many_locators_are_collected() {
    let error = StructError::from(TestDomainReason::TestError)
        .doing("process")
        .at("a")
        .at("b")
        .at("c")
        .at("d");

    assert_eq!(error.path_segments(), vec!["process", "a", "b", "c", "d"]);
    assert_eq!(
        error.target_path().as_deref(),
        Some("process / a / b / c / d")
    );
}

#[test]
fn test_path_segments_locator_main_is_first_when_no_action() {
    let error = StructError::from(TestDomainReason::TestError)
        .at("engine.toml")
        .doing("start engine");

    assert_eq!(error.path_segments(), vec!["start engine", "engine.toml"]);
}
