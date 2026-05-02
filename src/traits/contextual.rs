use crate::core::OperationContext;

/// Extension methods for attaching operation context and position to an error.
///
/// [`doing("...")`](ErrorWith::doing) and [`at("...")`](ErrorWith::at) are the
/// primary ways to annotate where and what was happening when the error occurred.
///
/// Implemented for [`StructError`](crate::StructError) and
/// `Result<T, E>` where `E: ErrorWith`.
///
/// # Example
///
/// ```rust
/// use orion_error::prelude::*;
/// use orion_error::UnifiedReason;
///
/// let err = StructError::from(UnifiedReason::validation_error())
///     .doing("parse config")      // what operation
///     .at("config.toml");          // what resource
///
/// assert_eq!(err.action_main().as_deref(), Some("parse config"));
/// assert_eq!(err.locator_main().as_deref(), Some("config.toml"));
///
/// // doing/at also works on Result chains:
/// let result: Result<(), StructError<UnifiedReason>> =
///     Err(StructError::from(UnifiedReason::validation_error()))
///         .doing("validate")
///         .at("input.json");
/// assert!(result.is_err());
/// ```
pub trait ErrorWith {
    fn position<S: Into<String>>(self, desc: S) -> Self;
    fn with_context<C: Into<OperationContext>>(self, ctx: C) -> Self;
    fn doing<S: Into<String>>(self, desc: S) -> Self
    where
        Self: Sized,
    {
        self.with_context(OperationContext::doing(desc))
    }
    fn at<C: Into<OperationContext>>(self, ctx: C) -> Self
    where
        Self: Sized,
    {
        self.with_context(ctx.into().into_at_context())
    }
}

impl<T, E: ErrorWith> ErrorWith for Result<T, E> {
    fn position<S: Into<String>>(self, desc: S) -> Self {
        self.map_err(|e| e.position(desc))
    }
    fn with_context<C: Into<OperationContext>>(self, ctx: C) -> Self {
        self.map_err(|e| e.with_context(ctx))
    }
}
