//! Error identity types and [`StructError`] identity snapshot.

use crate::core::{DomainReason, ErrorCategory};
use crate::reason::ErrorIdentityProvider;

use super::carrier::StructError;

/// Identity-first snapshot view of a [`StructError`].
///
/// Carries `code`, `category`, and optional detail/position/path for
/// governance, testing, policy decisions, and protocol projections.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorIdentity {
    pub code: String,
    pub category: ErrorCategory,
    pub reason: String,
    pub detail: Option<String>,
    pub position: Option<String>,
    pub path: Option<String>,
}

impl<T> StructError<T>
where
    T: DomainReason + ErrorIdentityProvider,
{
    /// Build an [`ErrorIdentity`] from this error.
    pub fn identity_snapshot(&self) -> ErrorIdentity {
        ErrorIdentity {
            code: self.stable_code().to_string(),
            category: self.error_category(),
            reason: self.reason().to_string(),
            detail: self.detail().clone(),
            position: self.position().clone(),
            path: self.target_path(),
        }
    }
}
