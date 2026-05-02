//! Source payload infrastructure.
//!
//! Types and functions for tracking, collecting, and inspecting error source
//! chains. This module defines the internal source payload representation
//! and the public observation types that expose it.

use std::sync::Arc;
use std::error::Error as StdError;

use smol_str::SmolStr;

use super::carrier::StructError;
use crate::core::DomainReason;

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

pub(crate) type BoxedSource = Arc<dyn StdError + Send + Sync + 'static>;

// ---------------------------------------------------------------------------
// SourcePayloadKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum SourcePayloadKind {
    Std,
    Struct,
}

// ---------------------------------------------------------------------------
// InternalSourceState – ephemeral builder state
// ---------------------------------------------------------------------------

pub(crate) struct InternalSourceState {
    pub(crate) source: BoxedSource,
    pub(crate) kind: SourcePayloadKind,
    pub(crate) frames: Vec<SourceFrame>,
}

// ---------------------------------------------------------------------------
// InternalSourcePayload – persisted variant, keeps frames in an Arc
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) enum InternalSourcePayload {
    Std {
        source: BoxedSource,
        frames: Arc<Vec<SourceFrame>>,
    },
    Struct {
        source: BoxedSource,
        frames: Arc<Vec<SourceFrame>>,
    },
}

impl InternalSourcePayload {
    pub(crate) fn new(
        source: BoxedSource,
        kind: SourcePayloadKind,
        frames: Vec<SourceFrame>,
    ) -> Self {
        let frames = Arc::new(frames);
        match kind {
            SourcePayloadKind::Std => Self::Std { source, frames },
            SourcePayloadKind::Struct => Self::Struct { source, frames },
        }
    }

    pub(crate) fn from_state(state: InternalSourceState) -> Self {
        Self::new(state.source, state.kind, state.frames)
    }

    pub(crate) fn source_ref(&self) -> &(dyn StdError + 'static) {
        match self {
            Self::Std { source, .. } | Self::Struct { source, .. } => source.as_ref(),
        }
    }

    pub(crate) fn source_arc(&self) -> BoxedSource {
        match self {
            Self::Std { source, .. } | Self::Struct { source, .. } => Arc::clone(source),
        }
    }

    pub(crate) fn kind(&self) -> SourcePayloadKind {
        match self {
            Self::Std { .. } => SourcePayloadKind::Std,
            Self::Struct { .. } => SourcePayloadKind::Struct,
        }
    }

    pub(crate) fn frames(&self) -> &[SourceFrame] {
        match self {
            Self::Std { frames, .. } | Self::Struct { frames, .. } => frames.as_ref(),
        }
    }

    pub(crate) fn root_cause(&self) -> &(dyn StdError + 'static) {
        let mut cur = self.source_ref();
        while let Some(next) = cur.source() {
            cur = next;
        }
        cur
    }

    pub(crate) fn source_chain(&self) -> Vec<String> {
        self.frames()
            .iter()
            .map(|frame| frame.message.to_string())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// SourcePayloadRef – public read-only borrow
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct SourcePayloadRef<'a> {
    pub(crate) payload: &'a InternalSourcePayload,
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

// ---------------------------------------------------------------------------
// SourceFrame
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SourceFrame {
    pub index: usize,
    pub message: SmolStr,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub display: Option<SmolStr>,
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    pub debug: Option<SmolStr>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub type_name: Option<SmolStr>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub error_code: Option<i32>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub reason: Option<SmolStr>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub path: Option<SmolStr>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub detail: Option<SmolStr>,
    /// Context key-value fields (from `with_field` / `record_field`).
    #[cfg_attr(feature = "serde", serde(skip))]
    pub context_fields: Vec<(SmolStr, SmolStr)>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "ErrorMetadata::is_empty")
    )]
    pub metadata: ErrorMetadata,
    pub is_root_cause: bool,
}

use super::super::metadata::ErrorMetadata;

// ---------------------------------------------------------------------------
// Source collection functions
// ---------------------------------------------------------------------------

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
            message: source.to_string().into(),
            display: None,
            debug: None,
            type_name: if index == 0 {
                root_type_name.map(SmolStr::from)
            } else {
                None
            },
            error_code: None,
            reason: None,
            path: None,
            detail: None,
            metadata: ErrorMetadata::default(),
            is_root_cause: false,
            context_fields: Vec::new(),
        });
        cur = source.source();
        index += 1;
    }

    if let Some(last) = frames.last_mut() {
        last.is_root_cause = true;
    }

    frames
}

pub(crate) fn collect_source_frames_from<E>(source: &E) -> Vec<SourceFrame>
where
    E: StdError + Send + Sync + 'static,
{
    collect_source_frames(source, Some(std::any::type_name::<E>()))
}

pub(crate) fn collect_struct_error_source_frames<R>(source: &StructError<R>) -> Vec<SourceFrame>
where
    R: DomainReason,
{
    let mut frames = Vec::with_capacity(source.source_frames().len() + 1);
    let ctx_fields: Vec<(SmolStr, SmolStr)> = source
        .contexts()
        .iter()
        .flat_map(|c| c.context().items.iter())
        .map(|(k, v)| (SmolStr::from(k.as_str()), SmolStr::from(v.as_str())))
        .collect();

    frames.push(SourceFrame {
        index: 0,
        message: source.reason().to_string().into(),
        display: Some(source.to_string().into()),
        debug: None,
        type_name: Some(std::any::type_name::<StructError<R>>().to_string().into()),
        error_code: None,
        reason: Some(source.reason().to_string().into()),
        path: source.target_path().map(Into::into),
        detail: source.detail().clone().map(Into::into),
        metadata: source.context_metadata(),
        context_fields: ctx_fields,
        is_root_cause: source.source_frames().is_empty(),
    });

    frames.extend(source.source_frames().iter().cloned().map(|mut frame| {
        frame.index += 1;
        frame
    }));

    frames
}

// ---------------------------------------------------------------------------
// InternalSourceState methods
// ---------------------------------------------------------------------------

impl InternalSourceState {
    pub(crate) fn from_std<E>(source: E) -> Self
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

    pub(crate) fn from_struct<R>(source: StructError<R>) -> Self
    where
        R: DomainReason,
    {
        use super::std_bridge::internal_into_std_bridge;

        let frames = collect_struct_error_source_frames(&source);
        Self {
            source: internal_into_std_bridge(source),
            kind: SourcePayloadKind::Struct,
            frames,
        }
    }

    #[cfg(feature = "anyhow")]
    pub(crate) fn from_dyn_struct(source: super::std_bridge::OwnedDynStdStructError) -> Self {
        let frames = source.source_frames().to_vec();
        Self {
            source: Arc::new(source),
            kind: SourcePayloadKind::Struct,
            frames,
        }
    }
}

fn is_struct_error_type_name(type_name: &str) -> bool {
    type_name.contains("StructError<")
}

fn assert_non_struct_source(type_name: &str, message: &str) {
    assert!(!is_struct_error_type_name(type_name), "{message}");
}
