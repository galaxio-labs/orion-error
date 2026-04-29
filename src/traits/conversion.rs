use crate::{core::convert_error, core::DomainReason, StructError};

pub trait ErrorConv<T, R: DomainReason>: Sized {
    fn err_conv(self) -> Result<T, StructError<R>>;
}

pub trait ConvStructError<R: DomainReason>: Sized {
    fn conv(self) -> StructError<R>;
}


pub trait ErrorWrapAs<T, R: DomainReason>: Sized {
    fn wrap_as(self, reason: R, detail: impl Into<String>) -> Result<T, StructError<R>>;
}

pub trait WrapStructErrorAs<R: DomainReason>: Sized {
    fn wrap_as(self, reason: R, detail: impl Into<String>) -> StructError<R>;
}

impl<T, R1, R2> ErrorConv<T, R2> for Result<T, StructError<R1>>
where
    R1: DomainReason,
    R2: DomainReason + From<R1>,
{
    fn err_conv(self) -> Result<T, StructError<R2>> {
        match self {
            Ok(o) => Ok(o),
            Err(e) => Err(convert_error::<R1, R2>(e)),
        }
    }
}

impl<R1, R2> ConvStructError<R2> for StructError<R1>
where
    R1: DomainReason,
    R2: DomainReason + From<R1>,
{
    fn conv(self) -> StructError<R2> {
        convert_error::<R1, R2>(self)
    }
}


impl<T, R1, R2> ErrorWrapAs<T, R2> for Result<T, StructError<R1>>
where
    R1: DomainReason,
    R2: DomainReason,
{
    fn wrap_as(self, reason: R2, detail: impl Into<String>) -> Result<T, StructError<R2>> {
        let detail = detail.into();
        self.map_err(|e| e.wrap_as(reason, detail))
    }
}

impl<R1, R2> WrapStructErrorAs<R2> for StructError<R1>
where
    R1: DomainReason,
    R2: DomainReason,
{
    fn wrap_as(self, reason: R2, detail: impl Into<String>) -> StructError<R2> {
        self.wrap(reason).with_detail(detail)
    }
}

pub trait ToStructError<R>
where
    R: DomainReason,
{
    fn to_err(self) -> StructError<R>;
    fn err_result<T>(self) -> Result<T, StructError<R>>;
}
impl<R> ToStructError<R> for R
where
    R: DomainReason,
{
    fn to_err(self) -> StructError<R> {
        StructError::from(self)
    }
    fn err_result<T>(self) -> Result<T, StructError<R>> {
        Err(StructError::from(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::DomainReason, ErrorCode, OperationContext, StructError, UvsReason};

    // 定义测试用的 DomainReason
    #[derive(Debug, Clone, PartialEq, thiserror::Error)]
    enum TestReason {
        #[error("test error")]
        TestError,
        #[error("{0}")]
        Uvs(UvsReason),
    }

    impl ErrorCode for TestReason {
        fn error_code(&self) -> i32 {
            match self {
                TestReason::TestError => 1001,
                TestReason::Uvs(uvs) => uvs.error_code(),
            }
        }
    }

    impl DomainReason for TestReason {}

    impl From<UvsReason> for TestReason {
        fn from(uvs: UvsReason) -> Self {
            TestReason::Uvs(uvs)
        }
    }

    // 定义另一个 DomainReason 用于测试转换
    #[derive(Debug, Clone, PartialEq, thiserror::Error)]
    enum AnotherReason {
        #[error("another error")]
        AnotherError,
        #[error("{0}")]
        Uvs(UvsReason),
    }

    impl ErrorCode for AnotherReason {
        fn error_code(&self) -> i32 {
            match self {
                AnotherReason::AnotherError => 2001,
                AnotherReason::Uvs(uvs) => uvs.error_code(),
            }
        }
    }

    impl DomainReason for AnotherReason {}

    impl From<UvsReason> for AnotherReason {
        fn from(uvs: UvsReason) -> Self {
            AnotherReason::Uvs(uvs)
        }
    }

    impl From<TestReason> for AnotherReason {
        fn from(test: TestReason) -> Self {
            match test {
                TestReason::TestError => AnotherReason::AnotherError,
                TestReason::Uvs(uvs) => AnotherReason::Uvs(uvs),
            }
        }
    }

    #[test]
    fn test_error_conv_trait() {
        // 测试 ErrorConv trait 的 err_conv 方法
        let original_result: Result<i32, StructError<TestReason>> =
            Err(TestReason::TestError.to_err());

        let converted_result: Result<i32, StructError<AnotherReason>> = original_result.err_conv();

        assert!(converted_result.is_err());
        let converted_error = converted_result.unwrap_err();
        assert_eq!(converted_error.reason().error_code(), 2001);

        // 测试成功情况下的转换
        let success_result: Result<i32, StructError<TestReason>> = Ok(42);
        let converted_success: Result<i32, StructError<AnotherReason>> = success_result.err_conv();

        assert!(converted_success.is_ok());
        assert_eq!(converted_success.unwrap(), 42);
    }

    #[test]
    fn test_conv_struct_error_trait() {
        // 测试 ConvStructError trait 的 conv 方法
        let original_error: StructError<TestReason> = TestReason::TestError.to_err();

        let converted_error: StructError<AnotherReason> = original_error.conv();

        assert_eq!(converted_error.reason().error_code(), 2001);

        // 测试带有 UvsReason 的转换
        let uvs_error: StructError<TestReason> =
            TestReason::Uvs(UvsReason::network_error()).to_err();

        let converted_uvs_error: StructError<AnotherReason> = uvs_error.conv();

        assert_eq!(converted_uvs_error.reason().error_code(), 202);
    }

    #[test]
    fn test_to_struct_error_trait() {
        // 测试 ToStructError trait 的 to_err 方法
        let reason = TestReason::TestError;
        let error: StructError<TestReason> = reason.to_err();

        assert_eq!(error.reason().error_code(), 1001);

        // 测试 ToStructError trait 的 err_result 方法
        let reason2 = TestReason::TestError;
        let result: Result<String, StructError<TestReason>> = reason2.err_result();

        assert!(result.is_err());
        let error_from_result = result.unwrap_err();
        assert_eq!(error_from_result.reason().error_code(), 1001);

        // 测试使用 UvsReason
        let uvs_reason1 = UvsReason::validation_error();
        let uvs_error: StructError<UvsReason> = uvs_reason1.to_err();

        assert_eq!(uvs_error.reason().error_code(), 100);

        let uvs_reason2 = UvsReason::validation_error();
        let uvs_result: Result<i32, StructError<UvsReason>> = uvs_reason2.err_result();
        assert!(uvs_result.is_err());
        assert_eq!(uvs_result.unwrap_err().reason().error_code(), 100);
    }

    #[test]
    fn test_err_conv_preserves_source() {
        let source = std::io::Error::other("db unavailable");
        let original: Result<i32, StructError<TestReason>> =
            Err(StructError::from(TestReason::TestError).with_std_source(source));

        let converted: Result<i32, StructError<AnotherReason>> = original.err_conv();
        let err = converted.unwrap_err();

        assert_eq!(err.reason().error_code(), 2001);
        assert_eq!(err.source_ref().unwrap().to_string(), "db unavailable");
    }

    #[test]
    fn test_wrap_as_preserves_previous_struct_error_chain() {
        let original: Result<i32, StructError<TestReason>> =
            Err(StructError::from(TestReason::TestError)
                .with_detail("repo layer failed")
                .with_std_source(std::io::Error::other("db unavailable")));

        let wrapped: Result<i32, StructError<AnotherReason>> =
            original.wrap_as(AnotherReason::AnotherError, "service layer failed");
        let err = wrapped.unwrap_err();

        assert_eq!(err.reason().error_code(), 2001);
        assert_eq!(err.detail().as_deref(), Some("service layer failed"));
        assert_eq!(
            err.source_ref().unwrap().to_string(),
            "test error\n  -> Details: repo layer failed\n  -> Source: db unavailable"
        );
        assert_eq!(err.root_cause().unwrap().to_string(), "db unavailable");
        assert_eq!(err.source_chain().len(), 2);
        assert_eq!(err.source_frames()[0].message, "test error");
        assert!(err.source_frames()[0]
            .display
            .as_ref()
            .unwrap()
            .contains("repo layer failed"));
        assert_eq!(err.source_frames()[0].error_code, None);
        assert_eq!(err.source_frames()[0].reason.as_deref(), Some("test error"));
        assert_eq!(
            err.source_frames()[0].detail.as_deref(),
            Some("repo layer failed")
        );
        assert_eq!(err.source_frames()[1].message, "db unavailable");
        assert!(err.source_frames()[1].is_root_cause);
    }

    #[test]
    fn test_err_conv_preserves_context_metadata() {
        let original: Result<i32, StructError<TestReason>> =
            Err(StructError::from(TestReason::TestError).with_context(
                OperationContext::doing("load sink defaults")
                    .with_meta("config.kind", "sink_defaults"),
            ));

        let converted: Result<i32, StructError<AnotherReason>> = original.err_conv();
        let err = converted.unwrap_err();

        assert_eq!(
            err.context_metadata().get_str("config.kind"),
            Some("sink_defaults")
        );
    }

    #[test]
    fn test_wrap_as_preserves_source_frame_metadata() {
        let original: Result<i32, StructError<TestReason>> =
            Err(StructError::from(TestReason::TestError).with_context(
                OperationContext::doing("load sink defaults")
                    .with_meta("config.kind", "sink_defaults"),
            ));

        let wrapped: Result<i32, StructError<AnotherReason>> =
            original.wrap_as(AnotherReason::AnotherError, "service layer failed");
        let err = wrapped.unwrap_err();

        assert_eq!(err.detail().as_deref(), Some("service layer failed"));
        assert_eq!(
            err.source_frames()[0].metadata.get_str("config.kind"),
            Some("sink_defaults")
        );
    }

    #[test]
    fn test_wrap_as_preserves_detail_source_chain_and_metadata() {
        let original: Result<i32, StructError<TestReason>> =
            Err(StructError::from(TestReason::TestError)
                .with_detail("repo layer failed")
                .with_context(
                    OperationContext::doing("load sink defaults")
                        .with_meta("config.kind", "sink_defaults"),
                )
                .with_std_source(std::io::Error::other("db unavailable")));

        let wrapped: Result<i32, StructError<AnotherReason>> =
            original.wrap_as(AnotherReason::AnotherError, "service layer failed");
        let err = wrapped.unwrap_err();

        assert_eq!(err.reason().error_code(), 2001);
        assert_eq!(err.detail().as_deref(), Some("service layer failed"));
        assert!(err.source_ref().unwrap().to_string().contains("test error"));
        assert_eq!(err.root_cause().unwrap().to_string(), "db unavailable");
        assert_eq!(err.source_chain().len(), 2);
        assert_eq!(err.source_frames()[0].message, "test error");
        assert_eq!(err.source_frames()[0].error_code, None);
        assert_eq!(
            err.source_frames()[0].detail.as_deref(),
            Some("repo layer failed")
        );
        assert_eq!(
            err.source_frames()[0].metadata.get_str("config.kind"),
            Some("sink_defaults")
        );
        assert_eq!(err.source_frames()[1].message, "db unavailable");
        assert!(err.source_frames()[1].is_root_cause);
    }
}
