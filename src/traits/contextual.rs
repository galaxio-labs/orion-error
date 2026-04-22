use crate::OperationContext;

pub trait ErrorWith {
    #[deprecated(
        since = "0.7.0",
        note = "use doing(...) for action contexts; use at(...) for locator/resource contexts"
    )]
    fn want<S: Into<String>>(self, desc: S) -> Self;
    fn position<S: Into<String>>(self, desc: S) -> Self;
    fn attach_context<C: Into<OperationContext>>(self, ctx: C) -> Self;
    #[deprecated(
        since = "0.7.0",
        note = "use attach_context(...) for full context frames; use at(...) / doing(...) for semantic context helpers"
    )]
    fn with<C: Into<OperationContext>>(self, ctx: C) -> Self
    where
        Self: Sized,
    {
        self.attach_context(ctx)
    }
    fn doing<S: Into<String>>(self, desc: S) -> Self
    where
        Self: Sized,
    {
        self.attach_context(OperationContext::doing(desc))
    }
    fn at<C: Into<OperationContext>>(self, ctx: C) -> Self
    where
        Self: Sized,
    {
        self.attach_context(ctx.into().into_at_context())
    }
}

impl<T, E: ErrorWith> ErrorWith for Result<T, E> {
    #[allow(deprecated)]
    fn want<S: Into<String>>(self, desc: S) -> Self {
        self.map_err(|e| e.want(desc))
    }
    fn position<S: Into<String>>(self, desc: S) -> Self {
        self.map_err(|e| e.position(desc))
    }
    fn attach_context<C: Into<OperationContext>>(self, ctx: C) -> Self {
        self.map_err(|e| e.attach_context(ctx))
    }
}
