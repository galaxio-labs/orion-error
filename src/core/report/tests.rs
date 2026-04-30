    use crate::{
        core::DomainReason,
        protocol::DefaultExposurePolicy,
        core::{
            ErrorIdentity, ErrorMetadata, ErrorSnapshot, SnapshotContextFrame, SnapshotSourceFrame,
            SourceFrame,
        },
        OperationContext, StructError, UvsReason,
    };
    use crate::reason::{ErrorCategory, ErrorCode, ErrorIdentityProvider};

    use super::{
        DiagnosticReport, ErrorProtocolSnapshot, ExposureDecision, RedactPolicy,
        ReportProjectionParts, Visibility,
    };
    #[derive(Debug)]
    struct TestPolicy;

    impl RedactPolicy for TestPolicy {
        fn redact_key(&self, key: &str) -> bool {
            matches!(key, "token" | "password" | "config.secret")
        }

        fn redact_value(&self, _key: Option<&str>, _value: &str) -> Option<String> {
            Some("<redacted>".to_string())
        }
    }

    #[derive(Debug, Clone, PartialEq, thiserror::Error)]
    enum TestReason {
        #[error("test error")]
        TestError,
        #[error("{0}")]
        Uvs(UvsReason),
    }

    impl From<UvsReason> for TestReason {
        fn from(value: UvsReason) -> Self {
            Self::Uvs(value)
        }
    }

    impl DomainReason for TestReason {}

    impl ErrorCode for TestReason {
        fn error_code(&self) -> i32 {
            match self {
                TestReason::TestError => 1001,
                TestReason::Uvs(reason) => reason.error_code(),
            }
        }
    }

    impl ErrorIdentityProvider for TestReason {
        fn stable_code(&self) -> &'static str {
            match self {
                TestReason::TestError => "test.test_error",
                TestReason::Uvs(reason) => reason.stable_code(),
            }
        }

        fn error_category(&self) -> ErrorCategory {
            match self {
                TestReason::TestError => ErrorCategory::Logic,
                TestReason::Uvs(reason) => reason.error_category(),
            }
        }
    }

    fn test_identity(
        code: &str,
        category: ErrorCategory,
        reason: &str,
        detail: Option<&str>,
        path: Option<&str>,
    ) -> ErrorIdentity {
        ErrorIdentity {
            code: code.to_string(),
            category,
            reason: reason.to_string(),
            detail: detail.map(str::to_string),
            position: None,
            path: path.map(str::to_string),
        }
    }

    fn test_proto(
        report: DiagnosticReport,
        projection: ReportProjectionParts,
        identity: ErrorIdentity,
        decision: ExposureDecision,
    ) -> ErrorProtocolSnapshot {
        ErrorProtocolSnapshot {
            identity,
            decision,
            report,
            projection,
        }
    }
include!("report_cases_tests.rs");
include!("protocol_json_tests.rs");
include!("debug_view_tests.rs");
include!("redaction_tests.rs");
