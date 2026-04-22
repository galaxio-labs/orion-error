use crate::{DomainReason, ErrorCategory, StructError};

use super::{
    snapshot::{ErrorIdentitySnapshot, StableStructErrorSnapshot, StructErrorSnapshot},
    ErrorMetadata, MetadataValue, OperationContext, SourceFrame,
};

pub const POLICY_SNAPSHOT_TOP_LEVEL_FIELDS: &[&str] = &["identity", "decision", "report"];
pub const POLICY_DECISION_FIELDS: &[&str] =
    &["http_status", "visibility", "default_hints", "retryable"];
pub const HTTP_ERROR_RESPONSE_FIELDS: &[&str] = &[
    "status",
    "code",
    "category",
    "message",
    "visibility",
    "hints",
];
pub const CLI_ERROR_RESPONSE_FIELDS: &[&str] = &[
    "code",
    "category",
    "summary",
    "detail",
    "visibility",
    "hints",
];
pub const LOG_ERROR_RESPONSE_FIELDS: &[&str] = &[
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
];
pub const RPC_ERROR_RESPONSE_FIELDS: &[&str] = &[
    "status",
    "code",
    "category",
    "reason",
    "detail",
    "visibility",
    "hints",
    "retryable",
];

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorReport {
    pub reason: String,
    pub detail: Option<String>,
    pub position: Option<String>,
    pub want: Option<String>,
    pub path: Option<String>,
    pub context: Vec<OperationContext>,
    pub root_metadata: ErrorMetadata,
    pub source_frames: Vec<SourceFrame>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Compact,
    Verbose,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Internal,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorPolicyDecision {
    pub http_status: u16,
    pub visibility: Visibility,
    pub default_hints: Vec<&'static str>,
    pub retryable: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorPolicySnapshot {
    pub identity: ErrorIdentitySnapshot,
    pub decision: ErrorPolicyDecision,
    pub report: ErrorReport,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorHttpResponse {
    pub status: u16,
    pub code: String,
    pub category: ErrorCategory,
    pub message: String,
    pub visibility: Visibility,
    pub hints: Vec<String>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorCliResponse {
    pub code: String,
    pub category: ErrorCategory,
    pub summary: String,
    pub detail: String,
    pub visibility: Visibility,
    pub hints: Vec<String>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorLogResponse {
    pub code: String,
    pub category: ErrorCategory,
    pub reason: String,
    pub detail: Option<String>,
    pub operation: Option<String>,
    pub path: Option<String>,
    pub visibility: Visibility,
    pub hints: Vec<String>,
    pub root_metadata: ErrorMetadata,
    pub context: Vec<OperationContext>,
    pub source_frames: Vec<SourceFrame>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorRpcResponse {
    pub status: u16,
    pub code: String,
    pub category: ErrorCategory,
    pub reason: String,
    pub detail: Option<String>,
    pub visibility: Visibility,
    pub hints: Vec<String>,
    pub retryable: bool,
}

pub trait ErrorPolicy {
    fn http_status(&self, _identity: &ErrorIdentitySnapshot) -> u16 {
        500
    }

    fn visibility(&self, _identity: &ErrorIdentitySnapshot) -> Visibility {
        Visibility::Internal
    }

    fn default_hints(&self, _identity: &ErrorIdentitySnapshot) -> &'static [&'static str] {
        &[]
    }

    fn retryable(&self, _identity: &ErrorIdentitySnapshot) -> bool {
        false
    }

    fn decide(&self, identity: &ErrorIdentitySnapshot) -> ErrorPolicyDecision {
        ErrorPolicyDecision {
            http_status: self.http_status(identity),
            visibility: self.visibility(identity),
            default_hints: self.default_hints(identity).to_vec(),
            retryable: self.retryable(identity),
        }
    }
}

pub trait ErrorRenderer {
    type Output;

    fn render(&self, report: &ErrorReport) -> Self::Output;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorPolicyView {
    identity: ErrorIdentitySnapshot,
    report: ErrorReport,
}

impl ErrorPolicyView {
    pub fn new(identity: ErrorIdentitySnapshot, report: ErrorReport) -> Self {
        Self { identity, report }
    }

    pub fn identity(&self) -> &ErrorIdentitySnapshot {
        &self.identity
    }

    pub fn report(&self) -> &ErrorReport {
        &self.report
    }

    pub fn into_parts(self) -> (ErrorIdentitySnapshot, ErrorReport) {
        (self.identity, self.report)
    }

    pub fn render_with<R>(&self, renderer: R) -> R::Output
    where
        R: ErrorRenderer,
    {
        renderer.render(&self.report)
    }

    pub fn http_status(&self, policy: &impl ErrorPolicy) -> u16 {
        policy.http_status(&self.identity)
    }

    pub fn visibility(&self, policy: &impl ErrorPolicy) -> Visibility {
        policy.visibility(&self.identity)
    }

    pub fn default_hints(&self, policy: &impl ErrorPolicy) -> &'static [&'static str] {
        policy.default_hints(&self.identity)
    }

    pub fn decision(&self, policy: &impl ErrorPolicy) -> ErrorPolicyDecision {
        policy.decide(&self.identity)
    }

    pub fn snapshot(&self, policy: &impl ErrorPolicy) -> ErrorPolicySnapshot {
        ErrorPolicySnapshot {
            identity: self.identity.clone(),
            decision: self.decision(policy),
            report: self.report.clone(),
        }
    }

    pub fn http_response(&self, policy: &impl ErrorPolicy) -> ErrorHttpResponse {
        self.snapshot(policy).http_response()
    }

    pub fn cli_response(&self, policy: &impl ErrorPolicy) -> ErrorCliResponse {
        self.snapshot(policy).cli_response()
    }

    pub fn log_response(&self, policy: &impl ErrorPolicy) -> ErrorLogResponse {
        self.snapshot(policy).log_response()
    }

    pub fn rpc_response(&self, policy: &impl ErrorPolicy) -> ErrorRpcResponse {
        self.snapshot(policy).rpc_response()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextReportRenderer {
    mode: RenderMode,
}

impl TextReportRenderer {
    pub fn new(mode: RenderMode) -> Self {
        Self { mode }
    }
}

impl ErrorRenderer for TextReportRenderer {
    type Output = String;

    fn render(&self, report: &ErrorReport) -> Self::Output {
        match self.mode {
            RenderMode::Compact => report.render_compact(),
            RenderMode::Verbose => report.render_verbose(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultErrorPolicy;

impl ErrorPolicy for DefaultErrorPolicy {
    fn http_status(&self, identity: &ErrorIdentitySnapshot) -> u16 {
        match identity.category {
            ErrorCategory::Biz => 400,
            ErrorCategory::Conf | ErrorCategory::Logic | ErrorCategory::Sys => 500,
        }
    }

    fn visibility(&self, identity: &ErrorIdentitySnapshot) -> Visibility {
        match identity.category {
            ErrorCategory::Biz => Visibility::Public,
            ErrorCategory::Conf | ErrorCategory::Logic | ErrorCategory::Sys => Visibility::Internal,
        }
    }

    fn default_hints(&self, identity: &ErrorIdentitySnapshot) -> &'static [&'static str] {
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

    fn retryable(&self, identity: &ErrorIdentitySnapshot) -> bool {
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
    pub fn report(&self) -> ErrorReport {
        self.snapshot().report()
    }

    pub fn into_report(self) -> ErrorReport {
        self.into_snapshot().into_report()
    }

    pub fn report_redacted(&self, policy: &impl RedactPolicy) -> ErrorReport {
        self.report().redacted(policy)
    }
}

impl<T> StructError<T>
where
    T: DomainReason + crate::StableErrorIdentity,
{
    pub fn policy_report(&self) -> ErrorPolicyView {
        ErrorPolicyView::new(self.identity_snapshot(), self.report())
    }

    pub fn into_policy_report(self) -> ErrorPolicyView {
        let identity = self.identity_snapshot();
        let report = self.into_report();
        ErrorPolicyView::new(identity, report)
    }

    pub fn policy_snapshot(&self, policy: &impl ErrorPolicy) -> ErrorPolicySnapshot {
        self.policy_report().snapshot(policy)
    }

    pub fn into_policy_snapshot(self, policy: &impl ErrorPolicy) -> ErrorPolicySnapshot {
        self.into_policy_report().snapshot(policy)
    }

    pub fn http_response(&self, policy: &impl ErrorPolicy) -> ErrorHttpResponse {
        self.policy_snapshot(policy).http_response()
    }

    pub fn cli_response(&self, policy: &impl ErrorPolicy) -> ErrorCliResponse {
        self.policy_snapshot(policy).cli_response()
    }

    pub fn log_response(&self, policy: &impl ErrorPolicy) -> ErrorLogResponse {
        self.policy_snapshot(policy).log_response()
    }

    pub fn rpc_response(&self, policy: &impl ErrorPolicy) -> ErrorRpcResponse {
        self.policy_snapshot(policy).rpc_response()
    }
}

impl From<&StructErrorSnapshot> for ErrorReport {
    fn from(value: &StructErrorSnapshot) -> Self {
        value.report()
    }
}

impl From<StructErrorSnapshot> for ErrorReport {
    fn from(value: StructErrorSnapshot) -> Self {
        value.into_report()
    }
}

impl<T: DomainReason> From<&StructError<T>> for ErrorReport {
    fn from(value: &StructError<T>) -> Self {
        value.report()
    }
}

impl<T: DomainReason> From<StructError<T>> for ErrorReport {
    fn from(value: StructError<T>) -> Self {
        value.into_report()
    }
}

impl From<&StableStructErrorSnapshot> for ErrorReport {
    fn from(value: &StableStructErrorSnapshot) -> Self {
        value.report()
    }
}

impl From<StableStructErrorSnapshot> for ErrorReport {
    fn from(value: StableStructErrorSnapshot) -> Self {
        value.into_report()
    }
}

impl ErrorReport {
    pub fn render(&self, mode: RenderMode) -> String {
        self.render_with(TextReportRenderer::new(mode))
    }

    pub fn render_with<R>(&self, renderer: R) -> R::Output
    where
        R: ErrorRenderer,
    {
        renderer.render(self)
    }

    pub fn redacted(&self, policy: &impl RedactPolicy) -> Self {
        Self {
            reason: redact_required_text(Some("reason"), &self.reason, policy),
            detail: redact_optional_text(Some("detail"), self.detail.as_deref(), policy),
            position: redact_optional_text(Some("position"), self.position.as_deref(), policy),
            want: redact_optional_text(Some("want"), self.want.as_deref(), policy),
            path: redact_optional_text(Some("path"), self.path.as_deref(), policy),
            context: self
                .context
                .iter()
                .cloned()
                .map(|ctx| redact_context(ctx, policy))
                .collect(),
            root_metadata: redact_metadata(&self.root_metadata, policy),
            source_frames: self
                .source_frames
                .iter()
                .cloned()
                .map(|frame| redact_frame(frame, policy))
                .collect(),
        }
    }

    pub fn render_redacted(&self, mode: RenderMode, policy: &impl RedactPolicy) -> String {
        self.redacted(policy).render(mode)
    }

    fn render_compact(&self) -> String {
        let mut out = self.reason.clone();
        if let Some(detail) = &self.detail {
            out.push_str(": ");
            out.push_str(detail);
        }
        out
    }

    fn render_verbose(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("reason: {}", self.reason));

        if let Some(detail) = &self.detail {
            lines.push(format!("detail: {detail}"));
        }
        if let Some(position) = &self.position {
            lines.push(format!("position: {position}"));
        }
        if let Some(want) = &self.want {
            lines.push(format!("want: {want}"));
        }
        if let Some(path) = &self.path {
            if self.want.as_deref() != Some(path.as_str()) {
                lines.push(format!("path: {path}"));
            }
        }
        if !self.root_metadata.is_empty() {
            lines.push(format!("root_metadata: {:?}", self.root_metadata.as_map()));
        }
        if !self.context.is_empty() {
            lines.push("context:".to_string());
            for (idx, ctx) in self.context.iter().enumerate() {
                lines.push(format!("  [{idx}] {}", ctx.to_string().trim_end()));
            }
        }
        if !self.source_frames.is_empty() {
            lines.push("source_frames:".to_string());
            for frame in &self.source_frames {
                let mut frame_line = format!("  [{}] {}", frame.index, frame.message);
                if let Some(reason) = &frame.reason {
                    frame_line.push_str(&format!(" reason={reason}"));
                }
                if let Some(want) = &frame.want {
                    frame_line.push_str(&format!(" want={want}"));
                }
                if let Some(path) = &frame.path {
                    frame_line.push_str(&format!(" path={path}"));
                }
                if !frame.metadata.is_empty() {
                    frame_line.push_str(&format!(" metadata={:?}", frame.metadata.as_map()));
                }
                if frame.is_root_cause {
                    frame_line.push_str(" root_cause=true");
                }
                lines.push(frame_line);
            }
        }

        lines.join("\n")
    }
}

impl ErrorReport {
    pub fn policy_identity(&self) -> ErrorIdentitySnapshot {
        let category = if self.reason.contains("configuration error") {
            ErrorCategory::Conf
        } else {
            ErrorCategory::Sys
        };

        ErrorIdentitySnapshot {
            code: "report.unclassified".to_string(),
            category,
            reason: self.reason.clone(),
            detail: self.detail.clone(),
            position: self.position.clone(),
            want: self.want.clone(),
            path: self.path.clone(),
        }
    }

    pub fn http_status(&self, policy: &impl ErrorPolicy) -> u16 {
        policy.http_status(&self.policy_identity())
    }

    pub fn visibility(&self, policy: &impl ErrorPolicy) -> Visibility {
        policy.visibility(&self.policy_identity())
    }

    pub fn default_hints(&self, policy: &impl ErrorPolicy) -> &'static [&'static str] {
        policy.default_hints(&self.policy_identity())
    }

    pub fn decision(&self, policy: &impl ErrorPolicy) -> ErrorPolicyDecision {
        policy.decide(&self.policy_identity())
    }

    pub fn policy_snapshot(&self, policy: &impl ErrorPolicy) -> ErrorPolicySnapshot {
        ErrorPolicySnapshot {
            identity: self.policy_identity(),
            decision: self.decision(policy),
            report: self.clone(),
        }
    }

    pub fn http_response(&self, policy: &impl ErrorPolicy) -> ErrorHttpResponse {
        self.policy_snapshot(policy).http_response()
    }

    pub fn cli_response(&self, policy: &impl ErrorPolicy) -> ErrorCliResponse {
        self.policy_snapshot(policy).cli_response()
    }

    pub fn log_response(&self, policy: &impl ErrorPolicy) -> ErrorLogResponse {
        self.policy_snapshot(policy).log_response()
    }

    pub fn rpc_response(&self, policy: &impl ErrorPolicy) -> ErrorRpcResponse {
        self.policy_snapshot(policy).rpc_response()
    }

    #[cfg(feature = "serde_json")]
    pub fn to_policy_snapshot_json(
        &self,
        policy: &impl ErrorPolicy,
    ) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.policy_snapshot(policy))
    }

    pub fn policy_view(self, identity: ErrorIdentitySnapshot) -> ErrorPolicyView {
        ErrorPolicyView::new(identity, self)
    }

    #[cfg(feature = "serde_json")]
    pub fn to_policy_report_json(
        &self,
        policy: &impl ErrorPolicy,
    ) -> serde_json::Result<serde_json::Value> {
        self.to_policy_snapshot_json(policy)
    }

    #[cfg(feature = "serde_json")]
    pub fn to_http_error_json(
        &self,
        policy: &impl ErrorPolicy,
    ) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.http_response(policy))
    }

    #[cfg(feature = "serde_json")]
    pub fn to_cli_error_json(
        &self,
        policy: &impl ErrorPolicy,
    ) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.cli_response(policy))
    }

    #[cfg(feature = "serde_json")]
    pub fn to_log_error_json(
        &self,
        policy: &impl ErrorPolicy,
    ) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.log_response(policy))
    }

    #[cfg(feature = "serde_json")]
    pub fn to_rpc_error_json(
        &self,
        policy: &impl ErrorPolicy,
    ) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.rpc_response(policy))
    }
}

impl ErrorPolicySnapshot {
    pub fn http_response(&self) -> ErrorHttpResponse {
        ErrorHttpResponse {
            status: self.decision.http_status,
            code: self.identity.code.clone(),
            category: self.identity.category,
            message: match self.decision.visibility {
                Visibility::Public => self
                    .report
                    .detail
                    .clone()
                    .unwrap_or_else(|| self.report.reason.clone()),
                Visibility::Internal => self.identity.reason.clone(),
            },
            visibility: self.decision.visibility,
            hints: self
                .decision
                .default_hints
                .iter()
                .map(|hint| (*hint).to_string())
                .collect(),
        }
    }

    #[cfg(feature = "serde_json")]
    pub fn to_http_error_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.http_response())
    }

    pub fn cli_response(&self) -> ErrorCliResponse {
        ErrorCliResponse {
            code: self.identity.code.clone(),
            category: self.identity.category,
            summary: self.report.render(RenderMode::Compact),
            detail: self.report.render(RenderMode::Verbose),
            visibility: self.decision.visibility,
            hints: self
                .decision
                .default_hints
                .iter()
                .map(|hint| (*hint).to_string())
                .collect(),
        }
    }

    #[cfg(feature = "serde_json")]
    pub fn to_cli_error_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.cli_response())
    }

    pub fn log_response(&self) -> ErrorLogResponse {
        ErrorLogResponse {
            code: self.identity.code.clone(),
            category: self.identity.category,
            reason: self.identity.reason.clone(),
            detail: self.report.detail.clone(),
            operation: self.report.want.clone(),
            path: self.report.path.clone(),
            visibility: self.decision.visibility,
            hints: self
                .decision
                .default_hints
                .iter()
                .map(|hint| (*hint).to_string())
                .collect(),
            root_metadata: self.report.root_metadata.clone(),
            context: self.report.context.clone(),
            source_frames: self.report.source_frames.clone(),
        }
    }

    #[cfg(feature = "serde_json")]
    pub fn to_log_error_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.log_response())
    }

    pub fn rpc_response(&self) -> ErrorRpcResponse {
        ErrorRpcResponse {
            status: self.decision.http_status,
            code: self.identity.code.clone(),
            category: self.identity.category,
            reason: self.identity.reason.clone(),
            detail: match self.decision.visibility {
                Visibility::Public => self.report.detail.clone(),
                Visibility::Internal => None,
            },
            visibility: self.decision.visibility,
            hints: self
                .decision
                .default_hints
                .iter()
                .map(|hint| (*hint).to_string())
                .collect(),
            retryable: self.decision.retryable,
        }
    }

    #[cfg(feature = "serde_json")]
    pub fn to_rpc_error_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.rpc_response())
    }
}

fn redact_optional_text(
    key: Option<&str>,
    value: Option<&str>,
    policy: &impl RedactPolicy,
) -> Option<String> {
    value.and_then(|value| policy.redact_value(key, value))
}

fn redact_context(mut ctx: OperationContext, policy: &impl RedactPolicy) -> OperationContext {
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

    ctx.context_mut_for_report().items = redacted_items;
    let redacted_want = redact_optional_text(Some("want"), ctx.target().as_deref(), policy);
    let redacted_action = redact_optional_text(Some("action"), ctx.action().as_deref(), policy);
    let redacted_locator = redact_optional_text(Some("locator"), ctx.locator().as_deref(), policy);
    let redacted_path = ctx
        .path()
        .iter()
        .filter_map(|segment| redact_optional_text(Some("path"), Some(segment.as_str()), policy))
        .collect::<Vec<_>>();
    ctx.replace_target_for_report(redacted_want);
    ctx.replace_action_for_report(redacted_action);
    ctx.replace_locator_for_report(redacted_locator);
    ctx.replace_path_for_report(redacted_path);
    ctx.replace_metadata_for_report(redact_metadata(ctx.metadata(), policy));
    ctx
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
        ContextRecord, ErrorCategory, ErrorCode, ErrorIdentitySnapshot, OperationContext,
        SourceFrame, StableErrorIdentity, StructError, UvsReason,
    };

    use super::{
        DefaultErrorPolicy, ErrorCliResponse, ErrorHttpResponse, ErrorLogResponse, ErrorPolicy,
        ErrorPolicyDecision, ErrorPolicySnapshot, ErrorPolicyView, ErrorRenderer, ErrorReport,
        ErrorRpcResponse, RedactPolicy, RenderMode, TextReportRenderer, Visibility,
        CLI_ERROR_RESPONSE_FIELDS, HTTP_ERROR_RESPONSE_FIELDS, LOG_ERROR_RESPONSE_FIELDS,
        POLICY_DECISION_FIELDS, POLICY_SNAPSHOT_TOP_LEVEL_FIELDS, RPC_ERROR_RESPONSE_FIELDS,
    };
    use crate::{
        StableSnapshotContextFrame, StableSnapshotSourceFrame, StableStructErrorSnapshot,
        STABLE_SNAPSHOT_SCHEMA_VERSION,
    };

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

    impl ErrorCode for TestReason {
        fn error_code(&self) -> i32 {
            match self {
                TestReason::TestError => 1001,
                TestReason::Uvs(reason) => reason.error_code(),
            }
        }
    }

    impl StableErrorIdentity for TestReason {
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

        assert_eq!(report.reason, "system error");
        assert_eq!(
            report.root_metadata.get_str("component.name"),
            Some("engine")
        );
        assert_eq!(
            report.source_frames[0].metadata.get_str("config.kind"),
            Some("sink_defaults")
        );
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
        let via_borrowed = ErrorReport::from(&err);
        let via_owned = ErrorReport::from(err);

        assert_eq!(via_borrowed, via_method);
        assert_eq!(via_owned, via_method);
    }

    #[test]
    fn test_report_from_stable_snapshot_matches_report_methods() {
        let stable = StableStructErrorSnapshot {
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
                metadata: crate::ErrorMetadata::new(),
            }],
            root_metadata: crate::ErrorMetadata::new(),
            source_frames: vec![StableSnapshotSourceFrame {
                index: 0,
                message: "db unavailable".to_string(),
                error_code: None,
                reason: None,
                want: Some("load config".to_string()),
                path: Some("load config / read".to_string()),
                detail: Some("inner detail".to_string()),
                metadata: crate::ErrorMetadata::new(),
                is_root_cause: true,
            }],
        };

        let via_method = stable.report();
        let via_borrowed = ErrorReport::from(&stable);
        let via_owned = ErrorReport::from(stable);

        assert_eq!(via_borrowed, via_method);
        assert_eq!(via_owned, via_method);
    }

    #[test]
    fn test_report_verbose_render_includes_metadata() {
        let report = ErrorReport {
            reason: "test error".to_string(),
            detail: Some("failed".to_string()),
            position: None,
            want: Some("load".to_string()),
            path: Some("load / parse".to_string()),
            context: vec![],
            root_metadata: {
                let mut metadata = crate::ErrorMetadata::new();
                metadata.insert("component.name", "engine");
                metadata
            },
            source_frames: vec![SourceFrame {
                index: 0,
                message: "inner".to_string(),
                display: None,
                debug: "inner".to_string(),
                type_name: None,
                error_code: None,
                reason: None,
                want: None,
                path: None,
                detail: None,
                metadata: {
                    let mut metadata = crate::ErrorMetadata::new();
                    metadata.insert("config.kind", "sink_defaults");
                    metadata
                },
                is_root_cause: true,
            }],
        };

        let rendered = report.render(RenderMode::Verbose);
        assert!(rendered.contains("root_metadata"));
        assert!(rendered.contains("component.name"));
        assert!(rendered.contains("config.kind"));
    }

    #[test]
    fn test_text_report_renderer_matches_existing_render_output() {
        let report = ErrorReport {
            reason: "test error".to_string(),
            detail: Some("failed".to_string()),
            position: None,
            want: Some("load".to_string()),
            path: Some("load / parse".to_string()),
            context: vec![],
            root_metadata: crate::ErrorMetadata::new(),
            source_frames: vec![],
        };

        let renderer = TextReportRenderer::new(RenderMode::Verbose);
        let via_renderer = renderer.render(&report);
        let via_method = report.render(RenderMode::Verbose);

        assert_eq!(via_renderer, via_method);
    }

    #[test]
    fn test_render_with_uses_custom_renderer() {
        struct ReasonOnlyRenderer;

        impl ErrorRenderer for ReasonOnlyRenderer {
            type Output = String;

            fn render(&self, report: &ErrorReport) -> Self::Output {
                format!("only:{}", report.reason)
            }
        }

        let report = ErrorReport {
            reason: "test error".to_string(),
            detail: Some("failed".to_string()),
            position: None,
            want: None,
            path: None,
            context: vec![],
            root_metadata: crate::ErrorMetadata::new(),
            source_frames: vec![],
        };

        assert_eq!(report.render_with(ReasonOnlyRenderer), "only:test error");
    }

    #[test]
    fn test_default_error_policy_maps_category_to_http_status_and_visibility() {
        let policy = DefaultErrorPolicy;
        let biz_identity = ErrorIdentitySnapshot {
            code: "biz.validation_error".to_string(),
            category: ErrorCategory::Biz,
            reason: "validation error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
        };
        let sys_identity = ErrorIdentitySnapshot {
            code: "sys.io_error".to_string(),
            category: ErrorCategory::Sys,
            reason: "system error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
        };

        assert_eq!(policy.http_status(&biz_identity), 400);
        assert_eq!(policy.http_status(&sys_identity), 500);
        assert_eq!(policy.visibility(&biz_identity), Visibility::Public);
        assert_eq!(policy.visibility(&sys_identity), Visibility::Internal);
        assert_eq!(
            policy.default_hints(&sys_identity),
            ["check filesystem state", "verify file permissions"]
        );
        assert_eq!(
            policy.decide(&sys_identity),
            ErrorPolicyDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec!["check filesystem state", "verify file permissions"],
                retryable: false,
            }
        );
    }

    #[test]
    fn test_policy_view_uses_explicit_identity_without_report_side_guessing() {
        let report = ErrorReport {
            reason: "system error".to_string(),
            detail: Some("disk offline".to_string()),
            position: None,
            want: Some("load config".to_string()),
            path: Some("load config".to_string()),
            context: vec![],
            root_metadata: crate::ErrorMetadata::new(),
            source_frames: vec![],
        };
        let identity = ErrorIdentitySnapshot {
            code: "biz.validation_error".to_string(),
            category: ErrorCategory::Biz,
            reason: "validation error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
        };
        let policy = DefaultErrorPolicy;
        let view = report.policy_view(identity.clone());

        assert_eq!(view.identity(), &identity);
        assert_eq!(view.http_status(&policy), 400);
        assert_eq!(view.visibility(&policy), Visibility::Public);
        assert_eq!(view.report().reason, "system error");
    }

    #[test]
    fn test_struct_error_policy_report_uses_real_stable_identity() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("engine bootstrap failed")
            .with_context(OperationContext::doing("start engine"));

        let policy_view = err.policy_report();
        let policy = DefaultErrorPolicy;

        assert_eq!(policy_view.identity().code, "sys.io_error");
        assert_eq!(policy_view.identity().category, ErrorCategory::Sys);
        assert_eq!(policy_view.http_status(&policy), 500);
        assert_eq!(
            policy_view.default_hints(&policy),
            ["check filesystem state", "verify file permissions"]
        );
        assert_eq!(
            policy_view.decision(&policy),
            ErrorPolicyDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec!["check filesystem state", "verify file permissions"],
                retryable: false,
            }
        );
        assert_eq!(policy_view.report().reason, "system error");
    }

    #[test]
    fn test_policy_view_render_with_uses_underlying_report() {
        let report = ErrorReport {
            reason: "test error".to_string(),
            detail: Some("failed".to_string()),
            position: None,
            want: None,
            path: None,
            context: vec![],
            root_metadata: crate::ErrorMetadata::new(),
            source_frames: vec![],
        };
        let view = ErrorPolicyView::new(
            ErrorIdentitySnapshot {
                code: "test.test_error".to_string(),
                category: ErrorCategory::Logic,
                reason: "test error".to_string(),
                detail: None,
                position: None,
                want: None,
                path: None,
            },
            report.clone(),
        );

        assert_eq!(
            view.render_with(TextReportRenderer::new(RenderMode::Compact)),
            report.render(RenderMode::Compact)
        );
    }

    #[test]
    fn test_policy_view_snapshot_contains_identity_decision_and_report() {
        let report = ErrorReport {
            reason: "test error".to_string(),
            detail: Some("failed".to_string()),
            position: None,
            want: None,
            path: None,
            context: vec![],
            root_metadata: crate::ErrorMetadata::new(),
            source_frames: vec![],
        };
        let identity = ErrorIdentitySnapshot {
            code: "biz.validation_error".to_string(),
            category: ErrorCategory::Biz,
            reason: "validation error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
        };
        let view = ErrorPolicyView::new(identity.clone(), report.clone());

        assert_eq!(
            view.snapshot(&DefaultErrorPolicy),
            ErrorPolicySnapshot {
                identity,
                decision: ErrorPolicyDecision {
                    http_status: 400,
                    visibility: Visibility::Public,
                    default_hints: vec![],
                    retryable: false,
                },
                report,
            }
        );
    }

    #[test]
    fn test_report_decision_uses_policy_identity_fallback() {
        let report = ErrorReport {
            reason: "configuration error".to_string(),
            detail: Some("invalid config".to_string()),
            position: None,
            want: None,
            path: None,
            context: vec![],
            root_metadata: crate::ErrorMetadata::new(),
            source_frames: vec![],
        };

        assert_eq!(
            report.decision(&DefaultErrorPolicy),
            ErrorPolicyDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec![],
                retryable: false,
            }
        );
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_policy_snapshot_json_contains_identity_decision_and_report_sections() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("engine bootstrap failed")
            .with_context(OperationContext::doing("start engine"));

        let json_value = err
            .report()
            .to_policy_report_json(&DefaultErrorPolicy)
            .expect("serialize policy snapshot");

        assert_eq!(
            json_value["identity"]["code"],
            serde_json::json!("report.unclassified")
        );
        assert_eq!(
            json_value["decision"]["http_status"],
            serde_json::json!(500)
        );
        assert_eq!(
            json_value["decision"]["visibility"],
            serde_json::json!("Internal")
        );
        assert_eq!(
            json_value["report"]["reason"],
            serde_json::json!("system error")
        );
        assert_eq!(
            json_value["report"]["detail"],
            serde_json::json!("engine bootstrap failed")
        );
    }

    #[test]
    fn test_policy_snapshot_schema_constants_match_expected_fields() {
        assert_eq!(
            POLICY_SNAPSHOT_TOP_LEVEL_FIELDS,
            &["identity", "decision", "report"]
        );
        assert_eq!(
            POLICY_DECISION_FIELDS,
            &["http_status", "visibility", "default_hints", "retryable"]
        );
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_policy_snapshot_json_keys_match_schema_constants() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("engine bootstrap failed");

        let json_value = err
            .report()
            .to_policy_snapshot_json(&DefaultErrorPolicy)
            .expect("serialize policy snapshot");

        let mut top_level = json_value
            .as_object()
            .expect("policy snapshot object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        top_level.sort();

        let mut expected_top_level = POLICY_SNAPSHOT_TOP_LEVEL_FIELDS
            .iter()
            .map(|field| field.to_string())
            .collect::<Vec<_>>();
        expected_top_level.sort();

        let mut decision_fields = json_value["decision"]
            .as_object()
            .expect("decision object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        decision_fields.sort();

        let mut expected_decision_fields = POLICY_DECISION_FIELDS
            .iter()
            .map(|field| field.to_string())
            .collect::<Vec<_>>();
        expected_decision_fields.sort();

        assert_eq!(top_level, expected_top_level);
        assert_eq!(decision_fields, expected_decision_fields);
    }

    #[test]
    fn test_http_response_projection_uses_detail_for_public_visibility() {
        let err = StructError::from(TestReason::Uvs(UvsReason::business_error()))
            .with_detail("order state invalid");

        assert_eq!(
            err.http_response(&DefaultErrorPolicy),
            ErrorHttpResponse {
                status: 400,
                code: "biz.business_error".to_string(),
                category: ErrorCategory::Biz,
                message: "order state invalid".to_string(),
                visibility: Visibility::Public,
                hints: vec![],
            }
        );
    }

    #[test]
    fn test_http_response_projection_uses_reason_for_internal_visibility() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("disk offline");

        assert_eq!(
            err.http_response(&DefaultErrorPolicy),
            ErrorHttpResponse {
                status: 500,
                code: "sys.io_error".to_string(),
                category: ErrorCategory::Sys,
                message: "system error".to_string(),
                visibility: Visibility::Internal,
                hints: vec![
                    "check filesystem state".to_string(),
                    "verify file permissions".to_string(),
                ],
            }
        );
    }

    #[test]
    fn test_http_error_response_schema_constants_match_expected_fields() {
        assert_eq!(
            HTTP_ERROR_RESPONSE_FIELDS,
            &[
                "status",
                "code",
                "category",
                "message",
                "visibility",
                "hints"
            ]
        );
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_http_error_json_keys_match_schema_constants() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("disk offline");

        let json_value = err
            .report()
            .to_http_error_json(&DefaultErrorPolicy)
            .expect("serialize http error");

        let mut keys = json_value
            .as_object()
            .expect("http error object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();

        let mut expected = HTTP_ERROR_RESPONSE_FIELDS
            .iter()
            .map(|field| field.to_string())
            .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(keys, expected);
        assert_eq!(json_value["status"], serde_json::json!(500));
        assert_eq!(json_value["code"], serde_json::json!("report.unclassified"));
        assert_eq!(json_value["message"], serde_json::json!("system error"));
    }

    #[test]
    fn test_cli_response_projection_contains_summary_detail_and_hints() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("disk offline");

        assert_eq!(
            err.cli_response(&DefaultErrorPolicy),
            ErrorCliResponse {
                code: "sys.io_error".to_string(),
                category: ErrorCategory::Sys,
                summary: "system error: disk offline".to_string(),
                detail: "reason: system error\ndetail: disk offline".to_string(),
                visibility: Visibility::Internal,
                hints: vec![
                    "check filesystem state".to_string(),
                    "verify file permissions".to_string(),
                ],
            }
        );
    }

    #[test]
    fn test_cli_error_response_schema_constants_match_expected_fields() {
        assert_eq!(
            CLI_ERROR_RESPONSE_FIELDS,
            &[
                "code",
                "category",
                "summary",
                "detail",
                "visibility",
                "hints"
            ]
        );
    }

    #[test]
    fn test_log_response_projection_contains_machine_facing_diagnostics() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("disk offline")
            .with_std_source(std::io::Error::other("root cause"))
            .with_context(OperationContext::doing("load config").with_meta("tenant", "acme"));
        let mut root_metadata = crate::ErrorMetadata::new();
        root_metadata.insert("tenant", "acme");

        assert_eq!(
            err.log_response(&DefaultErrorPolicy),
            ErrorLogResponse {
                code: "sys.io_error".to_string(),
                category: ErrorCategory::Sys,
                reason: "system error".to_string(),
                detail: Some("disk offline".to_string()),
                operation: Some("load config".to_string()),
                path: Some("load config".to_string()),
                visibility: Visibility::Internal,
                hints: vec![
                    "check filesystem state".to_string(),
                    "verify file permissions".to_string(),
                ],
                root_metadata,
                context: vec![OperationContext::doing("load config").with_meta("tenant", "acme")],
                source_frames: err.report().source_frames,
            }
        );
    }

    #[test]
    fn test_log_error_response_schema_constants_match_expected_fields() {
        assert_eq!(
            LOG_ERROR_RESPONSE_FIELDS,
            &[
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
                "source_frames"
            ]
        );
    }

    #[test]
    fn test_rpc_response_projection_hides_internal_detail_and_marks_retryable() {
        let err = StructError::from(TestReason::Uvs(UvsReason::timeout_error()))
            .with_detail("downstream rpc timeout");

        assert_eq!(
            err.rpc_response(&DefaultErrorPolicy),
            ErrorRpcResponse {
                status: 500,
                code: "sys.timeout".to_string(),
                category: ErrorCategory::Sys,
                reason: "timeout error".to_string(),
                detail: None,
                visibility: Visibility::Internal,
                hints: vec![
                    "retry later".to_string(),
                    "inspect downstream service latency".to_string(),
                ],
                retryable: true,
            }
        );
    }

    #[test]
    fn test_rpc_response_projection_keeps_public_detail() {
        let err = StructError::from(TestReason::Uvs(UvsReason::business_error()))
            .with_detail("order state invalid");

        assert_eq!(
            err.rpc_response(&DefaultErrorPolicy),
            ErrorRpcResponse {
                status: 400,
                code: "biz.business_error".to_string(),
                category: ErrorCategory::Biz,
                reason: "business logic error".to_string(),
                detail: Some("order state invalid".to_string()),
                visibility: Visibility::Public,
                hints: vec![],
                retryable: false,
            }
        );
    }

    #[test]
    fn test_rpc_error_response_schema_constants_match_expected_fields() {
        assert_eq!(
            RPC_ERROR_RESPONSE_FIELDS,
            &[
                "status",
                "code",
                "category",
                "reason",
                "detail",
                "visibility",
                "hints",
                "retryable"
            ]
        );
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_cli_error_json_keys_match_schema_constants() {
        let err = StructError::from(TestReason::Uvs(UvsReason::business_error()))
            .with_detail("order state invalid");

        let json_value = err
            .report()
            .to_cli_error_json(&DefaultErrorPolicy)
            .expect("serialize cli error");

        let mut keys = json_value
            .as_object()
            .expect("cli error object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();

        let mut expected = CLI_ERROR_RESPONSE_FIELDS
            .iter()
            .map(|field| field.to_string())
            .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(keys, expected);
        assert_eq!(json_value["code"], serde_json::json!("report.unclassified"));
        assert_eq!(
            json_value["summary"],
            serde_json::json!("business logic error: order state invalid")
        );
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_log_error_json_keys_match_schema_constants() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("disk offline")
            .with_context(OperationContext::doing("load config"));

        let json_value = err
            .report()
            .to_log_error_json(&DefaultErrorPolicy)
            .expect("serialize log error");

        let mut keys = json_value
            .as_object()
            .expect("log error object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();

        let mut expected = LOG_ERROR_RESPONSE_FIELDS
            .iter()
            .map(|field| field.to_string())
            .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(keys, expected);
        assert_eq!(json_value["code"], serde_json::json!("report.unclassified"));
        assert_eq!(json_value["reason"], serde_json::json!("system error"));
        assert_eq!(json_value["operation"], serde_json::json!("load config"));
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_rpc_error_json_keys_match_schema_constants() {
        let err = StructError::from(TestReason::Uvs(UvsReason::timeout_error()))
            .with_detail("downstream rpc timeout");

        let json_value = err
            .report()
            .to_rpc_error_json(&DefaultErrorPolicy)
            .expect("serialize rpc error");

        let mut keys = json_value
            .as_object()
            .expect("rpc error object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();

        let mut expected = RPC_ERROR_RESPONSE_FIELDS
            .iter()
            .map(|field| field.to_string())
            .collect::<Vec<_>>();
        expected.sort();

        assert_eq!(keys, expected);
        assert_eq!(json_value["code"], serde_json::json!("report.unclassified"));
        assert_eq!(json_value["retryable"], serde_json::json!(false));
        assert_eq!(json_value["detail"], serde_json::Value::Null);
    }

    #[test]
    fn test_report_redaction_masks_sensitive_fields() {
        let err = StructError::from(TestReason::TestError)
            .with_detail("token=abc")
            .with_std_source(std::io::Error::other("token=abc"))
            .with_context(OperationContext::doing("load").with_meta("config.secret", "abc"));

        let rendered = err.render_redacted(RenderMode::Verbose, &TestPolicy);
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("abc"));
    }

    #[test]
    fn test_report_redaction_masks_source_frame_message() {
        let err = StructError::from(TestReason::TestError)
            .with_std_source(std::io::Error::other("https://svc.local?token=abc"));

        let rendered = err.render_redacted(RenderMode::Verbose, &TestPolicy);

        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("svc.local"));
        assert!(!rendered.contains("token=abc"));
    }

    #[test]
    fn test_report_redaction_masks_source_frame_display() {
        let report = ErrorReport {
            reason: "test error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
            context: vec![],
            root_metadata: crate::ErrorMetadata::new(),
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
                metadata: crate::ErrorMetadata::new(),
                is_root_cause: true,
            }],
        };

        let redacted = report.redacted(&TestPolicy);
        assert_eq!(
            redacted.source_frames[0].display.as_deref(),
            Some("<redacted>")
        );
        assert!(!redacted.source_frames[0]
            .display
            .as_deref()
            .unwrap()
            .contains("token=abc"));
    }

    #[test]
    fn test_report_redaction_masks_source_frame_debug() {
        let report = ErrorReport {
            reason: "test error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
            context: vec![],
            root_metadata: crate::ErrorMetadata::new(),
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
                metadata: crate::ErrorMetadata::new(),
                is_root_cause: true,
            }],
        };

        let redacted = report.redacted(&TestPolicy);
        assert_eq!(redacted.source_frames[0].debug, "<redacted>");
        assert!(!redacted.source_frames[0].debug.contains("token=abc"));
    }

    #[test]
    fn test_report_redaction_masks_root_and_frame_paths() {
        let report = ErrorReport {
            reason: "test error".to_string(),
            detail: None,
            position: Some("/srv/app/config.toml:10".to_string()),
            want: Some("load /srv/app/config.toml".to_string()),
            path: Some("load /srv/app/config.toml / parse".to_string()),
            context: vec![OperationContext::at("/srv/app/config.toml")],
            root_metadata: crate::ErrorMetadata::new(),
            source_frames: vec![SourceFrame {
                index: 0,
                message: "inner".to_string(),
                display: None,
                debug: "debug".to_string(),
                type_name: None,
                error_code: None,
                reason: None,
                want: Some("open /srv/app/config.toml".to_string()),
                path: Some("open /srv/app/config.toml / read".to_string()),
                detail: None,
                metadata: crate::ErrorMetadata::new(),
                is_root_cause: true,
            }],
        };

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

        let rendered = report.render_redacted(RenderMode::Verbose, &PathPolicy);
        assert!(rendered.contains("<path-redacted>"));
        assert!(!rendered.contains("/srv/app/config.toml"));
    }

    #[test]
    fn test_report_redaction_masks_reason_fields() {
        let report = ErrorReport {
            reason: "tenant secret error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
            context: vec![],
            root_metadata: crate::ErrorMetadata::new(),
            source_frames: vec![SourceFrame {
                index: 0,
                message: "inner".to_string(),
                display: None,
                debug: "debug".to_string(),
                type_name: None,
                error_code: None,
                reason: Some("tenant secret source".to_string()),
                want: None,
                path: None,
                detail: None,
                metadata: crate::ErrorMetadata::new(),
                is_root_cause: true,
            }],
        };

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
        assert_eq!(
            redacted.source_frames[0].reason.as_deref(),
            Some("tenant <redacted> source")
        );
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

        let rendered = err.render_redacted(RenderMode::Verbose, &ValueOnlyPolicy);
        assert!(rendered.contains("<detail-redacted>"));
        assert!(rendered.contains("<token-redacted>"));
        assert!(rendered.contains("<secret-redacted>"));
        assert!(!rendered.contains("token=abc"));
        assert!(!rendered.contains("token: abc"));
        assert!(!rendered.contains("config.secret\": \"abc"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_report_serialization_supports_structured_export() {
        let source = StructError::from(TestReason::TestError).with_context(
            OperationContext::doing("load defaults").with_meta("config.kind", "sink_defaults"),
        );
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
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

    #[cfg(feature = "serde")]
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
