use crate::core::OperationContext;

/// Extension methods for attaching context and position to an error.
///
/// Implemented for [`StructError`](crate::StructError) and
/// `Result<T, E>` where `E: ErrorWith`.
///
/// # Example
/// ```rust
/// use orion_error::prelude::*;
/// use orion_error::UvsReason;
///
/// let err = StructError::from(UvsReason::validation_error())
///     .doing("parse config")
///     .at("config.toml");
///
/// assert_eq!(err.action_main().as_deref(), Some("parse config"));
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
