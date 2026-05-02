use std::fmt::{Debug, Display};

/// Marker trait for domain-specific error reason types.
///
/// Implement this on your project's error reason enum so it can be used
/// as the generic parameter of [`StructError`](crate::StructError).
///
/// # Constraints
///
/// | Bound | Reason |
/// |-------|--------|
/// | `Display` + `Debug` | Errors must be printable for diagnostics and logging. |
/// | `PartialEq` | Enables assertion in tests (`assert_eq!(err.reason(), MyReason::Foo)`). |
/// | `Send + Sync` | Required for `StructError` to be `Send + Sync`, which is needed when errors cross async task boundaries or are captured by `anyhow::Error` / `Box<dyn Error>`. |
/// | `'static` | Enables type erasure via `dyn Error` and storage in `SourceFrame`. |
///
/// These bounds match the de-facto standard for Rust error types (the Error trait
/// requires `'static`, and practical use requires `Send` for thread safety).
///
/// # Derive
///
/// Prefer `#[derive(OrionError)]` (requires the `derive` feature), which
/// also implements [`ErrorCode`](crate::reason::ErrorCode) and
/// [`ErrorIdentityProvider`](crate::reason::ErrorIdentityProvider).
pub trait DomainReason: PartialEq + Display + Debug + Send + Sync + 'static {}
