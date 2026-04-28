use super::{DomainReason, ErrorCode, StructError};

/// Print an error with its full source chain to stderr.
///
/// This is a convenience wrapper around [`StructError::display_chain()`]
/// intended for binary/CLI entry points.
///
/// # Example
///
/// ```rust,ignore
/// use orion_error::report::print_error;
///
/// fn main() {
///     if let Err(err) = run() {
///         print_error(&err);
///         std::process::exit(1);
///     }
/// }
/// ```
pub fn print_error<R>(err: &StructError<R>)
where
    R: DomainReason + ErrorCode,
{
    eprintln!("{}", err.display_chain());
}
