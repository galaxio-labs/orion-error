use std::{error::Error as StdError, fmt};

use crate::{core::{DomainReason, OwnedDynStdStructError}, StructError};

mod private {
    pub trait Sealed {}
}

/// Marker trait for explicitly opt-in raw `std::error::Error` sources.
///
/// This is the explicit escape hatch for downstream crates that have their own raw
/// `StdError` types and want to route them through `raw_source(...)` before
/// calling `into_as(...)`.
///
/// Implement this trait only for genuine non-structured raw error types.
/// Do not implement it for wrappers around `StructError<_>`.
pub trait RawStdError: StdError + Send + Sync + 'static {}

#[doc(hidden)]
pub trait UnstructuredSource: private::Sealed {
    fn into_struct_error<R>(self, reason: R, detail: String) -> StructError<R>
    where
        R: DomainReason;
}

pub trait IntoAs<T, R: DomainReason>: Sized {
    fn into_as(self, reason: R, detail: impl Into<String>) -> Result<T, StructError<R>>;
}

#[derive(Debug)]
pub struct RawSource<E>(E);

/// Explicitly mark an opt-in raw `std::error::Error` as an unstructured source.
///
/// This is a narrow explicit escape hatch. It does **not** provide a blanket
/// `E: StdError` path, and it must not be used for `StructError<_>`.
///
/// Downstream crates may opt in their own raw `StdError` types by implementing
/// [`RawStdError`], instead of relying on a blanket `E: StdError` fallback.
///
/// ```rust
/// use std::fmt;
///
/// use orion_error::{IntoAs, UvsReason};
/// use orion_error::bridge::{raw_source, RawStdError};
///
/// #[derive(Debug)]
/// struct ThirdPartyError;
///
/// impl fmt::Display for ThirdPartyError {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "third-party failure")
///     }
/// }
///
/// impl std::error::Error for ThirdPartyError {}
/// impl RawStdError for ThirdPartyError {}
///
/// let result: Result<(), ThirdPartyError> = Err(ThirdPartyError);
/// let err = result
///     .map_err(raw_source)
///     .into_as(UvsReason::system_error(), "load failed")
///     .expect_err("expected structured error");
///
/// assert_eq!(err.source_ref().unwrap().to_string(), "third-party failure");
/// ```
///
/// ```compile_fail
/// use orion_error::{StructError, UvsReason};
/// use orion_error::bridge::{raw_source, RawStdError};
///
/// let structured = StructError::from(UvsReason::system_error());
/// let _ = raw_source(structured);
/// ```
pub fn raw_source<E>(err: E) -> RawSource<E>
where
    E: RawStdError,
{
    RawSource(err)
}

impl<E> RawSource<E> {
    pub fn into_inner(self) -> E {
        self.0
    }

    pub fn inner(&self) -> &E {
        &self.0
    }
}

impl<E: fmt::Display> fmt::Display for RawSource<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<E> StdError for RawSource<E>
where
    E: RawStdError,
{
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.0)
    }
}

impl<T, E, R> IntoAs<T, R> for Result<T, E>
where
    E: UnstructuredSource,
    R: DomainReason,
{
    fn into_as(self, reason: R, detail: impl Into<String>) -> Result<T, StructError<R>> {
        let detail = detail.into();
        self.map_err(|err| err.into_struct_error(reason, detail))
    }
}

fn attach_std_source<E, R>(err: E, reason: R, detail: String) -> StructError<R>
where
    E: StdError + Send + Sync + 'static,
    R: DomainReason,
{
    StructError::from(reason)
        .with_detail(detail)
        .with_std_source(err)
}

impl RawStdError for std::io::Error {}

impl private::Sealed for std::io::Error {}

impl UnstructuredSource for std::io::Error {
    fn into_struct_error<R>(self, reason: R, detail: String) -> StructError<R>
    where
        R: DomainReason,
    {
        attach_std_source(self, reason, detail)
    }
}

impl<E> private::Sealed for RawSource<E> where E: RawStdError {}

impl<E> UnstructuredSource for RawSource<E>
where
    E: RawStdError,
{
    fn into_struct_error<R>(self, reason: R, detail: String) -> StructError<R>
    where
        R: DomainReason,
    {
        attach_std_source(self.0, reason, detail)
    }
}

#[cfg(feature = "anyhow")]
#[derive(Debug)]
struct AnyhowStdSource(anyhow::Error);

#[cfg(feature = "anyhow")]
impl fmt::Display for AnyhowStdSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "anyhow")]
impl StdError for AnyhowStdSource {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0.source()
    }
}

#[cfg(feature = "anyhow")]
impl private::Sealed for anyhow::Error {}

#[cfg(feature = "anyhow")]
impl UnstructuredSource for anyhow::Error {
    fn into_struct_error<R>(self, reason: R, detail: String) -> StructError<R>
    where
        R: DomainReason,
    {
        match self.downcast::<OwnedDynStdStructError>() {
            Ok(source) => StructError::from(reason)
                .with_detail(detail)
                .with_dyn_struct_source(source),
            Err(err) => attach_std_source(AnyhowStdSource(err), reason, detail),
        }
    }
}

#[cfg(feature = "serde_json")]
impl RawStdError for serde_json::Error {}

#[cfg(feature = "serde_json")]
impl private::Sealed for serde_json::Error {}

#[cfg(feature = "serde_json")]
impl UnstructuredSource for serde_json::Error {
    fn into_struct_error<R>(self, reason: R, detail: String) -> StructError<R>
    where
        R: DomainReason,
    {
        attach_std_source(self, reason, detail)
    }
}

#[cfg(feature = "toml")]
impl RawStdError for toml::de::Error {}

#[cfg(feature = "toml")]
impl private::Sealed for toml::de::Error {}

#[cfg(feature = "toml")]
impl UnstructuredSource for toml::de::Error {
    fn into_struct_error<R>(self, reason: R, detail: String) -> StructError<R>
    where
        R: DomainReason,
    {
        attach_std_source(self, reason, detail)
    }
}

#[cfg(feature = "toml")]
impl RawStdError for toml::ser::Error {}

#[cfg(feature = "toml")]
impl private::Sealed for toml::ser::Error {}

#[cfg(feature = "toml")]
impl UnstructuredSource for toml::ser::Error {
    fn into_struct_error<R>(self, reason: R, detail: String) -> StructError<R>
    where
        R: DomainReason,
    {
        attach_std_source(self, reason, detail)
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt, io};

    use super::{raw_source, IntoAs, RawStdError};
    #[cfg(feature = "anyhow")]
    use crate::StructError;
    use crate::UvsReason;

    #[derive(Debug)]
    struct ThirdPartyError(&'static str);

    impl fmt::Display for ThirdPartyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for ThirdPartyError {}

    impl RawStdError for ThirdPartyError {}

    #[test]
    fn test_into_as_for_io_error() {
        let result: Result<(), io::Error> = Err(io::Error::other("disk offline"));

        let err = result
            .into_as(UvsReason::system_error(), "load config failed")
            .expect_err("expected structured error");

        assert_eq!(err.detail().as_deref(), Some("load config failed"));
        assert_eq!(err.source_ref().unwrap().to_string(), "disk offline");
    }

    #[test]
    fn test_into_as_for_raw_source_wrapper() {
        let result: Result<(), ThirdPartyError> = Err(ThirdPartyError("parser aborted"));

        let err = result
            .map_err(raw_source)
            .into_as(UvsReason::validation_error(), "parse config failed")
            .expect_err("expected structured error");

        assert_eq!(err.detail().as_deref(), Some("parse config failed"));
        assert_eq!(err.source_ref().unwrap().to_string(), "parser aborted");
    }

    #[cfg(feature = "anyhow")]
    #[test]
    fn test_into_as_for_anyhow_defaults_to_unstructured_source() {
        let result: Result<(), anyhow::Error> = Err(anyhow::anyhow!("network offline"));

        let err = result
            .into_as(UvsReason::system_error(), "load config failed")
            .expect_err("expected structured error");

        assert_eq!(err.detail().as_deref(), Some("load config failed"));
        assert_eq!(err.source_ref().unwrap().to_string(), "network offline");
        assert_eq!(err.source_frames()[0].message, "network offline");
    }

    #[cfg(feature = "anyhow")]
    #[test]
    fn test_into_as_for_anyhow_extracts_top_level_official_dyn_bridge() {
        let structured = StructError::from(UvsReason::validation_error())
            .with_detail("invalid port")
            .with_std_source(io::Error::other("not a number"));
        let structured_display = structured.to_string();
        let result: Result<(), anyhow::Error> = Err(anyhow::Error::new(structured.into_dyn_std()));

        let err = result
            .into_as(UvsReason::system_error(), "load config failed")
            .expect_err("expected structured error");

        assert_eq!(err.detail().as_deref(), Some("load config failed"));
        assert_eq!(err.source_ref().unwrap().to_string(), structured_display);
        assert_eq!(err.source_frames()[0].message, "validation error");
        assert_eq!(
            err.source_frames()[0].reason.as_deref(),
            Some("validation error")
        );
        assert_eq!(
            err.source_frames()[0].detail.as_deref(),
            Some("invalid port")
        );
        assert_eq!(err.root_cause().unwrap().to_string(), "not a number");
    }
}
