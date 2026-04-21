use crate::OperationContext;

pub trait ErrorWith {
    fn want<S: Into<String>>(self, desc: S) -> Self;
    fn position<S: Into<String>>(self, desc: S) -> Self;
    fn with<C: Into<OperationContext>>(self, ctx: C) -> Self;
    fn doing<S: Into<String>>(self, desc: S) -> Self
    where
        Self: Sized,
    {
        self.want(desc)
    }
    fn at<C: Into<OperationContext>>(self, ctx: C) -> Self
    where
        Self: Sized,
    {
        self.with(ctx)
    }
}

impl<T, E: ErrorWith> ErrorWith for Result<T, E> {
    fn want<S: Into<String>>(self, desc: S) -> Self {
        self.map_err(|e| e.want(desc))
    }
    fn position<S: Into<String>>(self, desc: S) -> Self {
        self.map_err(|e| e.position(desc))
    }
    fn with<C: Into<OperationContext>>(self, ctx: C) -> Self {
        self.map_err(|e| e.with(ctx))
    }
}
