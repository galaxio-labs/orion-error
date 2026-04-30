use std::fmt::{Debug, Display};

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
