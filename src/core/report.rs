use crate::{core::DomainReason, ErrorCategory, StructError};

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
    pub want: Option<String>,
    pub path: Option<String>,
    pub category: ErrorCategory,
    pub code: String,
    pub context: Vec<OperationContext>,
    pub root_metadata: ErrorMetadata,
    pub source_frames: Vec<SourceFrame>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Internal,
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
    pub report: DiagnosticReport,
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

impl<T: DomainReason + ErrorIdentityProvider> StructError<T> {
    pub fn report(&self) -> DiagnosticReport {
        self.snapshot().report()
    }

    pub fn into_report(self) -> DiagnosticReport {
        self.into_snapshot().into_report()
    }

    pub fn report_redacted(&self, policy: &impl RedactPolicy) -> DiagnosticReport {
        self.report().redacted(policy)
    }

    pub fn render(&self) -> String {
        self.report().render()
    }

    pub fn render_redacted(&self, policy: &impl RedactPolicy) -> String {
        self.report().render_redacted(policy)
    }
}

impl<T> StructError<T>
where
    T: DomainReason + ErrorIdentityProvider,
{
    pub fn exposure_snapshot(
        &self,
        exposure_policy: &impl ExposurePolicy,
    ) -> ErrorProtocolSnapshot {
        let identity = self.identity_snapshot();
        ErrorProtocolSnapshot {
            decision: exposure_policy.decide(&identity),
            identity,
            report: self.report(),
        }
    }

    pub fn into_exposure_snapshot(
        self,
        exposure_policy: &impl ExposurePolicy,
    ) -> ErrorProtocolSnapshot {
        let identity = self.identity_snapshot();
        let report = self.into_report();
        let decision = exposure_policy.decide(&identity);
        ErrorProtocolSnapshot {
            identity,
            decision,
            report,
        }
    }

    pub fn render_user_debug(&self, exposure_policy: &impl ExposurePolicy) -> String {
        self.exposure_snapshot(exposure_policy).render_user_debug()
    }

    pub fn render_user_debug_redacted(
        &self,
        exposure_policy: &impl ExposurePolicy,
        redact_policy: &impl RedactPolicy,
    ) -> String {
        self.exposure_snapshot(exposure_policy)
            .render_user_debug_redacted(redact_policy)
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

impl<T: DomainReason + ErrorIdentityProvider> From<&StructError<T>> for DiagnosticReport {
    fn from(value: &StructError<T>) -> Self {
        value.report()
    }
}

impl<T: DomainReason + ErrorIdentityProvider> From<StructError<T>> for DiagnosticReport {
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
    pub fn render(&self) -> String {
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

    pub fn redacted(&self, policy: &impl RedactPolicy) -> Self {
        Self {
            reason: redact_required_text(Some("reason"), &self.reason, policy),
            detail: redact_optional_text(Some("detail"), self.detail.as_deref(), policy),
            position: redact_optional_text(Some("position"), self.position.as_deref(), policy),
            want: redact_optional_text(Some("want"), self.want.as_deref(), policy),
            path: redact_optional_text(Some("path"), self.path.as_deref(), policy),
            category: self.category,
            code: self.code.clone(),
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

    pub fn render_redacted(&self, policy: &impl RedactPolicy) -> String {
        self.redacted(policy).render()
    }

    pub fn render_compact(&self) -> String {
        let mut out = self.reason.clone();
        if let Some(detail) = &self.detail {
            out.push_str(": ");
            out.push_str(detail);
        }
        out
    }

}

impl DiagnosticReport {
    pub fn exposure_identity(&self) -> ErrorIdentity {
        ErrorIdentity {
            code: self.code.clone(),
            category: self.category,
            reason: self.reason.clone(),
            detail: self.detail.clone(),
            position: self.position.clone(),
            want: self.want.clone(),
            path: self.path.clone(),
        }
    }

    pub fn http_status(&self, exposure_policy: &impl ExposurePolicy) -> u16 {
        exposure_policy.http_status(&self.exposure_identity())
    }

    pub fn visibility(&self, exposure_policy: &impl ExposurePolicy) -> Visibility {
        exposure_policy.visibility(&self.exposure_identity())
    }

    pub fn default_hints(&self, exposure_policy: &impl ExposurePolicy) -> &'static [&'static str] {
        exposure_policy.default_hints(&self.exposure_identity())
    }

    pub fn decision(&self, exposure_policy: &impl ExposurePolicy) -> ExposureDecision {
        exposure_policy.decide(&self.exposure_identity())
    }

    pub fn exposure_snapshot(
        &self,
        exposure_policy: &impl ExposurePolicy,
    ) -> ErrorProtocolSnapshot {
        ErrorProtocolSnapshot {
            identity: self.exposure_identity(),
            decision: self.decision(exposure_policy),
            report: self.clone(),
        }
    }

    #[cfg(feature = "serde_json")]
    pub fn to_exposure_snapshot_json(
        &self,
        exposure_policy: &impl ExposurePolicy,
    ) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self.exposure_snapshot(exposure_policy))
    }
}

impl ErrorProtocolSnapshot {
    pub fn render_user_debug(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "code          : {} ({:?})",
            self.identity.code, self.identity.category
        ));

        if let Some(detail) = self.report.detail.as_deref() {
            lines.push(format!("detail        : {detail}"));
        } else {
            lines.push(format!("detail        : {}", self.identity.reason));
        }

        lines.push(format!(
            "http          : {} {:?} retryable={}",
            self.decision.http_status, self.decision.visibility, self.decision.retryable
        ));

        if let Some(path) = self.identity.path.as_deref() {
            lines.push(format!("path          : {path}"));
        }

        let context_summary = self
            .report
            .context
            .iter()
            .flat_map(|ctx| ctx.context().items.iter())
            .map(|(key, value)| format!("{key}={value:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        if !context_summary.is_empty() {
            lines.push(format!("context       : {context_summary}"));
        }

        if let Some(component) = self.report.root_metadata.get_str("component.name") {
            lines.push(format!("component     : {component}"));
        } else if !self.report.root_metadata.is_empty() {
            lines.push(format!(
                "metadata      : {}",
                format_metadata_summary(&self.report.root_metadata)
            ));
        }

        if let Some(source) = root_cause_source_frame(&self.report.source_frames) {
            lines.push(format!("source        : {}", source.message));
        }

        lines.join("\n")
    }

    pub fn render_user_debug_redacted(&self, redact_policy: &impl RedactPolicy) -> String {
        self.redacted(redact_policy).render_user_debug()
    }

    #[cfg(feature = "serde_json")]
    pub fn to_http_error_json(&self) -> serde_json::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "status": self.decision.http_status,
            "code": self.identity.code,
            "category": format!("{:?}", self.identity.category),
            "message": match self.decision.visibility {
                Visibility::Public => self.report.detail.clone()
                    .unwrap_or_else(|| self.identity.reason.clone()),
                Visibility::Internal => self.identity.reason.clone(),
            },
            "visibility": format!("{:?}", self.decision.visibility),
            "hints": self.decision.default_hints,
        }))
    }

    #[cfg(feature = "serde_json")]
    pub fn to_cli_error_json(&self) -> serde_json::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "code": self.identity.code,
            "category": format!("{:?}", self.identity.category),
            "summary": self.report.render_compact(),
            "detail": self.report.render(),
            "visibility": format!("{:?}", self.decision.visibility),
            "hints": self.decision.default_hints,
        }))
    }

    #[cfg(feature = "serde_json")]
    pub fn to_log_error_json(&self) -> serde_json::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "code": self.identity.code,
            "category": format!("{:?}", self.identity.category),
            "reason": self.identity.reason,
            "detail": self.report.detail,
            "operation": self.report.want,
            "path": self.report.path,
            "visibility": format!("{:?}", self.decision.visibility),
            "hints": self.decision.default_hints,
            "root_metadata": self.report.root_metadata,
            "context": self.report.context,
            "source_frames": self.report.source_frames,
        }))
    }

    #[cfg(feature = "serde_json")]
    pub fn to_rpc_error_json(&self) -> serde_json::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "status": self.decision.http_status,
            "code": self.identity.code,
            "category": format!("{:?}", self.identity.category),
            "reason": self.identity.reason,
            "detail": match self.decision.visibility {
                Visibility::Public => self.report.detail.clone(),
                Visibility::Internal => None,
            },
            "visibility": format!("{:?}", self.decision.visibility),
            "hints": self.decision.default_hints,
            "retryable": self.decision.retryable,
        }))
    }

    pub fn redacted(&self, policy: &impl RedactPolicy) -> Self {
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
            report: self.report.redacted(policy),
        }
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
        core::DomainReason,
        core::{
            context::ContextRecord, ErrorIdentity, ErrorMetadata, SourceFrame, StableErrorSnapshot,
            StableSnapshotContextFrame, StableSnapshotSourceFrame,
        },
        ErrorCategory, ErrorCode, ErrorIdentityProvider, OperationContext, StructError, UvsReason,
    };

    use super::{
        DefaultExposurePolicy, DiagnosticReport, ErrorProtocolSnapshot, ExposureDecision,
        ExposurePolicy, RedactPolicy, Visibility,
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
        let report = DiagnosticReport {
            reason: "test error".to_string(),
            detail: Some("failed".to_string()),
            position: None,
            want: Some("load".to_string()),
            path: Some("load / parse".to_string()),
            category: ErrorCategory::Sys,
            code: "test.error".to_string(),
            context: vec![],
            root_metadata: {
                let mut metadata = ErrorMetadata::new();
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
                    let mut metadata = ErrorMetadata::new();
                    metadata.insert("config.kind", "sink_defaults");
                    metadata
                },
                is_root_cause: true,
            }],
        };

        let rendered = report.render();

        assert!(rendered.contains("reason: test error"));
        assert!(rendered.contains("detail: failed"));
        assert!(rendered.contains("root_metadata"));
        assert!(rendered.contains("source_frames"));
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
        assert_eq!(snapshot.report.reason, "system error");
    }

    #[test]
    fn test_report_decision_uses_exposure_identity_fallback() {
        let report = DiagnosticReport {
            reason: "configuration error".to_string(),
            detail: Some("invalid config".to_string()),
            position: None,
            want: None,
            path: None,
            category: ErrorCategory::Sys,
            code: "test.error".to_string(),
            context: vec![],
            root_metadata: ErrorMetadata::new(),
            source_frames: vec![],
        };

        assert_eq!(
            report.decision(&DefaultExposurePolicy),
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

        let json_value = err
            .report()
            .to_exposure_snapshot_json(&DefaultExposurePolicy)
            .expect("serialize exposure snapshot");

        assert_eq!(
            json_value["identity"]["code"],
            serde_json::json!("sys.io_error")
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

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_exposure_snapshot_json_keys_match_expected_shape() {
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with_detail("engine bootstrap failed");

        let json_value = err
            .report()
            .to_exposure_snapshot_json(&DefaultExposurePolicy)
            .expect("serialize exposure snapshot");

        let mut top_level = json_value
            .as_object()
            .expect("exposure snapshot object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        top_level.sort();

        let mut expected_top_level = ["identity", "decision", "report"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        expected_top_level.sort();

        let mut decision_fields = json_value["decision"]
            .as_object()
            .expect("decision object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        decision_fields.sort();

        let mut expected_decision_fields =
            ["http_status", "visibility", "default_hints", "retryable"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
        expected_decision_fields.sort();

        assert_eq!(top_level, expected_top_level);
        assert_eq!(decision_fields, expected_decision_fields);
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
        assert_eq!(json["category"], serde_json::json!("Biz"));
        assert_eq!(json["message"], serde_json::json!("order state invalid"));
        assert_eq!(json["visibility"], serde_json::json!("Public"));
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
        assert_eq!(json["category"], serde_json::json!("Sys"));
        assert_eq!(json["message"], serde_json::json!("system error"));
        assert_eq!(json["visibility"], serde_json::json!("Internal"));
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
        assert_eq!(json["category"], serde_json::json!("Sys"));
        assert_eq!(json["summary"], serde_json::json!("system error: disk offline"));
        assert_eq!(
            json["detail"],
            serde_json::json!("reason: system error\ndetail: disk offline")
        );
        assert_eq!(json["visibility"], serde_json::json!("Internal"));
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
        assert_eq!(json["category"], serde_json::json!("Sys"));
        assert_eq!(json["reason"], serde_json::json!("system error"));
        assert_eq!(json["detail"], serde_json::json!("disk offline"));
        assert_eq!(json["operation"], serde_json::json!("load config"));
        assert_eq!(json["path"], serde_json::json!("load config"));
        assert_eq!(json["visibility"], serde_json::json!("Internal"));
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
        assert_eq!(json["category"], serde_json::json!("Sys"));
        assert_eq!(json["reason"], serde_json::json!("timeout error"));
        assert_eq!(json["detail"], serde_json::json!(null));
        assert_eq!(json["visibility"], serde_json::json!("Internal"));
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
        assert_eq!(json["category"], serde_json::json!("Biz"));
        assert_eq!(json["reason"], serde_json::json!("business logic error"));
        assert_eq!(json["detail"], serde_json::json!("order state invalid"));
        assert_eq!(json["visibility"], serde_json::json!("Public"));
        assert_eq!(json["hints"], serde_json::json!([]));
        assert_eq!(json["retryable"], serde_json::json!(false));
    }

    #[test]
    fn test_exposure_snapshot_render_debug_summary_prefers_detail_path_context_and_component() {
        let snapshot = ErrorProtocolSnapshot {
            identity: ErrorIdentity {
                code: "biz.order_invalid".to_string(),
                category: ErrorCategory::Biz,
                reason: "invalid order".to_string(),
                detail: Some("order text must not be empty".to_string()),
                position: None,
                want: Some("place_order".to_string()),
                path: Some("place_order / parse order".to_string()),
            },
            decision: ExposureDecision {
                http_status: 400,
                visibility: Visibility::Public,
                default_hints: vec![],
                retryable: false,
            },
            report: DiagnosticReport {
                reason: "invalid order".to_string(),
                detail: Some("order text must not be empty".to_string()),
                position: None,
                want: Some("place_order".to_string()),
                path: Some("place_order / parse order".to_string()),
                category: ErrorCategory::Biz,
                code: "biz.order_invalid".to_string(),
                context: vec![{
                    let mut ctx = OperationContext::doing("place_order");
                    ctx.record_field("user_id", "42");
                    ctx.record_field("order.raw", "");
                    ctx.record_meta("component.name", "order_service");
                    ctx
                }],
                root_metadata: {
                    let mut metadata = ErrorMetadata::new();
                    metadata.insert("component.name", "order_service");
                    metadata.insert("trace.secret", "prod-token");
                    metadata
                },
                source_frames: vec![],
            },
        };

        let rendered = snapshot.render_user_debug();

        assert!(rendered.contains("code          : biz.order_invalid (Biz)"));
        assert!(rendered.contains("detail        : order text must not be empty"));
        assert!(rendered.contains("http          : 400 Public retryable=false"));
        assert!(rendered.contains("path          : place_order / parse order"));
        assert!(rendered.contains("context       : user_id=\"42\", order.raw=\"\""));
        assert!(rendered.contains("component     : order_service"));
        assert!(!rendered.contains("trace.secret"));
    }

    #[test]
    fn test_exposure_snapshot_render_debug_summary_falls_back_to_reason_and_source() {
        let snapshot = ErrorProtocolSnapshot {
            identity: ErrorIdentity {
                code: "sys.storage_full".to_string(),
                category: ErrorCategory::Sys,
                reason: "storage full".to_string(),
                detail: None,
                position: None,
                want: Some("place_order".to_string()),
                path: Some("place_order / save order".to_string()),
            },
            decision: ExposureDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec![],
                retryable: false,
            },
            report: DiagnosticReport {
                reason: "storage full".to_string(),
                detail: None,
                position: None,
                want: Some("place_order".to_string()),
                path: Some("place_order / save order".to_string()),
                category: ErrorCategory::Sys,
                code: "sys.storage_full".to_string(),
                context: vec![],
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
        };

        let rendered = snapshot.render_user_debug();

        assert!(rendered.contains("detail        : storage full"));
        assert!(rendered.contains("source        : storage full"));
    }

    #[test]
    fn test_exposure_snapshot_render_debug_summary_prefers_root_cause_source_frame() {
        let snapshot = ErrorProtocolSnapshot {
            identity: ErrorIdentity {
                code: "sys.io_error".to_string(),
                category: ErrorCategory::Sys,
                reason: "system error".to_string(),
                detail: Some("save order failed".to_string()),
                position: None,
                want: Some("place_order".to_string()),
                path: Some("place_order / save order".to_string()),
            },
            decision: ExposureDecision {
                http_status: 500,
                visibility: Visibility::Internal,
                default_hints: vec![],
                retryable: false,
            },
            report: DiagnosticReport {
                reason: "system error".to_string(),
                detail: Some("save order failed".to_string()),
                position: None,
                want: Some("place_order".to_string()),
                path: Some("place_order / save order".to_string()),
                category: ErrorCategory::Sys,
                code: "sys.io_error".to_string(),
                context: vec![],
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
        };

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

        let rendered = err.render_user_debug_redacted(&DefaultExposurePolicy, &TestPolicy);

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
        let report = DiagnosticReport {
            reason: "test error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
            category: ErrorCategory::Sys,
            code: "test.error".to_string(),
            context: vec![],
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
        let report = DiagnosticReport {
            reason: "test error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
            category: ErrorCategory::Sys,
            code: "test.error".to_string(),
            context: vec![],
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
        };

        let redacted = report.redacted(&TestPolicy);
        assert_eq!(redacted.source_frames[0].debug, "<redacted>");
        assert!(!redacted.source_frames[0].debug.contains("token=abc"));
    }

    #[test]
    fn test_report_redaction_masks_root_and_frame_paths() {
        let report = DiagnosticReport {
            reason: "test error".to_string(),
            detail: None,
            position: Some("/srv/app/config.toml:10".to_string()),
            want: Some("load /srv/app/config.toml".to_string()),
            path: Some("load /srv/app/config.toml / parse".to_string()),
            category: ErrorCategory::Sys,
            code: "test.error".to_string(),
            context: vec![OperationContext::at("/srv/app/config.toml")],
            root_metadata: ErrorMetadata::new(),
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
                metadata: ErrorMetadata::new(),
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

        let rendered = report.render_redacted(&PathPolicy);
        assert!(rendered.contains("<path-redacted>"));
        assert!(!rendered.contains("/srv/app/config.toml"));
    }

    #[test]
    fn test_report_redaction_masks_reason_fields() {
        let report = DiagnosticReport {
            reason: "tenant secret error".to_string(),
            detail: None,
            position: None,
            want: None,
            path: None,
            category: ErrorCategory::Sys,
            code: "test.error".to_string(),
            context: vec![],
            root_metadata: ErrorMetadata::new(),
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
                metadata: ErrorMetadata::new(),
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

        let rendered = err.render_redacted(&ValueOnlyPolicy);
        assert!(rendered.contains("<detail-redacted>"));
        assert!(rendered.contains("<token-redacted>"));
        assert!(rendered.contains("<secret-redacted>"));
        assert!(!rendered.contains("token=abc"));
        assert!(!rendered.contains("token: abc"));
        assert!(!rendered.contains("config.secret\": \"abc"));
    }
}
