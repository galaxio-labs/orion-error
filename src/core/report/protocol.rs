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
    /// use orion_error::protocol::DefaultExposurePolicy;
    /// use orion_error::{StructError, UvsReason};
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
        let projection = ReportProjectionParts {
            path: identity.path.clone(),
            root_metadata: self.context_metadata(),
            source_frames: self.source_frames().to_vec(),
        };
        ErrorProtocolSnapshot::from_parts(report, projection, identity, exposure_policy)
    }

    /// Consume this error and return its protocol/exposure snapshot.
    ///
    /// # Example
    ///
    /// ```rust
    /// use orion_error::protocol::DefaultExposurePolicy;
    /// use orion_error::{StructError, UvsReason};
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
        let projection = ReportProjectionParts {
            path: identity.path.clone(),
            root_metadata: self.context_metadata(),
            source_frames: self.source_frames().to_vec(),
        };
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

impl ErrorProtocolSnapshot {
    /// Render the embedded diagnostic report.
    ///
    /// This keeps protocol-boundary consumers from having to reach through
    /// `report()` just to obtain the human-facing text form.
    pub fn render(&self) -> String {
        self.report.render()
    }

    /// Render the embedded diagnostic report after redaction.
    pub fn render_redacted(&self, policy: &impl RedactPolicy) -> String {
        self.report.render_redacted(policy)
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

    /// Build a protocol shell from an already-materialized report plus stable
    /// identity.
    ///
    /// This is a secondary adapter/test entry point. It does not carry the
    /// full runtime-derived projection payload that
    /// `StructError::exposure_snapshot(...)` can assemble.
    ///
    /// Prefer `StructError::exposure_snapshot(...)` for normal business code
    /// and boundary output.
    pub(crate) fn from_report_skeleton(
        report: DiagnosticReport,
        identity: ErrorIdentity,
        exposure_policy: &impl ExposurePolicy,
    ) -> Self {
        let projection = ReportProjectionParts::from_identity_skeleton(&identity);
        Self::from_parts(report, projection, identity, exposure_policy)
    }

    pub fn render_user_debug(&self) -> String {
        let debug = self.user_debug_view();
        let mut lines = Vec::new();
        lines.push(format!("code          : {} ({})", debug.code, debug.category));
        lines.push(format!("detail        : {}", debug.detail));
        lines.push(format!(
            "http          : {} {} retryable={}",
            debug.http_status, debug.visibility, debug.retryable
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
            lines.push(format!("metadata      : {}", metadata));
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
            detail: self.report.detail().unwrap_or(self.identity.reason.as_str()),
            http_status: self.decision.http_status,
            visibility: self.decision.visibility.as_str(),
            retryable: self.decision.retryable,
            path: self.identity.path.as_deref(),
            context_summary: self
                .report
                .context()
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
            path: self.projection.path.clone(),
            summary: self.report.render_summary(),
            rendered_detail: self.report.render(),
            root_metadata: self.projection.root_metadata.clone(),
            context: self.report.context.as_ref().clone(),
            source_frames: self.projection.source_frames.clone(),
        }
    }

    /// Serialize to HTTP-bound error JSON.
    ///
    /// Requires feature: `"serde_json"`.
    #[cfg(feature = "serde_json")]
    pub fn to_http_error_json(&self) -> serde_json::Result<serde_json::Value> {
        self.protocol_json_view().to_http_json()
    }

    /// Serialize to CLI-bound error JSON.
    ///
    /// Requires feature: `"serde_json"`.
    #[cfg(feature = "serde_json")]
    pub fn to_cli_error_json(&self) -> serde_json::Result<serde_json::Value> {
        self.protocol_json_view().to_cli_json()
    }

    /// Serialize to log-bound error JSON.
    ///
    /// Requires feature: `"serde_json"`.
    #[cfg(feature = "serde_json")]
    pub fn to_log_error_json(&self) -> serde_json::Result<serde_json::Value> {
        self.protocol_json_view().to_log_json()
    }

    /// Serialize to RPC-bound error JSON.
    ///
    /// Requires feature: `"serde_json"`.
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
