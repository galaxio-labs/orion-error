/// Structured runtime error carrier with explicit bridge APIs for the standard
/// error ecosystem.
///
/// ```compile_fail
/// use orion_error::{StructError, UvsReason};
///
/// let err = StructError::from(UvsReason::system_error());
/// let _ = std::error::Error::source(&err);
/// ```
///
/// ```rust
/// use orion_error::{StructError, UvsReason};
///
/// let err = StructError::from(UvsReason::system_error());
/// let bridged = err.as_std();
/// let _ = std::error::Error::source(&bridged);
/// ```
#[derive(Debug, Clone)]
pub struct StructError<T: DomainReason> {
    imp: Box<StructErrorImpl<T>>,
}

impl<T: DomainReason> StructError<T> {
    pub fn imp(&self) -> &StructErrorImpl<T> {
        &self.imp
    }
}

impl<T: DomainReason> PartialEq for StructError<T> {
    fn eq(&self, other: &Self) -> bool {
        self.imp == other.imp
    }
}

impl<T: DomainReason> StructError<T> {
    pub fn reason(&self) -> &T {
        &self.imp.reason
    }

    pub fn detail(&self) -> &Option<String> {
        &self.imp.detail
    }

    pub fn position(&self) -> &Option<String> {
        &self.imp.position
    }

    pub fn new(
        reason: T,
        detail: Option<String>,
        position: Option<String>,
        context: Vec<OperationContext>,
    ) -> Self {
        Self::new_with_source(reason, detail, position, context, None)
    }

    fn new_with_source(
        reason: T,
        detail: Option<String>,
        position: Option<String>,
        context: Vec<OperationContext>,
        source_payload: Option<InternalSourcePayload>,
    ) -> Self {
        StructError {
            imp: Box::new(StructErrorImpl {
                reason,
                detail,
                position,
                context: Arc::new(context),
                source_payload,
            }),
        }
    }
}

impl<T> From<T> for StructError<T>
where
    T: DomainReason,
{
    fn from(value: T) -> Self {
        StructError::new(value, None, None, Vec::new())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct StructErrorImpl<T: DomainReason> {
    reason: T,
    detail: Option<String>,
    position: Option<String>,
    context: Arc<Vec<OperationContext>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    source_payload: Option<InternalSourcePayload>,
}

impl<T: DomainReason> PartialEq for StructErrorImpl<T> {
    fn eq(&self, other: &Self) -> bool {
        self.reason == other.reason
            && self.detail == other.detail
            && self.position == other.position
            && self.context == other.context
    }
}

impl<T: DomainReason> StructErrorImpl<T> {
    pub fn reason(&self) -> &T {
        &self.reason
    }

    pub fn detail(&self) -> &Option<String> {
        &self.detail
    }

    pub fn position(&self) -> &Option<String> {
        &self.position
    }

    pub fn context(&self) -> &Arc<Vec<OperationContext>> {
        &self.context
    }

    pub fn source_ref(&self) -> Option<&(dyn StdError + 'static)> {
        self.source_payload
            .as_ref()
            .map(InternalSourcePayload::source_ref)
    }

    fn source_payload_ref(&self) -> Option<SourcePayloadRef<'_>> {
        self.source_payload
            .as_ref()
            .map(|payload| SourcePayloadRef { payload })
    }

    pub fn source_frames(&self) -> &[SourceFrame] {
        self.source_payload
            .as_ref()
            .map(InternalSourcePayload::frames)
            .unwrap_or(&[])
    }
}

/// Convert a [`StructError`] from one reason type to another.
///
/// This preserves all detail, position, context, and source state while
/// mapping the reason via [`From`].
pub fn convert_error<R1, R2>(other: StructError<R1>) -> StructError<R2>
where
    R1: DomainReason,
    R2: DomainReason + From<R1>,
{
    StructError::new_with_source(
        other.imp.reason.into(),
        other.imp.detail,
        other.imp.position,
        Arc::try_unwrap(other.imp.context).unwrap_or_else(|arc| (*arc).clone()),
        other.imp.source_payload,
    )
}

impl<T: DomainReason> StructError<T> {
    fn with_internal_source(mut self, state: InternalSourceState) -> Self {
        self.imp.source_payload = Some(InternalSourcePayload::from_state(state));
        self
    }

    #[must_use]
    /// Route any supported source into the internal std/struct source split.
    ///
    /// Prefer [`with_source`](Self::with_source) for normal application code.
    /// This lower-level entry point stays internal so the public source story
    /// remains centered on `with_source(...)`.
    fn attach_source<S>(self, source: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.with_internal_source(source.into_source_payload())
    }

    #[must_use]
    /// Attach a non-structured source error explicitly.
    ///
    /// Prefer [`with_source`](Self::with_source) unless the call site needs to
    /// make the source channel explicit.
    pub fn with_std_source<E>(self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.with_internal_source(InternalSourceState::from_std(source))
    }

    #[must_use]
    /// Auto-route a source error through the internal std/struct source split.
    ///
    /// This is the recommended public entry point for attaching sources. It
    /// accepts both standard `StdError` values and `StructError<_>` values,
    /// automatically routing through the correct internal channel.
    ///
    /// Use [`with_std_source`](Self::with_std_source) or
    /// [`with_struct_source`](Self::with_struct_source) only when the call site
    /// intentionally wants to make that distinction explicit.
    #[allow(private_bounds)]
    pub fn with_source<S>(self, source: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.attach_source(source)
    }

    #[must_use]
    pub(crate) fn with_struct_error_source<R>(mut self, source: StructError<R>) -> Self
    where
        R: DomainReason,
    {
        self = self.with_internal_source(InternalSourceState::from_struct(source));
        self
    }

    #[must_use]
    /// Attach another `StructError<_>` explicitly as the structured source.
    ///
    /// Prefer [`with_source`](Self::with_source) unless the call site wants to
    /// make structured-source preservation explicit.
    pub fn with_struct_source<R>(self, source: StructError<R>) -> Self
    where
        R: DomainReason,
    {
        self.with_struct_error_source(source)
    }

    /// Internal helper: creates a new `StructError<R2>` wrapping `self` as the
    /// structured source.
    #[must_use]
    pub(crate) fn wrap<R2>(self, reason: R2) -> StructError<R2>
    where
        R2: DomainReason,
    {
        StructError::from(reason).with_struct_source(self)
    }

    pub fn source_ref(&self) -> Option<&(dyn StdError + 'static)> {
        self.imp.source_ref()
    }

    pub fn root_cause(&self) -> Option<&(dyn StdError + 'static)> {
        self.imp
            .source_payload
            .as_ref()
            .map(InternalSourcePayload::root_cause)
    }

    pub fn source_frames(&self) -> &[SourceFrame] {
        self.imp.source_frames()
    }

    /// Read-only source payload observation view.
    ///
    /// Prefer [`with_source`](Self::with_source), [`with_std_source`](Self::with_std_source),
    /// and [`with_struct_source`](Self::with_struct_source) when attaching
    /// sources. This accessor is intended for diagnostics, testing, and bridge
    /// inspection rather than normal application flow.
    pub fn source_payload(&self) -> Option<SourcePayloadRef<'_>> {
        self.imp.source_payload_ref()
    }

    /// Read-only source payload kind observation helper.
    ///
    /// This is a thin convenience wrapper over [`source_payload()`](Self::source_payload)
    /// for diagnostics and tests.
    pub fn source_payload_kind(&self) -> Option<SourcePayloadKind> {
        self.source_payload().map(|payload| payload.kind())
    }

    pub fn root_cause_frame(&self) -> Option<&SourceFrame> {
        self.source_frames().last()
    }

    /// Returns merged metadata from all context layers.
    ///
    /// Context layers are iterated in push order (innermost first). The merge
    /// uses an **inner wins** strategy: the first value set for any key is kept;
    /// outer layers only supply keys that are missing from inner layers.
    ///
    /// For example, if inner context sets `key = "inner"` and outer context sets
    /// `key = "outer"`, the result is `key = "inner"`.
    ///
    /// To query metadata from a specific context layer, use [`context_metadata_at`].
    pub fn context_metadata(&self) -> ErrorMetadata {
        let mut merged = ErrorMetadata::new();
        for ctx in self.contexts() {
            merged.merge_missing(ctx.metadata());
        }
        merged
    }

    /// Returns the metadata from a specific context layer by index.
    ///
    /// Index 0 is the innermost (first pushed) context layer.
    /// Returns `None` if the index is out of bounds.
    pub fn context_metadata_at(&self, index: usize) -> Option<&ErrorMetadata> {
        self.contexts().get(index).map(|ctx| ctx.metadata())
    }

    pub fn source_chain(&self) -> Vec<String> {
        self.imp
            .source_payload
            .as_ref()
            .map(InternalSourcePayload::source_chain)
            .unwrap_or_default()
    }

    pub fn into_std(self) -> OwnedStdStructError<T> {
        self.into()
    }

    pub fn into_boxed_std(self) -> Box<dyn StdError + Send + Sync + 'static> {
        self.into_std().into_boxed()
    }

    pub fn into_dyn_std(self) -> OwnedDynStdStructError
    where
        T: DomainReason,
    {
        self.into()
    }

    pub fn as_std(&self) -> StdStructRef<'_, T> {
        self.into()
    }

    pub fn display_chain(&self) -> String
    where
        T: std::fmt::Debug + Display + 'static,
    {
        let mut out = format!("{self}");
        let chain = self.source_chain();
        if !chain.is_empty() {
            out.push_str("\nCaused by:");
            for (idx, msg) in chain.iter().enumerate() {
                let mut lines = msg.lines();
                if let Some(first) = lines.next() {
                    out.push_str(&format!("\n  {idx}: {first}"));
                    for line in lines {
                        out.push_str(&format!("\n     {line}"));
                    }
                }
            }
        }
        out
    }

    #[cfg(feature = "anyhow")]
    pub(crate) fn with_dyn_struct_source(self, source: OwnedDynStdStructError) -> Self {
        self.with_internal_source(InternalSourceState::from_dyn_struct(source))
    }
}

impl<T: DomainReason> StructError<T> {
    pub fn builder(reason: T) -> StructErrorBuilder<T> {
        StructErrorBuilder {
            reason,
            detail: None,
            position: None,
            contexts: Vec::new(),
            source_payload: None,
        }
    }

    #[must_use]
    pub fn with_position(mut self, position: impl Into<String>) -> Self {
        self.imp.position = Some(position.into());
        self
    }
    #[must_use]
    pub fn with_context<C: Into<OperationContext>>(mut self, context: C) -> Self {
        Arc::make_mut(&mut self.imp.context).push(context.into());
        self
    }

    pub fn contexts(&self) -> &[OperationContext] {
        self.imp.context.as_ref()
    }

    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.imp.detail = Some(detail.into());
        self
    }
    pub fn err<V>(self) -> Result<V, Self> {
        Err(self)
    }

    pub fn action_main(&self) -> Option<String> {
        self.imp.context
            .iter()
            .rev()
            .find_map(|ctx| ctx.action().clone())
    }

    pub fn locator_main(&self) -> Option<String> {
        self.imp.context
            .iter()
            .rev()
            .find_map(|ctx| ctx.locator().clone())
    }

    pub fn path_segments(&self) -> Vec<String> {
        let mut path = Vec::new();
        let mut pending_locators: Vec<String> = Vec::new();

        for ctx in self.imp.context.iter().rev() {
            let locator_only = ctx.action().is_none()
                && ctx.compat_target().is_none()
                && ctx.locator().is_some()
                && ctx.path().len() <= 1;

            if locator_only {
                if let Some(locator) = ctx.locator().clone() {
                    pending_locators.push(locator);
                }
                continue;
            }

            let mut segments = ctx.normalized_path_segments();

            for locator in pending_locators.drain(..).rev() {
                if segments.last() != Some(&locator) {
                    segments.push(locator);
                }
            }

            for segment in segments {
                if path.last() != Some(&segment) {
                    path.push(segment);
                }
            }
        }

        for locator in pending_locators.into_iter().rev() {
            if path.last() != Some(&locator) {
                path.push(locator);
            }
        }

        path
    }

    pub fn target_path(&self) -> Option<String> {
        let segments = self.path_segments();
        if segments.is_empty() {
            None
        } else {
            Some(segments.join(" / "))
        }
    }
}

impl<T: DomainReason> ContextAdd<&OperationContext> for StructError<T> {
    fn add_context(&mut self, ctx: &OperationContext) {
        Arc::make_mut(&mut self.imp.context).push(ctx.clone());
    }
}
impl<T: DomainReason> ContextAdd<OperationContext> for StructError<T> {
    fn add_context(&mut self, ctx: OperationContext) {
        Arc::make_mut(&mut self.imp.context).push(ctx);
    }
}

impl<T: DomainReason> Display for StructError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{reason}", reason = self.reason())?;

        if let Some(pos) = &self.imp.position {
            write!(f, "\n  -> At: {pos}")?;
        }

        if let Some(path) = self.target_path() {
            write!(f, "\n  -> Path: {path}")?;
        }

        if let Some(detail) = &self.imp.detail {
            write!(f, "\n  -> Details: {detail}")?;
        }

        if let Some(source) = self.source_ref() {
            write!(f, "\n  -> Source: {source}")?;
        }

        if !self.imp.context.is_empty() {
            writeln!(f, "\n  -> Context stack:")?;

            for (i, c) in self.imp.context.iter().enumerate() {
                writeln!(f, "context {i}: ")?;
                writeln!(f, "{c}")?;
            }
        }

        Ok(())
    }
}

/// Builder for constructing a [`StructError`] with optional detail, position,
/// context, and source attachments.
///
/// Created via [`StructError::builder`]. Call [`finish`](StructErrorBuilder::finish)
/// to produce the final [`StructError`].
///
/// # Example
/// ```rust
/// use orion_error::{StructError, UvsReason};
///
/// let err = StructError::builder(UvsReason::validation_error())
///     .detail("port number out of range")
///     .position("src/config.rs:42")
///     .finish();
///
/// assert_eq!(err.detail().as_deref(), Some("port number out of range"));
/// ```
pub struct StructErrorBuilder<T: DomainReason> {
    reason: T,
    detail: Option<String>,
    position: Option<String>,
    contexts: Vec<OperationContext>,
    source_payload: Option<InternalSourcePayload>,
}

impl<T: DomainReason> StructErrorBuilder<T> {
    fn with_internal_source(mut self, state: InternalSourceState) -> Self {
        self.source_payload = Some(InternalSourcePayload::from_state(state));
        self
    }

    fn attach_source<S>(self, source: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.with_internal_source(source.into_source_payload())
    }

    /// Set the human-readable detail message for the error.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the source-code location (file, line, column).
    pub fn position(mut self, position: impl Into<String>) -> Self {
        self.position = Some(position.into());
        self
    }

    /// Attach an [`OperationContext`] to the builder.
    pub fn context(mut self, ctx: OperationContext) -> Self {
        self.contexts.push(ctx);
        self
    }

    /// Attach a borrowed [`OperationContext`] (cloned into the builder).
    pub fn context_ref(mut self, ctx: &OperationContext) -> Self {
        self.contexts.push(ctx.clone());
        self
    }

    /// Attach a non-structured source error explicitly.
    ///
    /// Prefer [`source`](Self::source) unless the builder call site needs to
    /// make the source channel explicit.
    pub fn source_std<E>(self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.with_internal_source(InternalSourceState::from_std(source))
    }

    /// Convenience sugar that auto-routes either a standard source error or an
    /// existing `StructError<_>` through the internal std/struct source split.
    ///
    /// This is the recommended builder entry point for attaching sources.
    /// Prefer `source_std(...)` / `source_struct(...)` only when the builder
    /// call site wants to make the source kind explicit.
    #[allow(private_bounds)]
    pub fn source<S>(self, source: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.attach_source(source)
    }

/// Attach a [`StructError`] as the structured source.
///
/// Prefer [`source`](Self::source) unless the builder call site needs to
/// make the structured-source channel explicit.
    pub fn source_struct<R>(mut self, source: StructError<R>) -> Self
    where
        R: DomainReason,
    {
        self = self.with_internal_source(InternalSourceState::from_struct(source));
        self
    }

    /// Consume the builder and produce a [`StructError`].
    pub fn finish(self) -> StructError<T> {
        StructError::new_with_source(
            self.reason,
            self.detail,
            self.position,
            self.contexts,
            self.source_payload,
        )
    }
}

impl<T: DomainReason> ErrorWith for StructError<T> {
    fn position<S: Into<String>>(mut self, pos: S) -> Self {
        self.imp.position = Some(pos.into());
        self
    }

    fn with_context<C: Into<OperationContext>>(mut self, ctx: C) -> Self {
        let ctx = ctx.into();
        self.add_context(ctx);
        self
    }
}
