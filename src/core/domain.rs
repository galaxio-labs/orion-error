use std::fmt::{Debug, Display};

use derive_more::From;
use thiserror::Error;

use super::UvsReason;

/// Marker trait for domain-specific error reason types.
///
/// Implement this on your project's error reason enum so it can be used
/// as the generic parameter of [`StructError`](crate::StructError).
///
/// # Requirements
///
/// The type must be `PartialEq + Display + Debug + Send + Sync + 'static`.
///
/// # Derive
///
/// Prefer `#[derive(OrionError)]` (requires the `derive` feature), which
/// also implements [`ErrorCode`](crate::reason::ErrorCode) and
/// [`ErrorIdentityProvider`](crate::reason::ErrorIdentityProvider).
pub trait DomainReason:
    PartialEq + Display + Debug + Send + Sync + 'static
{
}

/// Placeholder reason type that never carries meaningful semantics.
///
/// Used as a generic argument when the reason type is not yet known or
/// when a [`StructError`](crate::StructError) must be constructed without
/// a domain-specific reason.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Error, From)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum NullReason {
    #[allow(dead_code)]
    #[error("null")]
    Null,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl DomainReason for NullReason {}
