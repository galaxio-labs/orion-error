use crate::core::OperationContext;

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
