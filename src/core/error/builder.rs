//! Builder for constructing a [`StructError`] with optional detail, position,
//! context, and source attachments.
//!
//! Created via [`StructError::builder`].

use std::error::Error as StdError;

use super::carrier::StructError;
use crate::core::DomainReason;
use super::source_chain::{InternalSourcePayload, InternalSourceState};
use super::std_bridge::IntoSourcePayload;
use super::super::context::OperationContext;

pub struct StructErrorBuilder<T: DomainReason> {
    pub(crate) reason: T,
    pub(crate) detail: Option<String>,
    pub(crate) position: Option<String>,
    pub(crate) contexts: Vec<OperationContext>,
    pub(crate) source_payload: Option<InternalSourcePayload>,
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

    pub fn source_std<E>(self, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.with_internal_source(InternalSourceState::from_std(source))
    }

    #[allow(private_bounds)]
    pub fn source<S>(self, source: S) -> Self
    where
        S: IntoSourcePayload,
    {
        self.attach_source(source)
    }

    pub fn source_struct<R>(mut self, source: StructError<R>) -> Self
    where
        R: DomainReason,
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
