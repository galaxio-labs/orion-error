impl<T> ErrorIdentityProvider for StructError<T>
where
    T: DomainReason + ErrorIdentityProvider,
{
    fn stable_code(&self) -> &'static str {
        self.reason().stable_code()
    }

    fn error_category(&self) -> ErrorCategory {
        self.reason().error_category()
    }
}

type BoxedSource = Arc<dyn StdError + Send + Sync + 'static>;

/// Discriminator for the source payload channel.
///
/// - [`Std`](SourcePayloadKind::Std): the source was routed through the
///   standard `std::error::Error` channel.
/// - [`Struct`](SourcePayloadKind::Struct): the source was routed through
///   the structured `StructError` channel.
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

/// Read-only borrow of a source payload for diagnostics and testing.
///
/// Obtained via [`StructError::source_payload`].
#[derive(Debug, Clone, Copy)]
pub struct SourcePayloadRef<'a> {
    payload: &'a InternalSourcePayload,
}

trait IntoSourcePayload {
    fn into_source_payload(self) -> InternalSourceState;
}

/// Owned bridge wrapper that presents a [`StructError`] as a `dyn StdError`.
///
/// Created via [`StructError::into_std`]. Retains the full structured error
/// payload and can be recovered via
/// [`into_struct`](OwnedStdStructError::into_struct).
#[derive(Debug, Clone)]
pub struct OwnedStdStructError<R: DomainReason> {
    inner: StructError<R>,
}

/// Type-erased owned bridge wrapper for [`StructError`] values.
///
/// Created via [`StructError::into_dyn_std`]. Erases the `R: DomainReason`
/// type parameter so the error can cross generic boundaries while still
/// preserving structured source-frame metadata.
#[derive(Debug, Clone)]
pub struct OwnedDynStdStructError {
    display: String,
    source: Option<BoxedSource>,
    frames: Arc<Vec<SourceFrame>>,
}

/// Borrowed bridge wrapper that presents a [`StructError`] as a `dyn StdError`.
///
/// Created via [`StructError::as_std`]. Does not consume the underlying error.
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SourceFrame {
    /// Position in the source chain (0 = outermost frame).
    pub index: usize,
    /// Stable human-facing summary. For `StructError` sources this is the reason text,
    /// not the full multi-line display output.
    pub message: String,
    /// Full multi-line display output (available for `StructError` sources).
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
    /// Numeric error code for this source frame (when available).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub error_code: Option<i32>,
    /// Human-readable reason for this source frame (reason text from the
    /// `StructError` reason, when available).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub reason: Option<String>,
    /// Resource path for this source frame (when available).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub path: Option<String>,
    /// Human-readable detail message for this source frame (when available).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub detail: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "ErrorMetadata::is_empty")
    )]
    /// Key-value metadata attached to this source frame.
    pub metadata: ErrorMetadata,
    /// Whether this frame is the root cause of the error chain.
    pub is_root_cause: bool,
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
    R: DomainReason,
{
    let mut frames = Vec::with_capacity(source.source_frames().len() + 1);
    frames.push(SourceFrame {
        index: 0,
        message: source.reason().to_string(),
        display: Some(source.to_string()),
        debug: format!("{source:?}"),
        type_name: Some(std::any::type_name::<StructError<R>>().to_string()),
        error_code: None,
        reason: Some(source.reason().to_string()),
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
        R: DomainReason,
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

impl<E> IntoSourcePayload for E
where
    E: StdError + Send + Sync + 'static,
{
    fn into_source_payload(self) -> InternalSourceState {
        InternalSourceState::from_std(self)
    }
}

impl<R> IntoSourcePayload for StructError<R>
where
    R: DomainReason,
{
    fn into_source_payload(self) -> InternalSourceState {
        InternalSourceState::from_struct(self)
    }
}

fn internal_into_std_bridge<R>(source: StructError<R>) -> BoxedSource
where
    R: DomainReason,
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

    pub fn into_boxed(self) -> Box<dyn StdError + Send + Sync + 'static> {
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
    R: DomainReason,
{
    fn from(value: StructError<R>) -> Self {
        let display = value.to_string();
        let source = value
            .imp
            .source_payload
            .as_ref()
            .map(InternalSourcePayload::source_arc);
        let frames = collect_struct_error_source_frames(&value);
        Self {
            display,
            source,
            frames: Arc::new(frames),
        }
    }
}

impl<R> From<OwnedStdStructError<R>> for OwnedDynStdStructError
where
    R: DomainReason,
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
    R: DomainReason,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl<R> StdError for OwnedStdStructError<R>
where
    R: DomainReason,
{
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.source_ref()
    }
}

impl<'a, R> Display for StdStructRef<'a, R>
where
    R: DomainReason,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.inner, f)
    }
}

impl<'a, R> StdError for StdStructRef<'a, R>
where
    R: DomainReason,
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

