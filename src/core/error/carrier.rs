//! Core error carrier types.
//!
//! `StructError<T>` is the primary runtime error carrier. Closely associated
//! type `StructErrorImpl` lives here alongside the `Display`, `PartialEq`,
//! `ContextAdd`, and `ErrorWith` implementations.
//!
//! [`StructErrorBuilder`](super::builder::StructErrorBuilder) lives in the
//! sibling module `builder`.

use std::sync::Arc;
use std::error::Error as StdError;
use std::fmt::Display;

use super::source_chain::{
    InternalSourcePayload, InternalSourceState, SourcePayloadKind, SourcePayloadRef,
};
use super::std_bridge::{IntoSourcePayload, OwnedDynStdStructError, OwnedStdStructError,
    StdStructRef,
};
use super::builder::StructErrorBuilder;
use super::super::{
    context::OperationContext, domain::DomainReason, metadata::ErrorMetadata, ContextAdd,
};
use crate::traits::ErrorWith;

// ---------------------------------------------------------------------------
// StructError
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct StructError<T: DomainReason> {
    pub(crate) imp: Box<StructErrorImpl<T>>,
}

impl<T: DomainReason> StructError<T> {
    pub fn imp(&self) -> &StructErrorImpl<T> {
        &self.imp
    }

    pub(crate) fn into_imp(self) -> Box<StructErrorImpl<T>> {
        self.imp
    }

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

    pub(crate) fn new_with_source(
        reason: T,
        detail: Option<String>,
        position: Option<String>,
        context: Vec<OperationContext>,
        source_payload: Option<InternalSourcePayload>,
    ) -> Self {
        let context = if context.is_empty() {
            None
        } else {
            Some(Arc::new(context))
        };
        StructError {
            imp: Box::new(StructErrorImpl {
                reason,
                detail,
                position,
                context,
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

impl<T: DomainReason> PartialEq for StructError<T> {
    fn eq(&self, other: &Self) -> bool {
        self.imp == other.imp
    }
}

// ---------------------------------------------------------------------------
// StructErrorImpl
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct StructErrorImpl<T: DomainReason> {
    pub reason: T,
    pub detail: Option<String>,
    pub position: Option<String>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub context: Option<Arc<Vec<OperationContext>>>,
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    pub(crate) source_payload: Option<InternalSourcePayload>,
}

impl<T: DomainReason> PartialEq for StructErrorImpl<T> {
    fn eq(&self, other: &Self) -> bool {
        self.reason == other.reason
            && self.detail == other.detail
            && self.position == other.position
            && self.context.as_deref().map_or(&[][..], |v| v.as_slice()) == other.context.as_deref().map_or(&[][..], |v| v.as_slice())
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

    /// Borrow the context list (empty slice when no context was attached).
    pub fn context(&self) -> &[OperationContext] {
        self.context.as_deref().map_or(&[], |v| v.as_ref())
    }

    /// Get a clone of the context Arc (allocates an empty Vec when absent).
    /// Prefer [`context()`](Self::context) for read-only access.
    pub(crate) fn context_arc(&self) -> Arc<Vec<OperationContext>> {
        self.context.clone().unwrap_or_default()
    }

    pub fn source_ref(&self) -> Option<&(dyn StdError + 'static)> {
        self.source_payload
            .as_ref()
            .map(|sp| sp.source_ref())
    }

    fn source_payload_ref(&self) -> Option<SourcePayloadRef<'_>> {
        self.source_payload
            .as_ref()
            .map(|payload| SourcePayloadRef { payload })
    }

    pub fn source_frames(&self) -> &[crate::core::error::source_chain::SourceFrame] {
        self.source_payload
            .as_ref()
            .map(|sp| sp.frames())
            .unwrap_or(&[])
    }
}

// ---------------------------------------------------------------------------
// convert_error
// ---------------------------------------------------------------------------

pub fn convert_error<R1, R2>(other: StructError<R1>) -> StructError<R2>
where
    R1: DomainReason,
    R2: DomainReason + From<R1>,
{
    StructError::new_with_source(
        other.imp.reason.into(),
        other.imp.detail,
        other.imp.position,
        other.imp.context.map_or(Vec::new(), |arc| Arc::try_unwrap(arc).unwrap_or_else(|a| (*a).clone())),
        other.imp.source_payload,
    )
}

// ---------------------------------------------------------------------------
// StructError – source-related methods
// ---------------------------------------------------------------------------

impl<T: DomainReason> StructError<T> {
    fn with_internal_source(mut self, state: InternalSourceState) -> Self {
        self.imp.source_payload = Some(InternalSourcePayload::from_state(state));
        self
    }

    fn attach_source<S>(self, source: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.with_internal_source(source.into_source_payload())
    }

    pub fn with_std_source<E>(self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.with_internal_source(InternalSourceState::from_std(source))
    }

    #[allow(private_bounds)]
    pub fn with_source<S>(self, source: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.attach_source(source)
    }

    pub(crate) fn with_struct_error_source<R>(mut self, source: StructError<R>) -> Self
    where
        R: DomainReason,
    {
        self = self.with_internal_source(InternalSourceState::from_struct(source));
        self
    }

    pub fn with_struct_source<R>(self, source: StructError<R>) -> Self
    where
        R: DomainReason,
    {
        self.with_struct_error_source(source)
    }

    #[allow(dead_code)]
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
            .map(|sp| sp.root_cause())
    }

    pub fn source_frames(&self) -> &[super::source_chain::SourceFrame] {
        self.imp.source_frames()
    }

    pub fn source_payload(&self) -> Option<SourcePayloadRef<'_>> {
        self.imp.source_payload_ref()
    }

    pub fn source_payload_kind(&self) -> Option<SourcePayloadKind> {
        self.source_payload().map(|payload| payload.kind())
    }

    pub fn root_cause_frame(&self) -> Option<&super::source_chain::SourceFrame> {
        self.source_frames().last()
    }

    pub fn context_metadata(&self) -> ErrorMetadata {
        let mut merged = ErrorMetadata::new();
        for ctx in self.contexts() {
            merged.merge_missing(ctx.metadata());
        }
        merged
    }

    pub fn context_metadata_at(&self, index: usize) -> Option<&ErrorMetadata> {
        self.contexts().get(index).map(|ctx| ctx.metadata())
    }

    pub fn source_chain(&self) -> Vec<String> {
        self.imp
            .source_payload
            .as_ref()
            .map(|sp| sp.source_chain())
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

// ---------------------------------------------------------------------------
// StructError – builder, context, path methods
// ---------------------------------------------------------------------------

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

    pub fn with_position(mut self, position: impl Into<String>) -> Self {
        self.imp.position = Some(position.into());
        self
    }

    pub fn with_context<C: Into<OperationContext>>(mut self, context: C) -> Self {
        let vec = self.imp.context.get_or_insert_with(|| Arc::new(Vec::new()));
        Arc::make_mut(vec).push(context.into());
        self
    }

    pub fn contexts(&self) -> &[OperationContext] {
        self.imp.context.as_deref().map_or(&[], |v| v.as_ref())
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.imp.detail = Some(detail.into());
        self
    }

    pub fn err<V>(self) -> Result<V, Self> {
        Err(self)
    }

    fn context_slice(&self) -> &[OperationContext] {
        self.imp.context.as_deref().map_or(&[], |v| v.as_ref())
    }

    pub fn action_main(&self) -> Option<String> {
        self.context_slice()
            .iter()
            .rev()
            .find_map(|ctx| ctx.action().clone())
    }

    pub fn locator_main(&self) -> Option<String> {
        self.context_slice()
            .iter()
            .rev()
            .find_map(|ctx| ctx.locator().clone())
    }

    pub fn path_segments(&self) -> Vec<String> {
        let mut path = Vec::new();
        let mut pending_locators: Vec<String> = Vec::new();

        for ctx in self.context_slice().iter().rev() {
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

// ---------------------------------------------------------------------------
// ContextAdd
// ---------------------------------------------------------------------------

impl<T: DomainReason> ContextAdd<&OperationContext> for StructError<T> {
    fn add_context(&mut self, ctx: &OperationContext) {
        let vec = self.imp.context.get_or_insert_with(|| Arc::new(Vec::new()));
        Arc::make_mut(vec).push(ctx.clone());
    }
}

impl<T: DomainReason> ContextAdd<OperationContext> for StructError<T> {
    fn add_context(&mut self, ctx: OperationContext) {
        let vec = self.imp.context.get_or_insert_with(|| Arc::new(Vec::new()));
        Arc::make_mut(vec).push(ctx);
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

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

        let ctx_slice = self.context_slice();
        if !ctx_slice.is_empty() {
            writeln!(f, "\n  -> Context stack:")?;

            for (i, c) in ctx_slice.iter().enumerate() {
                writeln!(f, "context {i}: ")?;
                writeln!(f, "{c}")?;
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ErrorWith
// ---------------------------------------------------------------------------

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

