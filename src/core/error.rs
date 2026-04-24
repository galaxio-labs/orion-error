use std::{error::Error as StdError, fmt::Display, ops::Deref, sync::Arc};

use super::{
    context::OperationContext, domain::DomainReason, metadata::ErrorMetadata, ContextAdd,
    ErrorCategory, ErrorCode, ErrorIdentityProvider,
};
use crate::traits::ErrorWith;
#[macro_export]
macro_rules! location {
    () => {
        format!("{}:{}:{}", file!(), line!(), column!())
    };
}

impl<T: DomainReason + ErrorCode> ErrorCode for StructError<T> {
    fn error_code(&self) -> i32 {
        self.reason.error_code()
    }
}

impl<T> ErrorIdentityProvider for StructError<T>
where
    T: DomainReason + ErrorIdentityProvider,
{
    fn stable_code(&self) -> &'static str {
        self.reason.stable_code()
    }

    fn error_category(&self) -> ErrorCategory {
        self.reason.error_category()
    }
}

type BoxedSource = Arc<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum SourcePayloadKind {
    Std,
    Struct,
}

#[derive(Debug)]
struct InternalSourceState {
    source: BoxedSource,
    kind: SourcePayloadKind,
    frames: Vec<SourceFrame>,
}

/// Owned source payload.
///
/// This is the public write-side counterpart of [`SourcePayloadRef`].  It keeps
/// ordinary standard errors and already-structured errors in separate payload
/// channels before attaching them to a [`StructError`].
#[derive(Debug)]
pub struct SourcePayload {
    state: InternalSourceState,
}

#[derive(Debug, Clone)]
enum InternalSourcePayload {
    Std {
        source: BoxedSource,
        frames: Arc<Vec<SourceFrame>>,
    },
    Struct {
        source: BoxedSource,
        frames: Arc<Vec<SourceFrame>>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct SourcePayloadRef<'a> {
    payload: &'a InternalSourcePayload,
}

pub trait IntoSourcePayload {
    fn into_source_payload(self) -> SourcePayload;
}

#[derive(Debug, Clone)]
pub struct OwnedStdStructError<R: DomainReason> {
    inner: StructError<R>,
}

#[derive(Debug, Clone)]
pub struct OwnedDynStdStructError {
    display: String,
    source: Option<BoxedSource>,
    frames: Arc<Vec<SourceFrame>>,
}

#[derive(Debug)]
pub struct StdStructRef<'a, R: DomainReason> {
    inner: &'a StructError<R>,
}

fn is_struct_error_type_name(type_name: &str) -> bool {
    type_name.contains("StructError<")
}

fn assert_non_struct_source(type_name: &str, message: &str) {
    assert!(!is_struct_error_type_name(type_name), "{message}");
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SourceFrame {
    pub index: usize,
    /// Stable human-facing summary. For `StructError` sources this is the reason text,
    /// not the full multi-line display output.
    pub message: String,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub display: Option<String>,
    /// Raw `Debug` output for local diagnostics. Do not send this to production logs
    /// without redaction; it may include internal fields or sensitive values.
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    pub debug: String,
    /// Best-effort type name. Rust does not expose concrete type names for every
    /// `dyn Error` frame, so this is not a complete classification key.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub type_name: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub error_code: Option<i32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub reason: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub want: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub path: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub detail: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "ErrorMetadata::is_empty")
    )]
    pub metadata: ErrorMetadata,
    pub is_root_cause: bool,
}

fn merged_context_metadata(contexts: &[OperationContext]) -> ErrorMetadata {
    let mut merged = ErrorMetadata::new();
    for ctx in contexts {
        merged.merge_missing(ctx.metadata());
    }
    merged
}

fn collect_source_frames(
    err: &(dyn StdError + 'static),
    root_type_name: Option<&'static str>,
) -> Vec<SourceFrame> {
    let mut frames = Vec::new();
    let mut cur = Some(err);
    let mut index = 0;

    while let Some(source) = cur {
        frames.push(SourceFrame {
            index,
            message: source.to_string(),
            display: None,
            debug: format!("{source:?}"),
            type_name: if index == 0 {
                root_type_name.map(str::to_string)
            } else {
                None
            },
            error_code: None,
            reason: None,
            want: None,
            path: None,
            detail: None,
            metadata: ErrorMetadata::default(),
            is_root_cause: false,
        });
        cur = source.source();
        index += 1;
    }

    if let Some(last) = frames.last_mut() {
        last.is_root_cause = true;
    }

    frames
}

fn collect_source_frames_from<E>(source: &E) -> Vec<SourceFrame>
where
    E: StdError + Send + Sync + 'static,
{
    collect_source_frames(source, Some(std::any::type_name::<E>()))
}

fn collect_struct_error_source_frames<R>(source: &StructError<R>) -> Vec<SourceFrame>
where
    R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
{
    let mut frames = Vec::with_capacity(source.source_frames().len() + 1);
    frames.push(SourceFrame {
        index: 0,
        message: source.reason().to_string(),
        display: Some(source.to_string()),
        debug: format!("{source:?}"),
        type_name: Some(std::any::type_name::<StructError<R>>().to_string()),
        error_code: Some(source.error_code()),
        reason: Some(source.reason().to_string()),
        want: source.target_main(),
        path: source.target_path(),
        detail: source.detail().clone(),
        metadata: source.context_metadata(),
        is_root_cause: source.source_frames().is_empty(),
    });

    frames.extend(source.source_frames().iter().cloned().map(|mut frame| {
        frame.index += 1;
        frame
    }));

    frames
}

impl InternalSourceState {
    fn from_std<E>(source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        assert_non_struct_source(
            std::any::type_name::<E>(),
            "use with_struct_source(...) when attaching StructError sources",
        );
        let frames = collect_source_frames_from(&source);
        Self {
            source: Arc::new(source),
            kind: SourcePayloadKind::Std,
            frames,
        }
    }

    fn from_struct<R>(source: StructError<R>) -> Self
    where
        R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        let frames = collect_struct_error_source_frames(&source);
        Self {
            source: internal_into_std_bridge(source),
            kind: SourcePayloadKind::Struct,
            frames,
        }
    }

    #[cfg(feature = "anyhow")]
    fn from_dyn_struct(source: OwnedDynStdStructError) -> Self {
        let frames = source.source_frames().to_vec();
        Self {
            source: Arc::new(source),
            kind: SourcePayloadKind::Struct,
            frames,
        }
    }
}

impl SourcePayload {
    fn from_state(state: InternalSourceState) -> Self {
        Self { state }
    }

    fn into_internal_state(self) -> InternalSourceState {
        self.state
    }

    pub fn kind(&self) -> SourcePayloadKind {
        self.state.kind
    }

    pub fn source(&self) -> &(dyn StdError + 'static) {
        self.state.source.as_ref()
    }

    pub fn frames(&self) -> &[SourceFrame] {
        &self.state.frames
    }
}

impl<E> IntoSourcePayload for E
where
    E: StdError + Send + Sync + 'static,
{
    fn into_source_payload(self) -> SourcePayload {
        SourcePayload::from_state(InternalSourceState::from_std(self))
    }
}

impl<R> IntoSourcePayload for StructError<R>
where
    R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
{
    fn into_source_payload(self) -> SourcePayload {
        SourcePayload::from_state(InternalSourceState::from_struct(self))
    }
}

fn internal_into_std_bridge<R>(source: StructError<R>) -> BoxedSource
where
    R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
{
    Arc::new(OwnedStdStructError { inner: source })
}

impl<R> OwnedStdStructError<R>
where
    R: DomainReason,
{
    pub fn into_struct(self) -> StructError<R> {
        self.inner
    }

    pub fn inner(&self) -> &StructError<R> {
        &self.inner
    }

    pub fn into_boxed(self) -> Box<dyn StdError + Send + Sync + 'static>
    where
        R: ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        Box::new(self)
    }
}

impl<R> From<StructError<R>> for OwnedStdStructError<R>
where
    R: DomainReason,
{
    fn from(value: StructError<R>) -> Self {
        Self { inner: value }
    }
}

impl OwnedDynStdStructError {
    pub fn source_frames(&self) -> &[SourceFrame] {
        self.frames.as_ref()
    }

    pub fn into_boxed(self) -> Box<dyn StdError + Send + Sync + 'static> {
        Box::new(self)
    }
}

impl<R> From<StructError<R>> for OwnedDynStdStructError
where
    R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
{
    fn from(value: StructError<R>) -> Self {
        let display = value.to_string();
        let source = value
            .imp
            .source_payload
            .as_ref()
            .map(InternalSourcePayload::source_arc);
        let frames = Arc::new(collect_struct_error_source_frames(&value));
        Self {
            display,
            source,
            frames,
        }
    }
}

impl<R> From<OwnedStdStructError<R>> for OwnedDynStdStructError
where
    R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
{
    fn from(value: OwnedStdStructError<R>) -> Self {
        value.into_struct().into()
    }
}

impl Display for OwnedDynStdStructError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display)
    }
}

impl StdError for OwnedDynStdStructError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| source.as_ref() as &(dyn StdError + 'static))
    }
}

impl<'a, R> StdStructRef<'a, R>
where
    R: DomainReason,
{
    pub fn inner(&self) -> &'a StructError<R> {
        self.inner
    }
}

impl<'a, R> From<&'a StructError<R>> for StdStructRef<'a, R>
where
    R: DomainReason,
{
    fn from(value: &'a StructError<R>) -> Self {
        Self { inner: value }
    }
}

impl<R> Display for OwnedStdStructError<R>
where
    R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl<R> StdError for OwnedStdStructError<R>
where
    R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
{
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.source_ref()
    }
}

impl<'a, R> Display for StdStructRef<'a, R>
where
    R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.inner, f)
    }
}

impl<'a, R> StdError for StdStructRef<'a, R>
where
    R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
{
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.source_ref()
    }
}

impl InternalSourcePayload {
    fn new(source: BoxedSource, kind: SourcePayloadKind, frames: Vec<SourceFrame>) -> Self {
        let frames = Arc::new(frames);
        match kind {
            SourcePayloadKind::Std => Self::Std { source, frames },
            SourcePayloadKind::Struct => Self::Struct { source, frames },
        }
    }

    fn from_state(state: InternalSourceState) -> Self {
        Self::new(state.source, state.kind, state.frames)
    }

    fn source_ref(&self) -> &(dyn StdError + 'static) {
        match self {
            Self::Std { source, .. } | Self::Struct { source, .. } => source.as_ref(),
        }
    }

    fn source_arc(&self) -> BoxedSource {
        match self {
            Self::Std { source, .. } | Self::Struct { source, .. } => Arc::clone(source),
        }
    }

    fn kind(&self) -> SourcePayloadKind {
        match self {
            Self::Std { .. } => SourcePayloadKind::Std,
            Self::Struct { .. } => SourcePayloadKind::Struct,
        }
    }

    fn frames(&self) -> &[SourceFrame] {
        match self {
            Self::Std { frames, .. } | Self::Struct { frames, .. } => frames.as_ref(),
        }
    }

    fn root_cause(&self) -> &(dyn StdError + 'static) {
        let mut cur = self.source_ref();
        while let Some(next) = cur.source() {
            cur = next;
        }
        cur
    }

    fn source_chain(&self) -> Vec<String> {
        self.frames()
            .iter()
            .map(|frame| frame.message.clone())
            .collect()
    }
}

impl<'a> SourcePayloadRef<'a> {
    pub fn kind(&self) -> SourcePayloadKind {
        self.payload.kind()
    }

    pub fn source(&self) -> &'a (dyn StdError + 'static) {
        self.payload.source_ref()
    }

    pub fn frames(&self) -> &'a [SourceFrame] {
        self.payload.frames()
    }

    pub fn root_cause(&self) -> &'a (dyn StdError + 'static) {
        self.payload.root_cause()
    }

    pub fn source_chain(&self) -> Vec<String> {
        self.payload.source_chain()
    }
}

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

#[cfg(feature = "serde")]
impl<T: DomainReason> serde::Serialize for StructError<T>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let source_frames = self.source_frames();
        let source_chain = self.source_chain();
        let source_message = source_chain.first().cloned();
        let want = self.target_main();
        let path = self.target_path();

        let mut state = serializer.serialize_struct("StructError", 9)?;
        state.serialize_field("reason", &self.imp.reason)?;
        state.serialize_field("detail", &self.imp.detail)?;
        state.serialize_field("position", &self.imp.position)?;
        state.serialize_field("context", self.imp.context.as_ref())?;
        state.serialize_field("want", &want)?;
        state.serialize_field("path", &path)?;
        state.serialize_field("source_frames", &source_frames)?;
        state.serialize_field("source_message", &source_message)?;
        state.serialize_field("source_chain", &source_chain)?;
        state.end()
    }
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

impl<T: DomainReason> Deref for StructError<T> {
    type Target = StructErrorImpl<T>;

    fn deref(&self) -> &Self::Target {
        &self.imp
    }
}
impl<T: DomainReason> StructError<T> {
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
    /// Attach any source that can be converted into the dual-channel source
    /// payload model.
    ///
    /// Ordinary `StdError` values attach as `SourcePayloadKind::Std`, while
    /// `StructError<_>` attaches as `SourcePayloadKind::Struct`.
    pub fn attach_source<S>(self, source: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.with_internal_source(source.into_source_payload().into_internal_state())
    }

    #[must_use]
    /// Attach a non-structured source error.
    ///
    /// For `StructError<_>` sources, use `with_struct_source(...)` so metadata and
    /// structured source frames are preserved.
    pub fn with_std_source<E>(self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.with_internal_source(InternalSourceState::from_std(source))
    }

    #[must_use]
    /// Recommended helper that auto-routes either a standard source error or an
    /// existing `StructError<_>` through the dual-channel source model.
    ///
    /// Use `with_std_source(...)` / `with_struct_source(...)` instead when the
    /// call site should make the source kind explicit.
    pub fn with_source<S>(self, source: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.attach_source(source)
    }

    #[must_use]
    pub(crate) fn with_struct_error_source<R>(mut self, source: StructError<R>) -> Self
    where
        R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        self = self.with_internal_source(InternalSourceState::from_struct(source));
        self
    }

    #[must_use]
    pub fn with_struct_source<R>(self, source: StructError<R>) -> Self
    where
        R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        self.with_struct_error_source(source)
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

    pub fn source_payload(&self) -> Option<SourcePayloadRef<'_>> {
        self.imp.source_payload_ref()
    }

    pub fn source_payload_kind(&self) -> Option<SourcePayloadKind> {
        self.source_payload().map(|payload| payload.kind())
    }

    pub fn root_cause_frame(&self) -> Option<&SourceFrame> {
        self.source_frames().last()
    }

    pub fn context_metadata(&self) -> ErrorMetadata {
        merged_context_metadata(self.contexts())
    }

    pub fn source_chain(&self) -> Vec<String> {
        self.imp
            .source_payload
            .as_ref()
            .map(InternalSourcePayload::source_chain)
            .unwrap_or_default()
    }

    pub fn into_std(self) -> OwnedStdStructError<T>
    where
        T: ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        self.into()
    }

    pub fn into_boxed_std(self) -> Box<dyn StdError + Send + Sync + 'static>
    where
        T: ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        self.into_std().into_boxed()
    }

    pub fn into_dyn_std(self) -> OwnedDynStdStructError
    where
        T: ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        self.into()
    }

    pub fn as_std(&self) -> StdStructRef<'_, T>
    where
        T: ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        self.into()
    }

    pub fn display_chain(&self) -> String
    where
        T: ErrorCode + std::fmt::Debug + Display + 'static,
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

    pub fn render(&self, mode: super::report::RenderMode) -> String {
        self.report().render(mode)
    }

    pub fn render_redacted(
        &self,
        mode: super::report::RenderMode,
        policy: &impl super::report::RedactPolicy,
    ) -> String {
        self.report().render_redacted(mode, policy)
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

    /// 使用示例
    ///self.with_position(location!());
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

    // 提供修改方法
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.imp.detail = Some(detail.into());
        self
    }
    pub fn err<V>(self) -> Result<V, Self> {
        Err(self)
    }
    pub fn target_main(&self) -> Option<String> {
        self.context
            .iter()
            .rev()
            .find_map(|ctx| ctx.target().clone())
    }

    pub fn action_main(&self) -> Option<String> {
        self.context
            .iter()
            .rev()
            .find_map(|ctx| ctx.action().clone())
    }

    pub fn locator_main(&self) -> Option<String> {
        self.context
            .iter()
            .rev()
            .find_map(|ctx| ctx.locator().clone())
    }

    /// Compatibility alias for `target_main()`.
    ///
    /// Prefer `target_main()` in new code when pairing it with `target_path()`.
    pub fn target(&self) -> Option<String> {
        self.target_main()
    }

    pub fn path_segments(&self) -> Vec<String> {
        let mut path = Vec::new();
        let mut pending_locators: Vec<String> = Vec::new();

        for ctx in self.context.iter().rev() {
            let locator_only = ctx.action().is_none()
                && ctx.target().is_none()
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

/*
impl<S1: Into<String>, S2: Into<String>, T: DomainReason> ContextAdd<(S1, S2)> for StructError<T> {
    fn add_context(&mut self, val: (S1, S2)) {
        self.imp.context.items.push((val.0.into(), val.1.into()));
    }
}
*/

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

impl<T: std::fmt::Display + DomainReason + ErrorCode> Display for StructError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 核心错误信息
        write!(f, "[{}] {reason}", self.error_code(), reason = self.reason)?;

        // 位置信息优先显示
        if let Some(pos) = &self.position {
            write!(f, "\n  -> At: {pos}")?;
        }

        // 目标资源信息
        let want = self.target_main();
        if let Some(want) = &want {
            write!(f, "\n  -> Want: {want}")?;
        }

        if let Some(path) = self.target_path() {
            if want.as_deref() != Some(path.as_str()) {
                write!(f, "\n  -> Path: {path}")?;
            }
        }

        // 技术细节
        if let Some(detail) = &self.detail {
            write!(f, "\n  -> Details: {detail}")?;
        }

        if let Some(source) = self.source_ref() {
            write!(f, "\n  -> Source: {source}")?;
        }

        // 上下文信息
        if !self.context.is_empty() {
            writeln!(f, "\n  -> Context stack:")?;

            for (i, c) in self.context.iter().enumerate() {
                writeln!(f, "context {i}: ")?;
                writeln!(f, "{c}")?;
            }
        }

        Ok(())
    }
}

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

    pub fn attach_source<S>(self, source: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.with_internal_source(source.into_source_payload().into_internal_state())
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn position(mut self, position: impl Into<String>) -> Self {
        self.position = Some(position.into());
        self
    }

    pub fn context(mut self, ctx: OperationContext) -> Self {
        self.contexts.push(ctx);
        self
    }

    pub fn context_ref(mut self, ctx: &OperationContext) -> Self {
        self.contexts.push(ctx.clone());
        self
    }

    /// Attach a non-structured source error.
    ///
    /// For `StructError<_>` sources, use `source_struct(...)` so metadata and
    /// structured source frames are preserved.
    pub fn source_std<E>(self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.with_internal_source(InternalSourceState::from_std(source))
    }

    /// Convenience sugar that auto-routes either a standard source error or an
    /// existing `StructError<_>` through the dual-channel source model.
    ///
    /// Prefer `source_std(...)` / `source_struct(...)` when you want the call
    /// site to make the source kind explicit.
    pub fn source<S>(self, source: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.attach_source(source)
    }

    pub fn source_struct<R>(mut self, source: StructError<R>) -> Self
    where
        R: DomainReason + ErrorCode + std::fmt::Debug + Display + Send + Sync + 'static,
    {
        self = self.with_internal_source(InternalSourceState::from_struct(source));
        self
    }

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
    fn want<S: Into<String>>(mut self, desc: S) -> Self {
        let desc = desc.into();
        let ctx_stack = Arc::make_mut(&mut self.imp.context);
        if ctx_stack.is_empty() {
            ctx_stack.push(OperationContext::from_target(desc));
        } else if let Some(x) = ctx_stack.last_mut() {
            x.push_target_segment(desc);
        }
        self
    }
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

#[cfg(all(test, feature = "serde"))]
mod tests {
    use std::{error::Error as StdError, fmt};

    use crate::{
        core::context::{CallContext, ContextRecord},
        DomainReason, UvsReason,
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
        assert_eq!(json_value.get("want"), Some(&serde_json::Value::Null));
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

        let display_output = format!("{error}");
        assert!(display_output.contains("-> Source: outer source"));
    }

    #[test]
    fn test_struct_error_uses_outer_want_and_full_path() {
        let mut outer = OperationContext::doing("place_order");
        outer.with_doing("read_order_payload");
        outer.with_doing("parse_order");
        outer.record("order_id", "42");

        let error = StructError::from(TestDomainReason::TestError).with_context(outer);

        assert_eq!(error.action_main().as_deref(), Some("place_order"));
        assert_eq!(error.target_main().as_deref(), Some("place_order"));
        assert_eq!(error.target().as_deref(), Some("place_order"));
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
        assert!(display_output.contains("-> Want: place_order"));
        assert!(display_output.contains("-> Path: place_order / read_order_payload / parse_order"));
    }

    #[test]
    fn test_errorwith_doing_and_at_write_structured_context_semantics() {
        let error = StructError::from(TestDomainReason::TestError)
            .doing("parse config")
            .at("config.toml");

        assert_eq!(error.action_main().as_deref(), Some("parse config"));
        assert_eq!(error.locator_main().as_deref(), Some("config.toml"));
        assert_eq!(error.target_main().as_deref(), Some("parse config"));
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
        assert!(display_chain.contains("Caused by:"));
        assert!(display_chain.contains("0: outer source"));
        assert!(display_chain.contains("1: inner source"));
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
    fn test_struct_error_serialization_includes_want_and_path() {
        let mut outer = OperationContext::doing("place_order");
        outer.with_doing("read_order_payload");
        outer.record("order_id", "42");

        let error = StructError::from(TestDomainReason::TestError).with_context(outer);

        let json_value = serde_json::to_value(&error).unwrap();
        assert_eq!(
            json_value.get("want"),
            Some(&serde_json::Value::String("place_order".to_string()))
        );
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
            "[1001] test error\n  -> Details: repo layer failed\n  -> Source: db unavailable"
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
    fn test_attach_source_routes_std_source_payload() {
        let error = StructError::from(TestDomainReason::TestError)
            .attach_source(std::io::Error::other("disk offline"));

        assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Std));
        assert_eq!(error.source_ref().unwrap().to_string(), "disk offline");
        assert_eq!(error.source_frames()[0].message, "disk offline");
    }

    #[test]
    fn test_attach_source_routes_struct_source_payload() {
        let source = StructError::from(TestDomainReason::TestError)
            .with_detail("repo layer failed")
            .with_std_source(std::io::Error::other("db unavailable"));
        let error = StructError::from(TestDomainReason::Uvs(UvsReason::system_error()))
            .attach_source(source);

        assert_eq!(error.source_payload_kind(), Some(SourcePayloadKind::Struct));
        assert_eq!(
            error.source_frames()[0].reason.as_deref(),
            Some("test error")
        );
        assert_eq!(error.root_cause().unwrap().to_string(), "db unavailable");
    }

    #[test]
    fn test_builder_attach_source_routes_std_source_payload() {
        let error = StructError::builder(TestDomainReason::TestError)
            .attach_source(std::io::Error::other("disk offline"))
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
            "[1001] test error\n  -> Details: repo layer failed\n  -> Source: db unavailable"
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
            .with_context(
                OperationContext::doing("start engine").with_meta("component.name", "engine"),
            )
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

    #[cfg(feature = "serde")]
    #[test]
    fn test_source_frame_serialization_skips_empty_metadata() {
        let frame = SourceFrame {
            index: 0,
            message: "message".to_string(),
            display: None,
            debug: "debug".to_string(),
            type_name: None,
            error_code: None,
            reason: None,
            want: None,
            path: None,
            detail: None,
            metadata: ErrorMetadata::default(),
            is_root_cause: true,
        };

        let json_value = serde_json::to_value(&frame).unwrap();
        assert!(!json_value
            .as_object()
            .expect("object")
            .contains_key("metadata"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_source_frame_serialization_includes_metadata() {
        let error = StructError::from(TestDomainReason::TestError).with_context(
            OperationContext::doing("load sink defaults")
                .with_meta("config.kind", "sink_defaults")
                .with_meta("parse.line", 1u32),
        );
        let wrapped = StructError::from(TestDomainReason::Uvs(UvsReason::system_error()))
            .with_struct_source(error);

        let json_value = serde_json::to_value(&wrapped).unwrap();
        assert_eq!(
            json_value["source_frames"][0]["metadata"]["config.kind"],
            serde_json::Value::String("sink_defaults".to_string())
        );
        assert_eq!(
            json_value["source_frames"][0]["metadata"]["parse.line"],
            serde_json::json!(1)
        );
    }

    #[allow(deprecated)]
    #[test]
    fn test_attach_context_deprecated_alias_matches_with_context() {
        let ctx = OperationContext::doing("load config").with_meta("tenant", "acme");
        let via_new = StructError::from(TestDomainReason::TestError).with_context(ctx.clone());
        let via_old = StructError::from(TestDomainReason::TestError).attach_context(ctx);

        assert_eq!(via_old.contexts(), via_new.contexts());
        assert_eq!(via_old.context_metadata(), via_new.context_metadata());
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
}
