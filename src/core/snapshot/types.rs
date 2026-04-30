pub const STABLE_SNAPSHOT_SCHEMA_VERSION: &str = "orion-error.snapshot.v3";
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotContextFrame {
    /// Compatibility projection of the frame's root target value.
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
struct StableSnapshotContextFrame {
    target: Option<String>,
    action: Option<String>,
    locator: Option<String>,
    path: Vec<String>,
    metadata: ErrorMetadata,
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
    pub path: Option<String>,
    pub detail: Option<String>,
    pub metadata: ErrorMetadata,
    pub is_root_cause: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
struct StableSnapshotSourceFrame {
    index: usize,
    message: String,
    error_code: Option<i32>,
    reason: Option<String>,
    path: Option<String>,
    detail: Option<String>,
    metadata: ErrorMetadata,
    is_root_cause: bool,
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
    /// Stable exported operation path projection.
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
    schema_version: &'static str,
    reason: String,
    detail: Option<String>,
    position: Option<String>,
    /// Stable exported operation path projection.
    path: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip))]
    category: ErrorCategory,
    #[cfg_attr(feature = "serde", serde(skip))]
    code: String,
    context: Vec<StableSnapshotContextFrame>,
    root_metadata: ErrorMetadata,
    source_frames: Vec<StableSnapshotSourceFrame>,
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
    /// Stable exported operation path projection.
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
            path: self.path,
            category: self.category,
            code: self.code,
            context: self.context.into_iter().map(Into::into).collect(),
            root_metadata: self.root_metadata,
            source_frames: self.source_frames.into_iter().map(Into::into).collect(),
        }
    }

    /// Serialize to stable snapshot JSON.
    ///
    /// Requires feature: `"serde_json"`.
    #[cfg(feature = "serde_json")]
    pub fn to_stable_snapshot_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.stable_export())
    }

    pub fn report(&self) -> DiagnosticReport {
        self.clone().into_report()
    }

    pub fn into_report(self) -> DiagnosticReport {
        DiagnosticReport::from_parts(
            self.reason,
            self.detail,
            self.position,
            Arc::new(self.context.into_iter().map(Into::into).collect()),
        )
    }
}

impl StableErrorSnapshot {
    pub fn schema_version(&self) -> &'static str {
        self.schema_version
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    pub fn position(&self) -> Option<&str> {
        self.position.as_deref()
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub fn category(&self) -> ErrorCategory {
        self.category
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn root_metadata(&self) -> &ErrorMetadata {
        &self.root_metadata
    }

    pub fn report(&self) -> DiagnosticReport {
        DiagnosticReport::from_parts(
            self.reason.clone(),
            self.detail.clone(),
            self.position.clone(),
            Arc::new(self.context.iter().cloned().map(Into::into).collect()),
        )
    }

    pub fn into_report(self) -> DiagnosticReport {
        DiagnosticReport::from_parts(
            self.reason,
            self.detail,
            self.position,
            Arc::new(self.context.into_iter().map(Into::into).collect()),
        )
    }
}

impl From<SnapshotContextFrame> for StableSnapshotContextFrame {
    /// Strip compat/projection fields (`fields`, `result`) that are not stable
    /// across serialization boundaries. The stable snapshot format only carries
    /// path-related context; ad-hoc KV pairs and operation result are runtime-only.
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
    /// Reconstitute a runtime snapshot frame from stable data.
    ///
    /// `fields` and `result` cannot be recovered — they are intentionally excluded
    /// from the stable format because ad-hoc KV pairs lose meaning after
    /// serialization and the result is always `Fail` at snapshot time.
    /// Callers that need full-fidelity context should use `ErrorSnapshot` directly
    /// instead of going through the stable export/import round-trip.
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
            target: value.compat_target(),
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
        OperationContext::from_projection_parts(
            value.target,
            value.action,
            value.locator,
            value.path,
            value.fields,
            value.metadata,
            value.result,
        )
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
