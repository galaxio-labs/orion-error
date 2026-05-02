use crate::reason::{ErrorCategory, ErrorIdentityProvider};
use crate::{core::DomainReason, StructError};

#[allow(dead_code)]
// Testing-only assertion trait (no message).
pub trait TestAssert {
    type Output;
    fn assert(self) -> Self::Output;
}

#[allow(dead_code)]
// Testing-only assertion trait (with message).
pub trait TestAssertWithMsg<A> {
    type Output;
    fn assert(self, msg: A) -> Self::Output;
}

impl<T, E> TestAssert for Result<T, E>
where
    E: std::fmt::Display,
{
    type Output = T;

    fn assert(self) -> T {
        self.unwrap_or_else(|e| panic!("[TEST ASSERTION FAILED] \n Error details: {e}"))
    }
}

impl<T, E> TestAssertWithMsg<&str> for Result<T, E>
where
    E: std::fmt::Display,
{
    type Output = T;

    fn assert(self, msg: &str) -> T {
        self.unwrap_or_else(|e| panic!("[TEST ASSERTION FAILED] {msg} \n Error details: {e}"))
    }
}

impl<T> TestAssert for Option<T> {
    type Output = T;

    fn assert(self) -> T {
        self.unwrap_or_else(|| panic!("[OPTION ASSERTION FAILED] ",))
    }
}

/// Assert that the error's **stable code string** matches.
///
/// This asserts [`ErrorIdentityProvider::stable_code`], **not** the numeric
/// [`ErrorCode::error_code`]. For numeric assertions, call `.error_code()` directly.
///
/// # Example
///
/// ```rust
/// use orion_error::UvsReason;
/// use orion_error::dev::testing::assert_err_code;
/// use orion_error::StructError;
///
/// let err = StructError::from(UvsReason::system_error());
/// assert_err_code(&err, "sys.io_error");
/// ```
pub fn assert_err_code<R>(err: &StructError<R>, code: &str)
where
    R: DomainReason + ErrorIdentityProvider,
{
    assert_eq!(err.reason().stable_code(), code);
}

/// Assert that the error's [`ErrorCategory`] matches.
pub fn assert_err_category<R>(err: &StructError<R>, category: ErrorCategory)
where
    R: DomainReason + ErrorIdentityProvider,
{
    assert_eq!(err.reason().error_category(), category);
}

pub fn assert_err_identity<R>(err: &StructError<R>, code: &str, category: ErrorCategory)
where
    R: DomainReason + ErrorIdentityProvider,
{
    assert_err_code(err, code);
    assert_err_category(err, category);
}

pub fn assert_err_operation<R>(err: &StructError<R>, operation: &str)
where
    R: DomainReason,
{
    assert_eq!(err.action_main().as_deref(), Some(operation));
}

pub fn assert_err_path<R>(err: &StructError<R>, path: &str)
where
    R: DomainReason,
{
    assert_eq!(err.target_path().as_deref(), Some(path));
}

#[cfg(test)]
mod tests {
    use super::{
        assert_err_category, assert_err_code, assert_err_identity, assert_err_operation,
        assert_err_path,
    };
    use crate::conversion::ErrorWith;
    use crate::reason::ErrorCategory;
    use crate::{StructError, UvsReason};

    #[test]
    fn test_assert_err_code_helper() {
        let err = StructError::from(UvsReason::system_error());
        assert_err_code(&err, "sys.io_error");
    }

    #[test]
    fn test_assert_err_category_helper() {
        let err = StructError::from(UvsReason::business_error());
        assert_err_category(&err, ErrorCategory::Biz);
    }

    #[test]
    fn test_assert_err_identity_helper() {
        let err = StructError::from(UvsReason::network_error());
        assert_err_identity(&err, "sys.network_error", ErrorCategory::Sys);
    }

    #[test]
    fn test_assert_err_operation_helper() {
        let err = StructError::from(UvsReason::system_error())
            .with_detail("read config failed")
            .doing("load config");
        assert_err_operation(&err, "load config");
    }

    #[test]
    fn test_assert_err_path_helper() {
        let err = StructError::from(UvsReason::system_error())
            .with_detail("read config failed")
            .doing("load config")
            .at("config.toml");
        assert_err_path(&err, "load config / config.toml");
    }
}
