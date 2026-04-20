use crate::{DomainReason, StructError};

use super::{ErrorMetadata, MetadataValue, OperationContext, SourceFrame};

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
        ErrorReport {
            reason: self.reason().to_string(),
            detail: self.detail().clone(),
            position: self.position().clone(),
            want: self.target_main(),
            path: self.target_path(),
            context: self.contexts().to_vec(),
            root_metadata: self.context_metadata(),
            source_frames: self.source_frames().to_vec(),
        }
    }

    pub fn report_redacted(&self, policy: &impl RedactPolicy) -> ErrorReport {
        self.report().redacted(policy)
    }
}

impl ErrorReport {
    pub fn render(&self, mode: RenderMode) -> String {
        match mode {
            RenderMode::Compact => self.render_compact(),
            RenderMode::Verbose => self.render_verbose(),
        }
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
    let redacted_path = ctx
        .path()
        .iter()
        .filter_map(|segment| redact_optional_text(Some("path"), Some(segment.as_str()), policy))
        .collect::<Vec<_>>();
    ctx.replace_target_for_report(redacted_want);
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
        ContextRecord, ErrorCode, ErrorWith, OperationContext, SourceFrame, StructError, UvsReason,
    };

    use super::{ErrorReport, RedactPolicy, RenderMode};

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

    #[test]
    fn test_report_contains_root_and_source_data() {
        let source = StructError::from(TestReason::TestError).with(
            OperationContext::want("load defaults").with_meta("config.kind", "sink_defaults"),
        );
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with(OperationContext::want("start engine").with_meta("component.name", "engine"))
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
    fn test_report_redaction_masks_sensitive_fields() {
        let err = StructError::from(TestReason::TestError)
            .with_detail("token=abc")
            .with_source(std::io::Error::other("token=abc"))
            .with(OperationContext::want("load").with_meta("config.secret", "abc"));

        let rendered = err.render_redacted(RenderMode::Verbose, &TestPolicy);
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("abc"));
    }

    #[test]
    fn test_report_redaction_masks_source_frame_message() {
        let err = StructError::from(TestReason::TestError)
            .with_source(std::io::Error::other("https://svc.local?token=abc"));

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
            context: vec![OperationContext::want("load /srv/app/config.toml")],
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
                    Some("position") | Some("want") | Some("path") => {
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
            .with({
                let mut ctx = OperationContext::want("load");
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
        let source = StructError::from(TestReason::TestError).with(
            OperationContext::want("load defaults").with_meta("config.kind", "sink_defaults"),
        );
        let err = StructError::from(TestReason::Uvs(UvsReason::system_error()))
            .with(OperationContext::want("start engine").with_meta("component.name", "engine"))
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
            .with_source(std::io::Error::other("token=abc"))
            .with(OperationContext::want("load").with_meta("config.secret", "abc"));

        let json_value =
            serde_json::to_value(err.report_redacted(&TestPolicy)).expect("serialize redacted");

        let encoded = serde_json::to_string(&json_value).expect("json string");
        assert!(encoded.contains("<redacted>"));
        assert!(!encoded.contains("token=abc"));
        assert!(!encoded.contains("\"abc\""));
    }
}
