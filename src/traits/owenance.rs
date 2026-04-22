use crate::{core::DomainReason, StructError};

/// 非结构错误(StructError) 转化为结构错误。
///
use std::fmt::Display;
pub trait ErrorOweBase<T, R>
where
    R: DomainReason,
{
    #[deprecated(
        since = "0.7.0",
        note = "prefer into_as(...) for real errors; keep owe(...) only for legacy Display-only values"
    )]
    fn owe(self, reason: R) -> Result<T, StructError<R>>;
}

impl<T, E, R> ErrorOweBase<T, R> for Result<T, E>
where
    E: Display,
    R: DomainReason,
{
    fn owe(self, reason: R) -> Result<T, StructError<R>> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                let msg = e.to_string();
                Err(StructError::from(reason).with_detail(msg))
            }
        }
    }
}
