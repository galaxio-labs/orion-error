use crate::{core::DomainReason, StructError};
use crate::reason::ErrorCategory;

use super::{
    snapshot::{ErrorIdentity, ErrorSnapshot, StableErrorSnapshot},
    ErrorIdentityProvider, ErrorMetadata, MetadataValue, OperationContext, SourceFrame,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticReport {
    pub reason: String,
    pub detail: Option<String>,
    pub position: Option<String>,
    pub context: Vec<OperationContext>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReportProjectionParts {
    pub want: Option<String>,
    pub path: Option<String>,
    pub root_metadata: ErrorMetadata,
    pub source_frames: Vec<SourceFrame>,
}

impl ReportProjectionParts {
    fn from_error<T: DomainReason>(err: &StructError<T>) -> Self {
        Self {
            want: err.target_main(),
            path: err.target_path(),
            root_metadata: err.context_metadata(),
            source_frames: err.source_frames().to_vec(),
        }
    }

    fn from_identity_skeleton(identity: &ErrorIdentity) -> Self {
        Self {
            want: identity.want.clone(),
            path: identity.path.clone(),
            root_metadata: ErrorMetadata::new(),
            source_frames: Vec::new(),
        }
    }

    fn redacted(&self, policy: &impl RedactPolicy) -> Self {
        Self {
            want: redact_optional_text(Some("want"), self.want.as_deref(), policy),
            path: redact_optional_text(Some("path"), self.path.as_deref(), policy),
            root_metadata: redact_metadata(&self.root_metadata, policy),
            source_frames: self
                .source_frames
                .iter()
                .cloned()
                .map(|frame| redact_frame(frame, policy))
                .collect(),
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "lowercase")
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Internal,
}

impl Visibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExposureDecision {
    pub http_status: u16,
    pub visibility: Visibility,
    pub default_hints: Vec<&'static str>,
    pub retryable: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorProtocolSnapshot {
    pub identity: ErrorIdentity,
    pub decision: ExposureDecision,
    report: DiagnosticReport,
    #[cfg_attr(feature = "serde", serde(skip))]
    projection: ReportProjectionParts,
}

pub trait ExposurePolicy {
    fn http_status(&self, _identity: &ErrorIdentity) -> u16 {
        500
    }

    fn visibility(&self, _identity: &ErrorIdentity) -> Visibility {
        Visibility::Internal
    }

    fn default_hints(&self, _identity: &ErrorIdentity) -> &'static [&'static str] {
        &[]
    }

    fn retryable(&self, _identity: &ErrorIdentity) -> bool {
        false
    }

    fn decide(&self, identity: &ErrorIdentity) -> ExposureDecision {
        ExposureDecision {
            http_status: self.http_status(identity),
            visibility: self.visibility(identity),
            default_hints: self.default_hints(identity).to_vec(),
            retryable: self.retryable(identity),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultExposurePolicy;

impl ExposurePolicy for DefaultExposurePolicy {
    fn http_status(&self, identity: &ErrorIdentity) -> u16 {
        match identity.category {
            ErrorCategory::Biz => 400,
            ErrorCategory::Conf | ErrorCategory::Logic | ErrorCategory::Sys => 500,
        }
    }

    fn visibility(&self, identity: &ErrorIdentity) -> Visibility {
        match identity.category {
            ErrorCategory::Biz => Visibility::Public,
            ErrorCategory::Conf | ErrorCategory::Logic | ErrorCategory::Sys => Visibility::Internal,
        }
    }

    fn default_hints(&self, identity: &ErrorIdentity) -> &'static [&'static str] {
        match identity.code.as_str() {
            "sys.io_error" => &["check filesystem state", "verify file permissions"],
            "sys.network_error" => &["check network connectivity", "retry the request"],
            "sys.timeout" => &["retry later", "inspect downstream service latency"],
            "conf.core_invalid" | "conf.feature_invalid" | "conf.dynamic_invalid" => {
                &["check configuration values", "validate config source"]
            }
            _ => &[],
        }
    }

    fn retryable(&self, identity: &ErrorIdentity) -> bool {
        matches!(identity.code.as_str(), "sys.network_error" | "sys.timeout")
    }
}

pub trait RedactPolicy {
    fn redact_key(&self, _key: &str) -> bool {
        false
    }

    fn redact_value(&self, _key: Option<&str>, value: &str) -> Option<String> {
        Some(value.to_string())
    }
}

impl<T: DomainReason> StructError<T> {
    /// Build a [`DiagnosticReport`] from this error.
    ///
    /// The report carries human-readable reason, detail, context, and source
    /// frames — no identity or protocol data.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orion_error::{StructError, UvsReason};
    /// use orion_error::report::DiagnosticReport;
    ///
    /// let err = StructError::from(UvsReason::validation_error())
    ///     .with_detail("field `email` is required");
    ///
    /// let report: DiagnosticReport = err.report();
    /// assert!(report.reason.contains("validation"));
    /// assert_eq!(report.detail.as_deref(), Some("field `email` is required"));
    /// ```
    pub fn report(&self) -> DiagnosticReport {
        DiagnosticReport::from_parts(
            self.reason().to_string(),
            self.detail().clone(),
            self.position().clone(),
            self.contexts().to_vec(),
        )
    }

    /// Consume this error and return its human-readable diagnostic report.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orion_error::{StructError, UvsReason};
    ///
    /// let report = StructError::from(UvsReason::validation_error())
    ///     .with_detail("field `email` is required")
    ///     .into_report();
    ///
    /// assert!(report.reason.contains("validation"));
    /// assert_eq!(report.detail.as_deref(), Some("field `email` is required"));
    /// ```
    pub fn into_report(self) -> DiagnosticReport {
        DiagnosticReport::from_parts(
            self.reason().to_string(),
            self.detail().clone(),
            self.position().clone(),
            self.contexts().to_vec(),
        )
    }

    /// Build a redacted [`DiagnosticReport`] using the provided policy.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orion_error::{StructError, UvsReason};
    /// use orion_error::report::RedactPolicy;
    ///
    /// struct HideDetail;
    ///
    /// impl RedactPolicy for HideDetail {
    ///     fn redact_value(&self, key: Option<&str>, value: &str) -> Option<String> {
    ///         if key == Some("detail") {
    ///             Some("<redacted>".to_string())
    ///         } else {
    ///             Some(value.to_string())
    ///         }
    ///     }
    /// }
    ///
    /// let report = StructError::from(UvsReason::validation_error())
    ///     .with_detail("token=abc")
    ///     .report_redacted(&HideDetail);
    ///
    /// assert_eq!(report.detail.as_deref(), Some("<redacted>"));
    /// ```
    pub fn report_redacted(&self, policy: &impl RedactPolicy) -> DiagnosticReport {
        self.report().redacted(policy)
    }

    /// Render this error as a human-readable diagnostic string.
    ///
    /// Delegates to [`DiagnosticReport::render()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use orion_error::StructError;
    /// use orion_error::reason::UvsReason;
    ///
    /// let s = StructError::from(UvsReason::validation_error())
    ///     .with_detail("field `email` is required")
    ///     .render();
    /// assert!(s.contains("validation"));
    /// assert!(s.contains("field `email` is required"));
    /// ```
    pub fn render(&self) -> String {
        self.report().render()
    }

    /// Render a redacted human-readable diagnostic string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orion_error::{StructError, UvsReason};
    /// use orion_error::report::RedactPolicy;
    ///
    /// struct HideDetail;
    ///
    /// impl RedactPolicy for HideDetail {
    ///     fn redact_value(&self, key: Option<&str>, value: &str) -> Option<String> {
    ///         if key == Some("detail") {
    ///             Some("<redacted>".to_string())
    ///         } else {
    ///             Some(value.to_string())
    ///         }
    ///     }
    /// }
    ///
    /// let rendered = StructError::from(UvsReason::validation_error())
    ///     .with_detail("token=abc")
    ///     .render_redacted(&HideDetail);
    ///
    /// assert!(rendered.contains("detail: <redacted>"));
    /// ```
    pub fn render_redacted(&self, policy: &impl RedactPolicy) -> String {
        self.report().render_redacted(policy)
    }
}

impl<T: DomainReason + ErrorIdentityProvider> StructError<T> {
    /// Build an [`ErrorProtocolSnapshot`] by combining identity, exposure
    /// decision, and diagnostic report in one pass.
    ///
    /// This is the primary entry point for protocol-level error output.
    /// Requires [`ErrorIdentityProvider`] (provided by `#[derive(OrionError)]`).
    ///
    /// # Example
    ///
    /// ```rust
    /// use orion_error::{DefaultExposurePolicy, StructError, UvsReason};
    ///
    /// let err = StructError::from(UvsReason::system_error())
    ///     .with_detail("disk full");
    /// let proto = err.exposure_snapshot(&DefaultExposurePolicy);
    /// assert_eq!(proto.identity.code, "sys.io_error");
    /// assert_eq!(proto.decision.http_status, 500);
    /// ```
    pub fn exposure_snapshot(
        &self,
        exposure_policy: &impl ExposurePolicy,
    ) -> ErrorProtocolSnapshot {
        let identity = self.identity_snapshot();
        let report = self.report();
        let projection = ReportProjectionParts::from_error(self);
        ErrorProtocolSnapshot::from_parts(report, projection, identity, exposure_policy)
    }

    /// Consume this error and return its protocol/exposure snapshot.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orion_error::{DefaultExposurePolicy, StructError, UvsReason};
    ///
    /// let proto = StructError::from(UvsReason::system_error())
    ///     .with_detail("disk full")
    ///     .into_exposure_snapshot(&DefaultExposurePolicy);
    ///
    /// assert_eq!(proto.identity.code, "sys.io_error");
    /// assert_eq!(proto.decision.http_status, 500);
    /// ```
    pub fn into_exposure_snapshot(
        self,
        exposure_policy: &impl ExposurePolicy,
    ) -> ErrorProtocolSnapshot {
        let identity = self.identity_snapshot();
        let projection = ReportProjectionParts::from_error(&self);
        let report = self.into_report();
        let decision = exposure_policy.decide(&identity);
        ErrorProtocolSnapshot {
            identity,
            decision,
            report,
            projection,
        }
    }

}

impl From<&ErrorSnapshot> for DiagnosticReport {
    fn from(value: &ErrorSnapshot) -> Self {
        value.report()
    }
}

impl From<ErrorSnapshot> for DiagnosticReport {
    fn from(value: ErrorSnapshot) -> Self {
        value.into_report()
    }
}

impl<T: DomainReason> From<&StructError<T>> for DiagnosticReport {
    fn from(value: &StructError<T>) -> Self {
        value.report()
    }
}

impl<T: DomainReason> From<StructError<T>> for DiagnosticReport {
    fn from(value: StructError<T>) -> Self {
        value.into_report()
    }
}

impl From<&StableErrorSnapshot> for DiagnosticReport {
    fn from(value: &StableErrorSnapshot) -> Self {
        value.report()
    }
}

impl From<StableErrorSnapshot> for DiagnosticReport {
    fn from(value: StableErrorSnapshot) -> Self {
        value.into_report()
    }
}

impl DiagnosticReport {
    pub(crate) fn from_parts(
        reason: String,
        detail: Option<String>,
        position: Option<String>,
        context: Vec<OperationContext>,
    ) -> Self {
        Self {
            reason,
            detail,
            position,
            context,
        }
    }

    /// Render this report as a human-readable diagnostic string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orion_error::{StructError, UvsReason};
    ///
    /// let err = StructError::from(UvsReason::validation_error())
    ///     .with_detail("field `email` is required");
    /// let report = err.report();
    /// let output = report.render();
    /// assert!(output.contains("reason:"));
    /// assert!(output.contains("validation"));
    /// ```
    pub fn render(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("reason: {}", self.reason));

        if let Some(detail) = &self.detail {
            lines.push(format!("detail: {detail}"));
        }
        if let Some(position) = &self.position {
            lines.push(format!("position: {position}"));
        }
        if !self.context.is_empty() {
            lines.push("context:".to_string());
            for (idx, ctx) in self.context.iter().enumerate() {
                lines.push(format!("  [{idx}] {}", ctx.to_string().trim_end()));
            }
        }

        lines.join("\n")
    }

    /// Return a redacted copy of this report.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orion_error::{StructError, UvsReason};
    /// use orion_error::report::RedactPolicy;
    ///
    /// struct HidePosition;
    ///
    /// impl RedactPolicy for HidePosition {
    ///     fn redact_value(&self, key: Option<&str>, value: &str) -> Option<String> {
    ///         if key == Some("position") {
    ///             Some("<hidden>".to_string())
    ///         } else {
    ///             Some(value.to_string())
    ///         }
    ///     }
    /// }
    ///
    /// let report = StructError::from(UvsReason::validation_error())
    ///     .with_position("src/main.rs:42")
    ///     .report()
    ///     .redacted(&HidePosition);
    ///
    /// assert_eq!(report.position.as_deref(), Some("<hidden>"));
    /// ```
    pub fn redacted(&self, policy: &impl RedactPolicy) -> Self {
        Self {
            reason: redact_required_text(Some("reason"), &self.reason, policy),
            detail: redact_optional_text(Some("detail"), self.detail.as_deref(), policy),
            position: redact_optional_text(Some("position"), self.position.as_deref(), policy),
            context: self
                .context
                .iter()
                .cloned()
                .map(|ctx| redact_context(ctx, policy))
            .collect(),
        }
    }

    /// Render this report after applying redaction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orion_error::{StructError, UvsReason};
    /// use orion_error::report::RedactPolicy;
    ///
    /// struct HideDetail;
    ///
    /// impl RedactPolicy for HideDetail {
    ///     fn redact_value(&self, key: Option<&str>, value: &str) -> Option<String> {
    ///         if key == Some("detail") {
    ///             Some("<redacted>".to_string())
    ///         } else {
    ///             Some(value.to_string())
    ///         }
    ///     }
    /// }
    ///
    /// let rendered = StructError::from(UvsReason::validation_error())
    ///     .with_detail("token=abc")
    ///     .report()
    ///     .render_redacted(&HideDetail);
    ///
    /// assert!(rendered.contains("detail: <redacted>"));
    /// ```
    pub fn render_redacted(&self, policy: &impl RedactPolicy) -> String {
        self.redacted(policy).render()
    }

    #[cfg(feature = "serde_json")]
    pub(crate) fn render_summary(&self) -> String {
        let mut out = self.reason.clone();
        if let Some(detail) = &self.detail {
            out.push_str(": ");
            out.push_str(detail);
        }
        out
    }

}

impl ErrorProtocolSnapshot {
    pub fn report(&self) -> &DiagnosticReport {
        &self.report
    }

    pub fn into_report(self) -> DiagnosticReport {
        self.report
    }

    pub(crate) fn from_parts(
        report: DiagnosticReport,
        projection: ReportProjectionParts,
        identity: ErrorIdentity,
        exposure_policy: &impl ExposurePolicy,
    ) -> Self {
        Self {
            decision: exposure_policy.decide(&identity),
            identity,
            report,
            projection,
        }
    }

    pub fn from_report_skeleton(
        report: DiagnosticReport,
        identity: ErrorIdentity,
        exposure_policy: &impl ExposurePolicy,
    ) -> Self {
        // Secondary entry point for tests/adapters that already hold report +
        // identity and only need a protocol shell.
        let projection = ReportProjectionParts::from_identity_skeleton(&identity);
        Self::from_parts(report, projection, identity, exposure_policy)
    }

    #[doc(hidden)]
    pub fn from_report(
        report: DiagnosticReport,
        identity: ErrorIdentity,
        exposure_policy: &impl ExposurePolicy,
    ) -> Self {
        Self::from_report_skeleton(report, identity, exposure_policy)
    }

    pub fn render_user_debug(&self) -> String {
        let debug = self.user_debug_view();
        let mut lines = Vec::new();
        lines.push(format!(
            "code          : {} ({})",
            debug.code,
            debug.category
        ));
        lines.push(format!("detail        : {}", debug.detail));

        lines.push(format!(
            "http          : {} {} retryable={}",
            debug.http_status,
            debug.visibility,
            debug.retryable
        ));

        if let Some(path) = debug.path {
            lines.push(format!("path          : {path}"));
        }

        let context_summary = debug.context_summary;
        if !context_summary.is_empty() {
            lines.push(format!("context       : {context_summary}"));
        }

        if let Some(component) = debug.component {
            lines.push(format!("component     : {component}"));
        } else if let Some(metadata) = debug.metadata_summary {
            lines.push(format!(
                "metadata      : {}",
                metadata
            ));
        }

        if let Some(source_message) = debug.source_message {
            lines.push(format!("source        : {source_message}"));
        }

        lines.join("\n")
    }

    pub fn render_user_debug_redacted(&self, redact_policy: &impl RedactPolicy) -> String {
        self.redacted(redact_policy).render_user_debug()
    }

    fn user_debug_view(&self) -> UserDebugView<'_> {
        UserDebugView {
            code: self.identity.code.as_str(),
            category: self.identity.category.as_str(),
            detail: self
                .report
                .detail
                .as_deref()
                .unwrap_or(self.identity.reason.as_str()),
            http_status: self.decision.http_status,
            visibility: self.decision.visibility.as_str(),
            retryable: self.decision.retryable,
            path: self.identity.path.as_deref(),
            context_summary: self
                .report
                .context
                .iter()
                .flat_map(|ctx| ctx.context().items.iter())
                .map(|(key, value)| format!("{key}={value:?}"))
                .collect::<Vec<_>>()
                .join(", "),
            component: self.projection.root_metadata.get_str("component.name"),
            metadata_summary: (!self.projection.root_metadata.is_empty())
                .then(|| format_metadata_summary(&self.projection.root_metadata)),
            source_message: root_cause_source_frame(&self.projection.source_frames)
                .map(|source| source.message.as_str()),
        }
    }

    #[cfg(feature = "serde_json")]
    fn protocol_json_view(&self) -> ProtocolJsonView {
        ProtocolJsonView {
            status: self.decision.http_status,
            code: self.identity.code.clone(),
            category: self.identity.category.as_str().to_string(),
            reason: self.identity.reason.clone(),
            message: match self.decision.visibility {
                Visibility::Public => self
                    .report
                    .detail
                    .clone()
                    .unwrap_or_else(|| self.identity.reason.clone()),
                Visibility::Internal => self.identity.reason.clone(),
            },
            detail: self.report.detail.clone(),
            rpc_detail: match self.decision.visibility {
                Visibility::Public => self.report.detail.clone(),
                Visibility::Internal => None,
            },
            visibility: self.decision.visibility.as_str().to_string(),
            hints: self.decision.default_hints.clone(),
            retryable: self.decision.retryable,
            operation: self.projection.want.clone(),
            path: self.projection.path.clone(),
            summary: self.report.render_summary(),
            rendered_detail: self.report.render(),
            root_metadata: self.projection.root_metadata.clone(),
            context: self.report.context.clone(),
            source_frames: self.projection.source_frames.clone(),
        }
    }

    #[cfg(feature = "serde_json")]
    pub fn to_http_error_json(&self) -> serde_json::Result<serde_json::Value> {
        self.protocol_json_view().to_http_json()
    }

    #[cfg(feature = "serde_json")]
    pub fn to_cli_error_json(&self) -> serde_json::Result<serde_json::Value> {
        self.protocol_json_view().to_cli_json()
    }

    #[cfg(feature = "serde_json")]
    pub fn to_log_error_json(&self) -> serde_json::Result<serde_json::Value> {
        self.protocol_json_view().to_log_json()
    }

    #[cfg(feature = "serde_json")]
    pub fn to_rpc_error_json(&self) -> serde_json::Result<serde_json::Value> {
        self.protocol_json_view().to_rpc_json()
    }

    pub fn redacted(&self, policy: &impl RedactPolicy) -> Self {
        let report = self.report.redacted(policy);
        let projection = self.projection.redacted(policy);
        Self {
            identity: ErrorIdentity {
                code: self.identity.code.clone(),
                category: self.identity.category,
                reason: redact_required_text(Some("reason"), &self.identity.reason, policy),
                detail: redact_optional_text(
                    Some("detail"),
                    self.identity.detail.as_deref(),
                    policy,
                ),
                position: redact_optional_text(
                    Some("position"),
                    self.identity.position.as_deref(),
                    policy,
                ),
                want: redact_optional_text(Some("want"), self.identity.want.as_deref(), policy),
                path: redact_optional_text(Some("path"), self.identity.path.as_deref(), policy),
            },
            decision: self.decision.clone(),
            report,
            projection,
        }
    }
}

#[cfg(feature = "serde_json")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProtocolJsonView {
    status: u16,
    code: String,
    category: String,
    reason: String,
    message: String,
    detail: Option<String>,
    rpc_detail: Option<String>,
    visibility: String,
    hints: Vec<&'static str>,
    retryable: bool,
    operation: Option<String>,
    path: Option<String>,
    summary: String,
    rendered_detail: String,
    root_metadata: ErrorMetadata,
    context: Vec<OperationContext>,
    source_frames: Vec<SourceFrame>,
}

#[cfg(feature = "serde_json")]
#[derive(serde::Serialize)]
struct HttpErrorJson<'a> {
    status: u16,
    code: &'a str,
    category: &'a str,
    message: &'a str,
    visibility: &'a str,
    hints: &'a [&'static str],
}

#[cfg(feature = "serde_json")]
#[derive(serde::Serialize)]
struct CliErrorJson<'a> {
    code: &'a str,
    category: &'a str,
    summary: &'a str,
    detail: &'a str,
    visibility: &'a str,
    hints: &'a [&'static str],
}

#[cfg(feature = "serde_json")]
#[derive(serde::Serialize)]
struct LogErrorJson<'a> {
    code: &'a str,
    category: &'a str,
    reason: &'a str,
    detail: &'a Option<String>,
    operation: &'a Option<String>,
    path: &'a Option<String>,
    visibility: &'a str,
    hints: &'a [&'static str],
    root_metadata: &'a ErrorMetadata,
    context: &'a [OperationContext],
    source_frames: &'a [SourceFrame],
}

#[cfg(feature = "serde_json")]
#[derive(serde::Serialize)]
struct RpcErrorJson<'a> {
    status: u16,
    code: &'a str,
    category: &'a str,
    reason: &'a str,
    detail: &'a Option<String>,
    visibility: &'a str,
    hints: &'a [&'static str],
    retryable: bool,
}

struct UserDebugView<'a> {
    code: &'a str,
    category: &'static str,
    detail: &'a str,
    http_status: u16,
    visibility: &'static str,
    retryable: bool,
    path: Option<&'a str>,
    context_summary: String,
    component: Option<&'a str>,
    metadata_summary: Option<String>,
    source_message: Option<&'a str>,
}

#[cfg(feature = "serde_json")]
impl ProtocolJsonView {
    fn to_http_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(HttpErrorJson {
            status: self.status,
            code: &self.code,
            category: &self.category,
            message: &self.message,
            visibility: &self.visibility,
            hints: &self.hints,
        })
    }

    fn to_cli_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(CliErrorJson {
            code: &self.code,
            category: &self.category,
            summary: &self.summary,
            detail: &self.rendered_detail,
            visibility: &self.visibility,
            hints: &self.hints,
        })
    }

    fn to_log_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(LogErrorJson {
            code: &self.code,
            category: &self.category,
            reason: &self.reason,
            detail: &self.detail,
            operation: &self.operation,
            path: &self.path,
            visibility: &self.visibility,
            hints: &self.hints,
            root_metadata: &self.root_metadata,
            context: &self.context,
            source_frames: &self.source_frames,
        })
    }

    fn to_rpc_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(RpcErrorJson {
            status: self.status,
            code: &self.code,
            category: &self.category,
            reason: &self.reason,
            detail: &self.rpc_detail,
            visibility: &self.visibility,
            hints: &self.hints,
            retryable: self.retryable,
        })
    }
}

fn root_cause_source_frame(source_frames: &[SourceFrame]) -> Option<&SourceFrame> {
    source_frames
        .iter()
        .find(|frame| frame.is_root_cause)
        .or_else(|| source_frames.last())
}

fn format_metadata_summary(metadata: &ErrorMetadata) -> String {
    metadata
        .iter()
        .map(|(key, value)| format!("{key}={}", format_metadata_value(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_metadata_value(value: &MetadataValue) -> String {
    match value {
        MetadataValue::String(value) => format!("{value:?}"),
        MetadataValue::Bool(value) => value.to_string(),
        MetadataValue::I64(value) => value.to_string(),
        MetadataValue::U64(value) => value.to_string(),
    }
}

fn redact_optional_text(
    key: Option<&str>,
    value: Option<&str>,
    policy: &impl RedactPolicy,
) -> Option<String> {
    value.and_then(|value| policy.redact_value(key, value))
}

fn redact_context(ctx: OperationContext, policy: &impl RedactPolicy) -> OperationContext {
    let mut redacted_items = Vec::with_capacity(ctx.context().items.len());
    for (key, value) in &ctx.context().items {
        let kept = if policy.redact_key(key) {
            policy
                .redact_value(Some(key.as_str()), value)
                .or_else(|| Some("<redacted>".to_string()))
        } else {
            policy.redact_value(Some(key.as_str()), value)
        };

        if let Some(value) = kept {
            redacted_items.push((key.clone(), value));
        }
    }

    let redacted_target = ctx.compat_target();
    let redacted_want = redact_optional_text(Some("want"), redacted_target.as_deref(), policy);
    let redacted_action = redact_optional_text(Some("action"), ctx.action().as_deref(), policy);
    let redacted_locator = redact_optional_text(Some("locator"), ctx.locator().as_deref(), policy);
    let redacted_path = ctx
        .path()
        .iter()
        .filter_map(|segment| redact_optional_text(Some("path"), Some(segment.as_str()), policy))
        .collect::<Vec<_>>();
    OperationContext::from_projection_parts(
        redacted_want,
        redacted_action,
        redacted_locator,
        redacted_path,
        redacted_items,
        redact_metadata(ctx.metadata(), policy),
        ctx.result().clone(),
    )
}

fn redact_metadata(metadata: &ErrorMetadata, policy: &impl RedactPolicy) -> ErrorMetadata {
    let mut redacted = ErrorMetadata::new();
    for (key, value) in metadata.iter() {
        match value {
            MetadataValue::String(value) => {
                if policy.redact_key(key) {
                    if let Some(value) = policy
                        .redact_value(Some(key.as_str()), value)
                        .or_else(|| Some("<redacted>".to_string()))
                    {
                        redacted.insert(key.clone(), value);
                    }
                } else if let Some(value) = policy.redact_value(Some(key.as_str()), value) {
                    redacted.insert(key.clone(), value);
                }
            }
            MetadataValue::Bool(value) => {
                if !policy.redact_key(key) {
                    redacted.insert(key.clone(), *value);
                }
            }
            MetadataValue::I64(value) => {
                if !policy.redact_key(key) {
                    redacted.insert(key.clone(), *value);
                }
            }
            MetadataValue::U64(value) => {
                if !policy.redact_key(key) {
                    redacted.insert(key.clone(), *value);
                }
            }
        }
    }
    redacted
}

fn redact_frame(mut frame: SourceFrame, policy: &impl RedactPolicy) -> SourceFrame {
    frame.message = redact_required_text(Some("source.message"), &frame.message, policy);
    frame.display = redact_optional_text(Some("source.display"), frame.display.as_deref(), policy);
    frame.debug = redact_required_text(Some("source.debug"), &frame.debug, policy);
    frame.detail = redact_optional_text(Some("detail"), frame.detail.as_deref(), policy);
    frame.reason = redact_optional_text(Some("source.reason"), frame.reason.as_deref(), policy);
    frame.want = redact_optional_text(Some("want"), frame.want.as_deref(), policy);
    frame.path = redact_optional_text(Some("path"), frame.path.as_deref(), policy);
    frame.metadata = redact_metadata(&frame.metadata, policy);
    frame
}

fn redact_required_text(key: Option<&str>, value: &str, policy: &impl RedactPolicy) -> String {
    policy
        .redact_value(key, value)
        .unwrap_or_else(|| "<redacted>".to_string())
}

#[cfg(test)]
mod tests {
    use crate::{
        core::DomainReason,
        core::{
            ErrorIdentity, ErrorMetadata, SourceFrame, StableErrorSnapshot,
            StableSnapshotContextFrame, StableSnapshotSourceFrame,
        },
        OperationContext, StructError, UvsReason,
    };
    use crate::reason::{ErrorCategory, ErrorCode, ErrorIdentityProvider};

    use super::{
        DefaultExposurePolicy, DiagnosticReport, ErrorProtocolSnapshot, ExposureDecision,
        ExposurePolicy, RedactPolicy, ReportProjectionParts, Visibility,
    };
    use crate::core::STABLE_SNAPSHOT_SCHEMA_VERSION;
    #[derive(Debug)]
    struct TestPolicy;

    impl RedactPolicy for TestPolicy {
        fn redact_key(&self, key: &str) -> bool {
            matches!(key, "token" | "password" | "config.secret")
        }

        fn redact_value(&self, _key: Option<&str>, _value: &str) -> Option<String> {
            Some("<redacted>".to_string())
        }
    }

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

    fn test_identity(
        code: &str,
        category: ErrorCategory,
        reason: &str,
        detail: Option<&str>,
        want: Option<&str>,
        path: Option<&str>,
    ) -> ErrorIdentity {
        ErrorIdentity {
            code: code.to_string(),
            category,
            reason: reason.to_string(),
            detail: detail.map(str::to_string),
            position: None,
            want: want.map(str::to_string),
            path: path.map(str::to_string),
        }
    }

    fn test_proto(
        report: DiagnosticReport,
        projection: ReportProjectionParts,
        identity: ErrorIdentity,
        decision: ExposureDecision,
    ) -> ErrorProtocolSnapshot {
        ErrorProtocolSnapshot {
            identity,
            decision,
            report,
            projection,
        }
    }

    #[test]
    fn test_report_contains_root_and_source_data() {
        let source = StructError::from(TestReason::TestError).with_context(
            OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
        );
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_context(
                OperationContext::doing("start engine").with_meta("component.name", "engine"),
            )
            .with_struct_source(source);

        let report = err.report();
        let rendered = report.render();

        assert_eq!(report.reason, "system error");
        assert!(rendered.contains("reason: system error"));
        assert!(rendered.contains("context:"));
        assert!(rendered.contains("start engine"));
    }

    #[test]
    fn test_struct_error_into_report_matches_borrowed_report() {
        let source = StructError::from(TestReason::TestError).with_context(
            OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
        );
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_context(
                OperationContext::doing("start engine").with_meta("component.name", "engine"),
            )
            .with_struct_source(source);

        let via_borrowed = err.report();
        let via_owned = err.into_report();

        assert_eq!(via_owned, via_borrowed);
    }

    #[test]
    fn test_report_from_struct_error_matches_report_methods() {
        let source = StructError::from(TestReason::TestError).with_context(
            OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
        );
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_context(
                OperationContext::doing("start engine").with_meta("component.name", "engine"),
            )
            .with_struct_source(source);

        let via_method = err.report();
        let via_borrowed = DiagnosticReport::from(&err);
        let via_owned = DiagnosticReport::from(err);

        assert_eq!(via_borrowed, via_method);
        assert_eq!(via_owned, via_method);
    }

    #[test]
    fn test_report_from_stable_snapshot_matches_report_methods() {
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
        let via_borrowed = DiagnosticReport::from(&stable);
        let via_owned = DiagnosticReport::from(stable);

        assert_eq!(via_borrowed, via_method);
        assert_eq!(via_owned, via_method);
    }

    #[test]
    fn test_report_verbose_render_includes_metadata() {
        let report = DiagnosticReport::from_parts(
            "test error".to_string(),
            Some("failed".to_string()),
            None,
            vec![OperationContext::doing("load")],
        );

        let rendered = report.render();

        assert!(rendered.contains("reason: test error"));
        assert!(rendered.contains("detail: failed"));
        assert!(rendered.contains("context:"));
    }

    #[test]
    fn test_default_exposure_policy_maps_category_to_http_status_and_visibility() {
        let exposure_policy = DefaultExposurePolicy;
        let biz_identity = ErrorIdentity {
            code: "biz.validation_error".to_string(),
            category: ErrorCategory::Biz,
            reason: "validation error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
        };
        let sys_identity = ErrorIdentity {
            code: "sys.io_error".to_string(),
            category: ErrorCategory::Sys,
            reason: "system error".to_string(),
            detail: None,
            position: None,
            want: None,
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

    #[test]
    fn test_struct_error_exposure_snapshot_uses_real_stable_identity() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("engine bootstrap failed")
            .with_context(OperationContext::doing("start engine"));

        let snapshot = err.exposure_snapshot(&DefaultExposurePolicy);

        assert_eq!(snapshot.identity.code, "sys.io_error");
        assert_eq!(snapshot.identity.category, ErrorCategory::Sys);
        assert_eq!(snapshot.decision.http_status, 500);
        assert_eq!(
            snapshot.decision.default_hints,
            vec!["check filesystem state", "verify file permissions"]
        );
        assert_eq!(
            snapshot.decision,
            ExposureDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec!["check filesystem state", "verify file permissions"],
                retryable: false,
            }
        );
        assert_eq!(snapshot.report().reason, "system error");
    }

    #[test]
    fn test_report_decision_uses_exposure_identity_fallback() {
        let report = DiagnosticReport::from_parts(
            "configuration error".to_string(),
            Some("invalid config".to_string()),
            None,
            vec![],
        );

        let identity = ErrorIdentity {
            code: "test.error".to_string(),
            category: ErrorCategory::Sys,
            reason: "configuration error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
        };

        let snapshot = ErrorProtocolSnapshot::from_report_skeleton(
            report,
            identity,
            &DefaultExposurePolicy,
        );

        assert_eq!(
            snapshot.decision,
            ExposureDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec![],
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
        assert_eq!(snapshot.report().reason, "system error");
        assert_eq!(snapshot.report().detail, Some("engine bootstrap failed".to_string()));
    }

    #[test]
    fn test_exposure_snapshot_contains_identity_decision_and_report() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("engine bootstrap failed");

        let snapshot = err.exposure_snapshot(&DefaultExposurePolicy);

        assert_eq!(snapshot.identity.code, "sys.io_error");
        assert_eq!(snapshot.identity.category, ErrorCategory::Sys);
        assert!(snapshot.decision.http_status > 0);
        assert_eq!(snapshot.report().reason, "system error");
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

    #[test]
    fn test_exposure_snapshot_render_debug_summary_prefers_detail_path_context_and_component() {
        let snapshot = test_proto(
            DiagnosticReport {
                reason: "invalid order".to_string(),
                detail: Some("order text must not be empty".to_string()),
                position: None,
                context: vec![{
                    let mut ctx = OperationContext::doing("place_order");
                    ctx.record_field("user_id", "42");
                    ctx.record_field("order.raw", "");
                    ctx.record_meta("component.name", "order_service");
                    ctx
                }],
            },
            ReportProjectionParts {
                want: Some("place_order".to_string()),
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
                Some("place_order"),
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
            DiagnosticReport {
                reason: "storage full".to_string(),
                detail: None,
                position: None,
                context: vec![],
            },
            ReportProjectionParts {
                want: Some("place_order".to_string()),
                path: Some("place_order / save order".to_string()),
                root_metadata: ErrorMetadata::new(),
                source_frames: vec![SourceFrame {
                    index: 0,
                    message: "storage full".to_string(),
                    display: None,
                    debug: String::new(),
                    type_name: None,
                    error_code: None,
                    reason: None,
                    want: None,
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
                Some("place_order"),
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
            DiagnosticReport {
                reason: "system error".to_string(),
                detail: Some("save order failed".to_string()),
                position: None,
                context: vec![],
            },
            ReportProjectionParts {
                want: Some("place_order".to_string()),
                path: Some("place_order / save order".to_string()),
                root_metadata: ErrorMetadata::new(),
                source_frames: vec![
                    SourceFrame {
                        index: 0,
                        message: "storage layer failed".to_string(),
                        display: None,
                        debug: String::new(),
                        type_name: None,
                        error_code: None,
                        reason: None,
                        want: None,
                        path: None,
                        detail: None,
                        metadata: ErrorMetadata::new(),
                        is_root_cause: false,
                    },
                    SourceFrame {
                        index: 1,
                        message: "disk offline".to_string(),
                        display: None,
                        debug: String::new(),
                        type_name: None,
                        error_code: None,
                        reason: None,
                        want: None,
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
                Some("place_order"),
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
                vec![],
            ),
            ReportProjectionParts {
                want: None,
                path: None,
                root_metadata: ErrorMetadata::new(),
                source_frames: vec![SourceFrame {
                    index: 0,
                    message: "inner".to_string(),
                    display: Some("inner token=abc".to_string()),
                    debug: "debug".to_string(),
                    type_name: None,
                    error_code: None,
                    reason: None,
                    want: None,
                    path: None,
                    detail: None,
                    metadata: ErrorMetadata::new(),
                    is_root_cause: true,
                }],
            },
            test_identity("test.error", ErrorCategory::Logic, "test error", None, None, None),
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
                vec![],
            ),
            ReportProjectionParts {
                want: None,
                path: None,
                root_metadata: ErrorMetadata::new(),
                source_frames: vec![SourceFrame {
                    index: 0,
                    message: "inner".to_string(),
                    display: None,
                    debug: "debug token=abc".to_string(),
                    type_name: None,
                    error_code: None,
                    reason: None,
                    want: None,
                    path: None,
                    detail: None,
                    metadata: ErrorMetadata::new(),
                    is_root_cause: true,
                }],
            },
            test_identity("test.error", ErrorCategory::Logic, "test error", None, None, None),
            ExposureDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec![],
                retryable: false,
            },
        );

        let redacted = snapshot.redacted(&TestPolicy);
        assert_eq!(redacted.projection.source_frames[0].debug, "<redacted>");
        assert!(!redacted.projection.source_frames[0].debug.contains("token=abc"));
    }

    #[test]
    fn test_report_redaction_masks_root_and_frame_paths() {
        let report = DiagnosticReport::from_parts(
            "test error".to_string(),
            None,
            Some("/srv/app/config.toml:10".to_string()),
            vec![OperationContext::at("/srv/app/config.toml")],
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
            vec![],
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
        assert_eq!(redacted.reason, "tenant <redacted> error");
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
}
