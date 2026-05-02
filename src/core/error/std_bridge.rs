//! Standard-error ecosystem bridge types.
//!
//! Wrappers that present a `StructError` through the `std::error::Error`
//! trait while preserving structured source-frame metadata.

use std::error::Error as StdError;
use std::fmt::Display;
use std::sync::Arc;

use super::carrier::StructError;
use super::source_chain::{
    collect_struct_error_source_frames, BoxedSource, InternalSourceState, SourceFrame,
};
use crate::core::DomainReason;
use crate::reason::{ErrorCategory, ErrorIdentityProvider};

// ---------------------------------------------------------------------------
// IntoSourcePayload – internal marker trait for source auto-routing
// ---------------------------------------------------------------------------

pub(crate) trait IntoSourcePayload {
    fn into_source_payload(self) -> InternalSourceState;
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

// ---------------------------------------------------------------------------
// ErrorIdentityProvider for StructError
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// internal_into_std_bridge
// ---------------------------------------------------------------------------

pub(crate) fn internal_into_std_bridge<R>(source: StructError<R>) -> BoxedSource
where
    R: DomainReason,
{
    Arc::new(OwnedStdStructError { inner: source })
}

// ---------------------------------------------------------------------------
// OwnedStdStructError
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OwnedStdStructError<R: DomainReason> {
    pub(crate) inner: StructError<R>,
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

// ---------------------------------------------------------------------------
// OwnedDynStdStructError
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OwnedDynStdStructError {
    display: String,
    source: Option<BoxedSource>,
    frames: Arc<Vec<SourceFrame>>,
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
        let frames = collect_struct_error_source_frames(&value);
        let imp = value.into_imp();
        let source = imp.source_payload.as_ref().map(|sp| sp.source_arc());
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

// ---------------------------------------------------------------------------
// StdStructRef
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct StdStructRef<'a, R: DomainReason> {
    inner: &'a StructError<R>,
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
