#[cfg(test)]
mod tests {

    use derive_more::From;
    use thiserror::Error;

    use crate::conversion::ErrorWith;
    use crate::reason::{DomainReason, ErrorCode};
    use crate::{
        core::convert_error, testing::TestAssertWithMsg, OperationContext, StructError,
        UnifiedReason,
    };

    // 测试用领域原因类型
    #[derive(Debug, PartialEq, Clone, Error, From)]
    enum TestDomainReason {
        #[error("why1")]
        Why1,
        #[error("{0}")]
        General(UnifiedReason),
    }

    impl ErrorCode for TestDomainReason {
        fn error_code(&self) -> i32 {
            match self {
                TestDomainReason::Why1 => 200,
                TestDomainReason::General(uvs_reason) => uvs_reason.error_code(),
            }
        }
    }

    impl DomainReason for TestDomainReason {}

    // 另一个领域原因类型用于转换测试
    #[derive(Debug, PartialEq, Clone, Error, From)]
    enum OtherDomainReason {
        #[error("why1")]
        Why2,
        #[error("{0}")]
        General(UnifiedReason),
    }

    impl DomainReason for OtherDomainReason {}

    impl ErrorCode for OtherDomainReason {
        fn error_code(&self) -> i32 {
            match self {
                OtherDomainReason::Why2 => 300,
                OtherDomainReason::General(uvs_reason) => uvs_reason.error_code(),
            }
        }
    }

    impl From<TestDomainReason> for OtherDomainReason {
        fn from(value: TestDomainReason) -> Self {
            match value {
                TestDomainReason::Why1 => Self::Why2,
                TestDomainReason::General(uvs_reason) => Self::General(uvs_reason),
            }
        }
    }

    #[test]
    fn test_domain_error_creation() {
        let err = StructError::from(TestDomainReason::Why1);

        assert_eq!(err.reason(), &TestDomainReason::Why1);
        assert_eq!(err.reason().error_code(), 200);
    }

    #[test]
    fn test_error_with_details() {
        let err = StructError::from(TestDomainReason::Why1).with_detail("detailed message");

        assert_eq!(err.detail(), &Some("detailed message".to_string()));
    }

    #[test]
    fn test_error_context() {
        let mut ctx = OperationContext::doing("user_profile");
        ctx.record("user_id", "12345");

        let err = StructError::from(TestDomainReason::Why1).with_context(ctx);

        assert_eq!(err.action_main(), Some("user_profile".to_string()));
        assert!(err
            .contexts()
            .first()
            .unwrap()
            .context()
            .items
            .contains(&("user_id".into(), "12345".into())));
    }

    #[test]
    fn test_error_conversion() {
        let original = StructError::from(TestDomainReason::Why1)
            .with_detail("conversion test")
            .with_position("test.rs:1")
            .with_context(OperationContext::doing("ctx").context().clone());

        let converted: StructError<OtherDomainReason> = convert_error(original);

        assert_eq!(converted.reason(), &OtherDomainReason::Why2);
        assert_eq!(converted.detail(), &Some("conversion test".into()));
    }

    #[test]
    fn test_error_display() {
        let mut ctx = OperationContext::new();
        ctx.record("step", "initialization");
        ctx.record("resource", "database");

        let err = StructError::from(TestDomainReason::General(UnifiedReason::core_conf()))
            .with_detail("missing db config")
            .with_position("src/config.rs:42")
            .doing("database_config")
            .with_context(ctx);

        let display_output = format!("{err}");
        println!("{display_output}");

        assert!(display_output.contains("configuration error << core config"));
        assert!(display_output.contains("-> At: src/config.rs:42"));
        assert!(display_output.contains("database_config"));
        assert!(display_output.contains("-> Info: missing db config"));
        assert!(display_output.contains("database_config"));
        assert!(display_output.contains("step: initialization"));
        assert!(display_output.contains("resource: database"));
    }

    #[test]
    #[should_panic]
    fn test_error_assertions() {
        let result: Result<(), _> = StructError::from(TestDomainReason::Why1).err();

        // 使用自定义断言trait
        result.assert("This should panic with domain error");
    }
}
