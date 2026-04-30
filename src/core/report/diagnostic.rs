#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticReport {
    reason: String,
    detail: Option<String>,
    position: Option<String>,
    context: Arc<Vec<OperationContext>>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReportProjectionParts {
    pub path: Option<String>,
    pub root_metadata: ErrorMetadata,
    pub source_frames: Vec<SourceFrame>,
}

impl ReportProjectionParts {
    fn from_identity_skeleton(identity: &ErrorIdentity) -> Self {
        Self {
            path: identity.path.clone(),
            root_metadata: ErrorMetadata::new(),
            source_frames: Vec::new(),
        }
    }

    fn redacted(&self, policy: &impl RedactPolicy) -> Self {
        Self {
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
    /// assert!(report.reason().contains("validation"));
    /// assert_eq!(report.detail(), Some("field `email` is required"));
    /// ```
    pub fn report(&self) -> DiagnosticReport {
        DiagnosticReport::from_parts(
            self.reason().to_string(),
            self.detail().clone(),
            self.position().clone(),
            self.imp().context_arc(),
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
    /// assert!(report.reason().contains("validation"));
    /// assert_eq!(report.detail(), Some("field `email` is required"));
    /// ```
    pub fn into_report(self) -> DiagnosticReport {
        DiagnosticReport::from_parts(
            self.reason().to_string(),
            self.detail().clone(),
            self.position().clone(),
            self.imp().context_arc(),
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
    /// assert_eq!(report.detail(), Some("<redacted>"));
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
    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    pub fn position(&self) -> Option<&str> {
        self.position.as_deref()
    }

    pub fn context(&self) -> &[OperationContext] {
        self.context.as_ref()
    }

    pub(crate) fn from_parts(
        reason: String,
        detail: Option<String>,
        position: Option<String>,
        context: Arc<Vec<OperationContext>>,
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
    /// assert_eq!(report.position(), Some("<hidden>"));
    /// ```
    pub fn redacted(&self, policy: &impl RedactPolicy) -> Self {
        Self {
            reason: redact_required_text(Some("reason"), &self.reason, policy),
            detail: redact_optional_text(Some("detail"), self.detail.as_deref(), policy),
            position: redact_optional_text(Some("position"), self.position.as_deref(), policy),
            context: Arc::new(
                self.context
                    .iter()
                    .cloned()
                    .map(|ctx| redact_context(ctx, policy))
                    .collect(),
            ),
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
