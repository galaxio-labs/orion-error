use crate::{core::DomainReason, StructError, UvsFrom};

/// 非结构错误(StructError) 转化为结构错误。
///
use std::{error::Error as StdError, fmt::Display};
pub trait ErrorOweBase<T, R>
where
    R: DomainReason,
{
    fn owe(self, reason: R) -> Result<T, StructError<R>>;
}

pub trait ErrorOweSourceBase<T, R>
where
    R: DomainReason,
{
    fn owe_source(self, reason: R) -> Result<T, StructError<R>>;
}

pub trait ErrorOwe<T, R>: ErrorOweBase<T, R>
where
    R: DomainReason + UvsFrom,
{
    fn owe_logic(self) -> Result<T, StructError<R>>;
    fn owe_biz(self) -> Result<T, StructError<R>>;
    fn owe_rule(self) -> Result<T, StructError<R>>;
    fn owe_validation(self) -> Result<T, StructError<R>>;
    fn owe_data(self) -> Result<T, StructError<R>>;
    fn owe_conf(self) -> Result<T, StructError<R>>;
    fn owe_res(self) -> Result<T, StructError<R>>;
    fn owe_net(self) -> Result<T, StructError<R>>;
    fn owe_timeout(self) -> Result<T, StructError<R>>;
    fn owe_sys(self) -> Result<T, StructError<R>>;
}

pub trait ErrorOweSource<T, R>: ErrorOweSourceBase<T, R>
where
    R: DomainReason + UvsFrom,
{
    fn owe_logic_source(self) -> Result<T, StructError<R>>;
    fn owe_biz_source(self) -> Result<T, StructError<R>>;
    fn owe_rule_source(self) -> Result<T, StructError<R>>;
    fn owe_validation_source(self) -> Result<T, StructError<R>>;
    fn owe_data_source(self) -> Result<T, StructError<R>>;
    fn owe_conf_source(self) -> Result<T, StructError<R>>;
    fn owe_res_source(self) -> Result<T, StructError<R>>;
    fn owe_net_source(self) -> Result<T, StructError<R>>;
    fn owe_timeout_source(self) -> Result<T, StructError<R>>;
    fn owe_sys_source(self) -> Result<T, StructError<R>>;
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

impl<T, E, R> ErrorOwe<T, R> for Result<T, E>
where
    E: Display,
    R: DomainReason + UvsFrom,
{
    fn owe_logic(self) -> Result<T, StructError<R>> {
        map_err_with(self, <R as UvsFrom>::from_logic)
    }
    fn owe_biz(self) -> Result<T, StructError<R>> {
        map_err_with(self, <R as UvsFrom>::from_biz)
    }
    fn owe_rule(self) -> Result<T, StructError<R>> {
        map_err_with(self, <R as UvsFrom>::from_rule)
    }
    fn owe_validation(self) -> Result<T, StructError<R>> {
        map_err_with(self, <R as UvsFrom>::from_validation)
    }
    fn owe_data(self) -> Result<T, StructError<R>> {
        map_err_with(self, <R as UvsFrom>::from_data)
    }
    fn owe_conf(self) -> Result<T, StructError<R>> {
        map_err_with(self, <R as UvsFrom>::from_conf)
    }
    fn owe_res(self) -> Result<T, StructError<R>> {
        map_err_with(self, <R as UvsFrom>::from_res)
    }
    fn owe_net(self) -> Result<T, StructError<R>> {
        map_err_with(self, <R as UvsFrom>::from_net)
    }
    fn owe_timeout(self) -> Result<T, StructError<R>> {
        map_err_with(self, <R as UvsFrom>::from_timeout)
    }
    fn owe_sys(self) -> Result<T, StructError<R>> {
        map_err_with(self, <R as UvsFrom>::from_sys)
    }
}

impl<T, E, R> ErrorOweSourceBase<T, R> for Result<T, E>
where
    E: StdError + Send + Sync + 'static,
    R: DomainReason,
{
    fn owe_source(self, reason: R) -> Result<T, StructError<R>> {
        self.map_err(|e| {
            let detail = e.to_string();
            StructError::from(reason).with_detail(detail).with_source(e)
        })
    }
}

impl<T, E, R> ErrorOweSource<T, R> for Result<T, E>
where
    E: StdError + Send + Sync + 'static,
    R: DomainReason + UvsFrom,
{
    fn owe_logic_source(self) -> Result<T, StructError<R>> {
        map_err_with_source(self, <R as UvsFrom>::from_logic)
    }
    fn owe_biz_source(self) -> Result<T, StructError<R>> {
        map_err_with_source(self, <R as UvsFrom>::from_biz)
    }
    fn owe_rule_source(self) -> Result<T, StructError<R>> {
        map_err_with_source(self, <R as UvsFrom>::from_rule)
    }
    fn owe_validation_source(self) -> Result<T, StructError<R>> {
        map_err_with_source(self, <R as UvsFrom>::from_validation)
    }
    fn owe_data_source(self) -> Result<T, StructError<R>> {
        map_err_with_source(self, <R as UvsFrom>::from_data)
    }
    fn owe_conf_source(self) -> Result<T, StructError<R>> {
        map_err_with_source(self, <R as UvsFrom>::from_conf)
    }
    fn owe_res_source(self) -> Result<T, StructError<R>> {
        map_err_with_source(self, <R as UvsFrom>::from_res)
    }
    fn owe_net_source(self) -> Result<T, StructError<R>> {
        map_err_with_source(self, <R as UvsFrom>::from_net)
    }
    fn owe_timeout_source(self) -> Result<T, StructError<R>> {
        map_err_with_source(self, <R as UvsFrom>::from_timeout)
    }
    fn owe_sys_source(self) -> Result<T, StructError<R>> {
        map_err_with_source(self, <R as UvsFrom>::from_sys)
    }
}

fn map_err_with<T, E, R, F>(result: Result<T, E>, f: F) -> Result<T, StructError<R>>
where
    E: Display,
    R: DomainReason,
    F: FnOnce() -> R,
{
    result.map_err(|e| {
        let detail = e.to_string();
        let reason = f();
        StructError::from(reason).with_detail(detail)
    })
}

fn map_err_with_source<T, E, R, F>(result: Result<T, E>, f: F) -> Result<T, StructError<R>>
where
    E: StdError + Send + Sync + 'static,
    R: DomainReason,
    F: FnOnce() -> R,
{
    result.map_err(|e| {
        let detail = e.to_string();
        let reason = f();
        StructError::from(reason).with_detail(detail).with_source(e)
    })
}
