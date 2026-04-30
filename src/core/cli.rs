use super::{DomainReason, StructError};

/// Print an error with its full source chain to stderr.
///
/// This is a convenience wrapper around [`StructError::display_chain()`]
/// intended for binary/CLI entry points.
///
/// # Example
///
/// ```rust
/// use orion_error::{cli::print_error, StructError, UvsReason};
///
/// let err = StructError::from(UvsReason::system_error())
///     .with_detail("config not found");
/// print_error(&err);
/// ```
pub fn print_error<R>(err: &StructError<R>)
where
    R: DomainReason,
{
    eprintln!("{}", err.display_chain());
}
