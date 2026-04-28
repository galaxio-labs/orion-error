use crate::{core::DomainReason, StructError};

use super::{
    context::OperationResult, report::DiagnosticReport, ErrorCategory, ErrorIdentityProvider,
    ErrorMetadata, OperationContext, SourceFrame,
};

pub const STABLE_SNAPSHOT_SCHEMA_VERSION: &str = "orion-error.snapshot.v2";
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotContextFrame {
    /// Stable root operation name.
    pub target: Option<String>,
    /// Action/phase captured by `doing(...)`.
    pub action: Option<String>,
    /// Resource/location captured by `at(...)`.
    pub locator: Option<String>,
    /// Stable path segments captured from runtime context.
    pub path: Vec<String>,
    /// Stable machine-readable metadata payload.
    pub metadata: ErrorMetadata,
    /// Compatibility projection of ad-hoc context key/value pairs.
    pub fields: Vec<(String, String)>,
    /// Compatibility projection of runtime scope result.
    pub result: OperationResult,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct StableSnapshotContextFrame {
    pub target: Option<String>,
    pub action: Option<String>,
    pub locator: Option<String>,
    pub path: Vec<String>,
    pub metadata: ErrorMetadata,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotSourceFrame {
    pub index: usize,
    /// Stable human-facing summary for diagnostics and snapshot assertions.
    pub message: String,
    /// Compatibility projection of formatted display output.
    pub display: Option<String>,
    /// Compatibility projection of best-effort runtime type name.
    pub type_name: Option<String>,
    pub error_code: Option<i32>,
    pub reason: Option<String>,
    pub want: Option<String>,
    pub path: Option<String>,
    pub detail: Option<String>,
    pub metadata: ErrorMetadata,
    pub is_root_cause: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct StableSnapshotSourceFrame {
    pub index: usize,
    pub message: String,
    pub error_code: Option<i32>,
    pub reason: Option<String>,
    pub want: Option<String>,
    pub path: Option<String>,
    pub detail: Option<String>,
    pub metadata: ErrorMetadata,
    pub is_root_cause: bool,
}

/// Stable machine-readable snapshot view derived from `StructError`.
///
/// This object is intentionally separate from runtime propagation semantics.
/// It carries exported diagnostic data, but does not implement `StdError`
/// or own any runtime source object handles.
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorSnapshot {
    pub reason: String,
    pub detail: Option<String>,
    pub position: Option<String>,
    pub want: Option<String>,
    pub path: Option<String>,
    pub category: ErrorCategory,
    pub code: String,
    pub context: Vec<SnapshotContextFrame>,
    pub root_metadata: ErrorMetadata,
    pub source_frames: Vec<SnapshotSourceFrame>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct StableErrorSnapshot {
    pub schema_version: &'static str,
    pub reason: String,
    pub detail: Option<String>,
    pub position: Option<String>,
    pub want: Option<String>,
    pub path: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub category: ErrorCategory,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub code: String,
    pub context: Vec<StableSnapshotContextFrame>,
    pub root_metadata: ErrorMetadata,
    pub source_frames: Vec<StableSnapshotSourceFrame>,
}

/// Identity-first snapshot view.
///
/// This view keeps `code` and `category` available for governance, testing,
/// policy decisions, and protocol projections without changing the stable
/// snapshot export contract.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorIdentity {
    pub code: String,
    pub category: ErrorCategory,
    pub reason: String,
    pub detail: Option<String>,
    pub position: Option<String>,
    pub want: Option<String>,
    pub path: Option<String>,
}

impl ErrorSnapshot {
    pub fn stable_context(&self) -> &[SnapshotContextFrame] {
        &self.context
    }

    pub fn stable_source_frames(&self) -> &[SnapshotSourceFrame] {
        &self.source_frames
    }

    pub fn root_source_frame(&self) -> Option<&SnapshotSourceFrame> {
        self.source_frames.iter().find(|frame| frame.is_root_cause)
    }

    pub fn stable_export(&self) -> StableErrorSnapshot {
        self.clone().into_stable_export()
    }

    pub fn into_stable_export(self) -> StableErrorSnapshot {
        StableErrorSnapshot {
            schema_version: STABLE_SNAPSHOT_SCHEMA_VERSION,
            reason: self.reason,
            detail: self.detail,
            position: self.position,
            want: self.want,
            path: self.path,
            category: self.category,
            code: self.code,
            context: self.context.into_iter().map(Into::into).collect(),
            root_metadata: self.root_metadata,
            source_frames: self.source_frames.into_iter().map(Into::into).collect(),
        }
    }

    #[cfg(feature = "serde_json")]
    pub fn to_stable_snapshot_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.stable_export())
    }

    pub fn report(&self) -> DiagnosticReport {
        self.clone().into_report()
    }

    pub fn into_report(self) -> DiagnosticReport {
        DiagnosticReport {
            reason: self.reason,
            detail: self.detail,
            position: self.position,
            want: self.want,
            path: self.path,
            category: self.category,
            code: self.code,
            context: self.context.into_iter().map(Into::into).collect(),
            root_metadata: self.root_metadata,
            source_frames: self.source_frames.into_iter().map(Into::into).collect(),
        }
    }
}

impl StableErrorSnapshot {
    pub fn report(&self) -> DiagnosticReport {
        DiagnosticReport {
            reason: self.reason.clone(),
            detail: self.detail.clone(),
            position: self.position.clone(),
            want: self.want.clone(),
            path: self.path.clone(),
            category: self.category,
            code: self.code.clone(),
            context: self.context.iter().cloned().map(Into::into).collect(),
            root_metadata: self.root_metadata.clone(),
            source_frames: self.source_frames.iter().cloned().map(Into::into).collect(),
        }
    }

    pub fn into_report(self) -> DiagnosticReport {
        DiagnosticReport {
            reason: self.reason,
            detail: self.detail,
            position: self.position,
            want: self.want,
            path: self.path,
            category: self.category,
            code: self.code,
            context: self.context.into_iter().map(Into::into).collect(),
            root_metadata: self.root_metadata,
            source_frames: self.source_frames.into_iter().map(Into::into).collect(),
        }
    }
}

impl SnapshotContextFrame {
    pub fn stable_export(&self) -> StableSnapshotContextFrame {
        self.clone().into()
    }
}

impl SnapshotSourceFrame {
    pub fn stable_export(&self) -> StableSnapshotSourceFrame {
        self.clone().into()
    }
}

impl From<SnapshotContextFrame> for StableSnapshotContextFrame {
    fn from(value: SnapshotContextFrame) -> Self {
        StableSnapshotContextFrame {
            target: value.target,
            action: value.action,
            locator: value.locator,
            path: value.path,
            metadata: value.metadata,
        }
    }
}

impl From<&SnapshotContextFrame> for StableSnapshotContextFrame {
    fn from(value: &SnapshotContextFrame) -> Self {
        value.clone().into()
    }
}

impl From<StableSnapshotContextFrame> for SnapshotContextFrame {
    fn from(value: StableSnapshotContextFrame) -> Self {
        SnapshotContextFrame {
            target: value.target,
            action: value.action,
            locator: value.locator,
            path: value.path,
            metadata: value.metadata,
            fields: Vec::new(),
            result: OperationResult::Fail,
        }
    }
}

impl From<&StableSnapshotContextFrame> for SnapshotContextFrame {
    fn from(value: &StableSnapshotContextFrame) -> Self {
        value.clone().into()
    }
}

impl From<SnapshotSourceFrame> for StableSnapshotSourceFrame {
    fn from(value: SnapshotSourceFrame) -> Self {
        StableSnapshotSourceFrame {
            index: value.index,
            message: value.message,
            error_code: value.error_code,
            reason: value.reason,
            want: value.want,
            path: value.path,
            detail: value.detail,
            metadata: value.metadata,
            is_root_cause: value.is_root_cause,
        }
    }
}

impl From<&SnapshotSourceFrame> for StableSnapshotSourceFrame {
    fn from(value: &SnapshotSourceFrame) -> Self {
        value.clone().into()
    }
}

impl From<StableSnapshotSourceFrame> for SnapshotSourceFrame {
    fn from(value: StableSnapshotSourceFrame) -> Self {
        SnapshotSourceFrame {
            index: value.index,
            message: value.message,
            display: None,
            type_name: None,
            error_code: value.error_code,
            reason: value.reason,
            want: value.want,
            path: value.path,
            detail: value.detail,
            metadata: value.metadata,
            is_root_cause: value.is_root_cause,
        }
    }
}

impl From<&StableSnapshotSourceFrame> for SnapshotSourceFrame {
    fn from(value: &StableSnapshotSourceFrame) -> Self {
        value.clone().into()
    }
}

impl From<OperationContext> for SnapshotContextFrame {
    fn from(value: OperationContext) -> Self {
        Self {
            target: value.target().clone(),
            action: value.action().clone(),
            locator: value.locator().clone(),
            path: value.normalized_path_segments(),
            metadata: value.metadata().clone(),
            fields: value.context().items.clone(),
            result: value.result().clone(),
        }
    }
}

impl From<SnapshotContextFrame> for OperationContext {
    fn from(value: SnapshotContextFrame) -> Self {
        let mut ctx = value
            .target
            .clone()
            .map(OperationContext::from_target)
            .unwrap_or_default();
        ctx.replace_target_for_report(value.target);
        ctx.replace_action_for_report(value.action);
        ctx.replace_locator_for_report(value.locator);
        ctx.replace_path_for_report(value.path);
        ctx.context_mut_for_report().items = value.fields;
        ctx.replace_metadata_for_report(value.metadata);
        match value.result {
            OperationResult::Suc => ctx.mark_suc(),
            OperationResult::Fail => {}
            OperationResult::Cancel => ctx.mark_cancel(),
        }
        ctx
    }
}

impl From<StableSnapshotContextFrame> for OperationContext {
    fn from(value: StableSnapshotContextFrame) -> Self {
        SnapshotContextFrame::from(value).into()
    }
}

impl From<&StableSnapshotContextFrame> for OperationContext {
    fn from(value: &StableSnapshotContextFrame) -> Self {
        value.clone().into()
    }
}

impl From<SourceFrame> for SnapshotSourceFrame {
    fn from(value: SourceFrame) -> Self {
        Self {
            index: value.index,
            message: value.message,
            display: value.display,
            type_name: value.type_name,
            error_code: value.error_code,
            reason: value.reason,
            want: value.want,
            path: value.path,
            detail: value.detail,
            metadata: value.metadata,
            is_root_cause: value.is_root_cause,
        }
    }
}

impl From<SnapshotSourceFrame> for SourceFrame {
    fn from(value: SnapshotSourceFrame) -> Self {
        Self {
            index: value.index,
            message: value.message,
            display: value.display,
            debug: String::new(),
            type_name: value.type_name,
            error_code: value.error_code,
            reason: value.reason,
            want: value.want,
            path: value.path,
            detail: value.detail,
            metadata: value.metadata,
            is_root_cause: value.is_root_cause,
        }
    }
}

impl From<StableSnapshotSourceFrame> for SourceFrame {
    fn from(value: StableSnapshotSourceFrame) -> Self {
        SnapshotSourceFrame::from(value).into()
    }
}

impl From<&StableSnapshotSourceFrame> for SourceFrame {
    fn from(value: &StableSnapshotSourceFrame) -> Self {
        value.clone().into()
    }
}

impl<T> StructError<T>
where
    T: DomainReason + ErrorIdentityProvider,
{
    pub fn snapshot(&self) -> ErrorSnapshot {
        ErrorSnapshot {
            reason: self.reason().to_string(),
            detail: self.detail().clone(),
            position: self.position().clone(),
            want: self.target_main(),
            path: self.target_path(),
            category: self.error_category(),
            code: self.stable_code().to_string(),
            context: self.contexts().iter().cloned().map(Into::into).collect(),
            root_metadata: self.context_metadata(),
            source_frames: self
                .source_frames()
                .iter()
                .cloned()
                .map(Into::into)
                .collect(),
        }
    }

    pub fn into_snapshot(self) -> ErrorSnapshot {
        self.snapshot()
    }

    pub fn identity_snapshot(&self) -> ErrorIdentity {
        ErrorIdentity {
            code: self.stable_code().to_string(),
            category: self.error_category(),
            reason: self.reason().to_string(),
            detail: self.detail().clone(),
            position: self.position().clone(),
            want: self.target_main(),
            path: self.target_path(),
        }
    }
}

impl<T> From<&StructError<T>> for ErrorSnapshot
where
    T: DomainReason + ErrorIdentityProvider,
{
    fn from(value: &StructError<T>) -> Self {
        value.snapshot()
    }
}

impl<T> From<StructError<T>> for ErrorSnapshot
where
    T: DomainReason + ErrorIdentityProvider,
{
    fn from(value: StructError<T>) -> Self {
        value.into_snapshot()
    }
}

impl<T> From<&StructError<T>> for StableErrorSnapshot
where
    T: DomainReason + ErrorIdentityProvider,
{
    fn from(value: &StructError<T>) -> Self {
        value.snapshot().into_stable_export()
    }
}

impl<T> From<StructError<T>> for StableErrorSnapshot
where
    T: DomainReason + ErrorIdentityProvider,
{
    fn from(value: StructError<T>) -> Self {
        value.into_snapshot().into_stable_export()
    }
}

impl From<&ErrorSnapshot> for StableErrorSnapshot {
    fn from(value: &ErrorSnapshot) -> Self {
        value.stable_export()
    }
}

impl From<ErrorSnapshot> for StableErrorSnapshot {
    fn from(value: ErrorSnapshot) -> Self {
        value.into_stable_export()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        core::{context::ContextRecord, DomainReason, ErrorMetadata, SourceFrame},
        ErrorCategory, ErrorCode, ErrorIdentityProvider, OperationContext, StructError, UvsReason,
    };

    use super::{
        DiagnosticReport, ErrorSnapshot, SnapshotContextFrame, SnapshotSourceFrame,
        StableErrorSnapshot, StableSnapshotContextFrame, StableSnapshotSourceFrame,
        STABLE_SNAPSHOT_SCHEMA_VERSION,
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
            .with_context(
                OperationContext::doing("start engine").with_meta("component.name", "engine"),
            )
            .with_struct_source(source);

        let snapshot = err.snapshot();

        assert_eq!(snapshot.reason, "system error");
        assert_eq!(snapshot.detail.as_deref(), Some("engine bootstrap failed"));
        assert_eq!(snapshot.position.as_deref(), Some("src/main.rs:42"));
        assert_eq!(snapshot.want.as_deref(), Some("start engine"));
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
        assert_eq!(identity.want.as_deref(), Some("start engine"));
        assert_eq!(identity.path.as_deref(), Some("start engine"));
    }

    #[test]
    fn test_snapshot_preserves_action_and_locator_context_fields() {
        let mut ctx = OperationContext::at("config.toml");
        ctx.with_doing("parse config");

        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_context(
                OperationContext::doing("load config").with_meta("component.name", "engine"),
            )
            .with_context(ctx);

        let snapshot = err.snapshot();

        assert_eq!(snapshot.context[0].action.as_deref(), Some("load config"));
        assert_eq!(snapshot.context[1].action.as_deref(), Some("parse config"));
        assert_eq!(snapshot.context[1].locator.as_deref(), Some("config.toml"));

        let report = snapshot.into_report();
        assert_eq!(report.context[1].action().as_deref(), Some("parse config"));
        assert_eq!(report.context[1].locator().as_deref(), Some("config.toml"));
    }

    #[test]
    fn test_snapshot_report_conversion_preserves_payload() {
        let snapshot = ErrorSnapshot {
            reason: "system error".to_string(),
            detail: Some("engine bootstrap failed".to_string()),
            position: Some("src/main.rs:42".to_string()),
            want: Some("start engine".to_string()),
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

        assert_eq!(report.reason, snapshot.reason);
        assert_eq!(report.detail, snapshot.detail);
        assert_eq!(report.position, snapshot.position);
        assert_eq!(report.want, snapshot.want);
        assert_eq!(report.path, snapshot.path);
        assert_eq!(
            report.context,
            snapshot
                .context
                .clone()
                .into_iter()
                .map(Into::into)
                .collect::<Vec<OperationContext>>()
        );
        assert_eq!(report.root_metadata, snapshot.root_metadata);
        assert_eq!(
            report.source_frames,
            snapshot
                .source_frames
                .clone()
                .into_iter()
                .map(Into::into)
                .collect::<Vec<SourceFrame>>()
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
            want: Some("start engine".to_string()),
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
                want: Some("load config".to_string()),
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

        assert_eq!(stable.schema_version, STABLE_SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(stable.reason, snapshot.reason);
        assert_eq!(stable.context[0].target.as_deref(), Some("start engine"));
        assert_eq!(stable.context[0].action.as_deref(), Some("start engine"));
        assert_eq!(stable.context[0].locator.as_deref(), Some("engine.toml"));
        assert_eq!(
            stable.context[0].path,
            vec!["start engine".to_string(), "engine.toml".to_string()]
        );
        assert_eq!(
            stable.source_frames[0].message,
            snapshot.source_frames[0].message
        );
        assert_eq!(
            stable.source_frames[0].metadata.get_str("config.kind"),
            Some("sink_defaults")
        );
    }

    #[test]
    fn test_snapshot_into_stable_export_matches_borrowed_stable_export() {
        let snapshot = ErrorSnapshot {
            reason: "system error".to_string(),
            detail: Some("outer detail".to_string()),
            position: Some("src/main.rs:42".to_string()),
            want: Some("start engine".to_string()),
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
                want: Some("load config".to_string()),
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
        assert_eq!(via_borrowed.schema_version, STABLE_SNAPSHOT_SCHEMA_VERSION);
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
    fn test_snapshot_frame_stable_from_matches_stable_export() {
        let context = SnapshotContextFrame {
            target: Some("start engine".to_string()),
            action: None,
            locator: None,
            path: vec!["start engine".to_string()],
            metadata: ErrorMetadata::new(),
            fields: vec![("tenant".to_string(), "alpha".to_string())],
            result: crate::core::context::OperationResult::Fail,
        };
        let source = SnapshotSourceFrame {
            index: 0,
            message: "db unavailable".to_string(),
            display: Some("db unavailable".to_string()),
            type_name: Some("std::io::Error".to_string()),
            error_code: None,
            reason: None,
            want: Some("load config".to_string()),
            path: Some("load config / read".to_string()),
            detail: Some("inner detail".to_string()),
            metadata: ErrorMetadata::new(),
            is_root_cause: true,
        };

        assert_eq!(
            StableSnapshotContextFrame::from(&context),
            context.stable_export()
        );
        assert_eq!(
            StableSnapshotContextFrame::from(context.clone()),
            context.stable_export()
        );
        assert_eq!(
            StableSnapshotSourceFrame::from(&source),
            source.stable_export()
        );
        assert_eq!(
            StableSnapshotSourceFrame::from(source.clone()),
            source.stable_export()
        );
    }

    #[test]
    fn test_stable_snapshot_into_report_matches_report() {
        let stable = StableErrorSnapshot {
            schema_version: STABLE_SNAPSHOT_SCHEMA_VERSION,
            reason: "system error".to_string(),
            detail: Some("outer detail".to_string()),
            position: None,
            want: Some("start engine".to_string()),
            path: Some("start engine".to_string()),
            context: vec![StableSnapshotContextFrame {
                target: Some("start engine".to_string()),
                action: None,
                locator: None,
                path: vec!["start engine".to_string()],
                metadata: ErrorMetadata::new(),
            }],
            root_metadata: ErrorMetadata::new(),
            source_frames: vec![StableSnapshotSourceFrame {
                index: 0,
                message: "db unavailable".to_string(),
                error_code: None,
                reason: None,
                want: Some("load config".to_string()),
                path: Some("load config / read".to_string()),
                detail: Some("inner detail".to_string()),
                metadata: ErrorMetadata::new(),
                is_root_cause: true,
            }],
            category: ErrorCategory::Sys,
            code: "sys.test_error".to_string(),
        };

        let via_method = stable.report();
        let via_owned = stable.clone().into_report();

        assert_eq!(via_owned, via_method);
    }

    #[test]
    fn test_stable_frame_to_compat_frame_defaults_compat_fields() {
        let context = StableSnapshotContextFrame {
            target: Some("start engine".to_string()),
            action: None,
            locator: None,
            path: vec!["start engine".to_string()],
            metadata: ErrorMetadata::new(),
        };
        let source = StableSnapshotSourceFrame {
            index: 0,
            message: "db unavailable".to_string(),
            error_code: None,
            reason: None,
            want: Some("load config".to_string()),
            path: Some("load config / read".to_string()),
            detail: Some("inner detail".to_string()),
            metadata: ErrorMetadata::new(),
            is_root_cause: true,
        };

        let compat_context = SnapshotContextFrame::from(&context);
        let compat_source = SnapshotSourceFrame::from(&source);

        assert_eq!(compat_context.target, context.target);
        assert_eq!(compat_context.path, context.path);
        assert_eq!(compat_context.fields, Vec::<(String, String)>::new());
        assert_eq!(
            compat_context.result,
            crate::core::context::OperationResult::Fail
        );
        assert_eq!(compat_source.message, source.message);
        assert_eq!(compat_source.display, None);
        assert_eq!(compat_source.type_name, None);
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
        assert_eq!(roundtrip.target().as_deref(), Some("start engine"));
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
            want: Some("load config".to_string()),
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

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_to_stable_snapshot_json_uses_stable_export_shape() {
        let snapshot = ErrorSnapshot {
            reason: "system error".to_string(),
            detail: Some("outer detail".to_string()),
            position: None,
            want: Some("start engine".to_string()),
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
                want: Some("load config".to_string()),
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
            want: Some("start engine".to_string()),
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
                want: Some("load config".to_string()),
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
                "want",
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
                "want",
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
}

#[cfg(doc)]
mod stable_snapshot_compile_fail_docs {
    //! ```compile_fail
    //! use orion_error::{StableErrorSnapshot, ErrorSnapshot};
    //!
    //! fn must_not_compile(stable: StableErrorSnapshot) -> ErrorSnapshot {
    //!     stable.into()
    //! }
    //! ```
}
