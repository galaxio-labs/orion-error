use thiserror::Error;

use super::{DomainReason, ErrorCategory, ErrorCode, ErrorIdentityProvider};

/// Configuration error sub-classification
/// 配置错误子分类
#[derive(Debug, Error, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ConfErrReason {
    #[error("core config")]
    Core,
    #[error("feature config error")]
    Feature,
    #[error("dynamic config error")]
    Dynamic,
}

/// Universal error reason classification with clear hierarchical structure
/// 统一错误原因分类 - 采用清晰的分层结构
///
/// # Error Code Ranges
/// - 100-199: Business Layer Errors (业务层错误)
/// - 200-299: Infrastructure Layer Errors (基础设施层错误)
/// - 300-399: Configuration & External Layer Errors (配置和外部层错误)
#[derive(Debug, Error, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum UnifiedReason {
    // === Business Layer Errors (100-199) ===
    /// Input validation errors (格式错误、参数校验失败等)
    #[error("validation error")]
    ValidationError,

    /// Business logic rule violations (业务规则违反、状态冲突等)
    #[error("business logic error")]
    BusinessError,

    /// Business logic rule violations (业务规则违反、状态冲突等)
    #[error("run rule error")]
    RunRuleError,

    /// Resource not found (查询的资源不存在)
    #[error("not found error")]
    NotFoundError,

    /// Permission and authorization errors (权限不足、认证失败)
    #[error("permission error")]
    PermissionError,

    // === Infrastructure Layer Errors (200-299) ===
    /// Database and data processing errors (数据库操作、数据格式错误)
    #[error("data error")]
    DataError,

    /// File system and OS-level errors (文件系统、操作系统错误)
    #[error("system error")]
    SystemError,

    /// Network connectivity and protocol errors (网络连接、HTTP请求错误)
    #[error("network error")]
    NetworkError,

    /// Resource exhaustion (内存不足、磁盘空间不足等)
    #[error("resource error")]
    ResourceError,

    /// Operation timeouts (操作超时)
    #[error("timeout error")]
    TimeoutError,

    // === Configuration & External Layer Errors (300-399) ===
    /// Configuration-related errors (配置相关错误)
    #[error("configuration error << {0}")]
    ConfigError(ConfErrReason),

    /// Third-party service errors (第三方服务错误)
    #[error("external service error")]
    ExternalError,

    /// Third-party service errors (第三方服务错误)
    #[error("BUG :logic error")]
    LogicError,
}

impl DomainReason for UnifiedReason {}

impl UnifiedReason {
    // === Configuration Error Constructors ===
    pub fn core_conf() -> Self {
        Self::ConfigError(ConfErrReason::Core)
    }

    pub fn feature_conf() -> Self {
        Self::ConfigError(ConfErrReason::Feature)
    }

    pub fn dynamic_conf() -> Self {
        Self::ConfigError(ConfErrReason::Dynamic)
    }

    // === Business Layer Constructors ===
    pub fn validation_error() -> Self {
        Self::ValidationError
    }

    pub fn business_error() -> Self {
        Self::BusinessError
    }

    pub fn rule_error() -> Self {
        Self::RunRuleError
    }

    pub fn not_found_error() -> Self {
        Self::NotFoundError
    }

    pub fn permission_error() -> Self {
        Self::PermissionError
    }

    // === Infrastructure Layer Constructors ===
    pub fn data_error() -> Self {
        Self::DataError
    }

    pub fn system_error() -> Self {
        Self::SystemError
    }

    pub fn network_error() -> Self {
        Self::NetworkError
    }

    pub fn resource_error() -> Self {
        Self::ResourceError
    }

    pub fn timeout_error() -> Self {
        Self::TimeoutError
    }

    // === External Layer Constructors ===
    pub fn external_error() -> Self {
        Self::ExternalError
    }

    pub fn logic_error() -> Self {
        Self::LogicError
    }
}

impl ErrorCode for UnifiedReason {
    fn error_code(&self) -> i32 {
        match self {
            // === Business Layer Errors (100-199) ===
            UnifiedReason::ValidationError => 100,
            UnifiedReason::BusinessError => 101,
            UnifiedReason::NotFoundError => 102,
            UnifiedReason::PermissionError => 103,
            UnifiedReason::LogicError => 104,
            UnifiedReason::RunRuleError => 105,

            // === Infrastructure Layer Errors (200-299) ===
            UnifiedReason::DataError => 200,
            UnifiedReason::SystemError => 201,
            UnifiedReason::NetworkError => 202,
            UnifiedReason::ResourceError => 203,
            UnifiedReason::TimeoutError => 204,

            // === Configuration & External Layer Errors (300-399) ===
            UnifiedReason::ConfigError(_) => 300,
            UnifiedReason::ExternalError => 301,
        }
    }
}

impl ErrorIdentityProvider for UnifiedReason {
    fn stable_code(&self) -> &'static str {
        match self {
            UnifiedReason::ValidationError => "biz.validation_error",
            UnifiedReason::BusinessError => "biz.business_error",
            UnifiedReason::RunRuleError => "biz.run_rule_error",
            UnifiedReason::NotFoundError => "biz.not_found",
            UnifiedReason::PermissionError => "biz.permission_denied",
            UnifiedReason::DataError => "sys.data_error",
            UnifiedReason::SystemError => "sys.io_error",
            UnifiedReason::NetworkError => "sys.network_error",
            UnifiedReason::ResourceError => "sys.resource_exhausted",
            UnifiedReason::TimeoutError => "sys.timeout",
            UnifiedReason::ConfigError(ConfErrReason::Core) => "conf.core_invalid",
            UnifiedReason::ConfigError(ConfErrReason::Feature) => "conf.feature_invalid",
            UnifiedReason::ConfigError(ConfErrReason::Dynamic) => "conf.dynamic_invalid",
            UnifiedReason::ExternalError => "sys.external_service_error",
            UnifiedReason::LogicError => "logic.internal_invariant_broken",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        match self {
            UnifiedReason::ConfigError(_) => ErrorCategory::Conf,
            UnifiedReason::LogicError => ErrorCategory::Logic,
            UnifiedReason::ValidationError
            | UnifiedReason::BusinessError
            | UnifiedReason::RunRuleError
            | UnifiedReason::NotFoundError
            | UnifiedReason::PermissionError => ErrorCategory::Biz,
            UnifiedReason::DataError
            | UnifiedReason::SystemError
            | UnifiedReason::NetworkError
            | UnifiedReason::ResourceError
            | UnifiedReason::TimeoutError
            | UnifiedReason::ExternalError => ErrorCategory::Sys,
        }
    }
}

impl UnifiedReason {
    /// Check if this error is retryable
    /// 检查错误是否可重试
    pub fn is_retryable(&self) -> bool {
        match self {
            // Infrastructure errors are often retryable
            UnifiedReason::NetworkError => true,
            UnifiedReason::TimeoutError => true,
            UnifiedReason::ResourceError => true,
            UnifiedReason::SystemError => true,
            UnifiedReason::ExternalError => true,

            // Business logic errors are generally not retryable
            UnifiedReason::ValidationError => false,
            UnifiedReason::BusinessError => false,
            UnifiedReason::RunRuleError => false,
            UnifiedReason::NotFoundError => false,
            UnifiedReason::PermissionError => false,

            // Configuration errors require manual intervention
            UnifiedReason::ConfigError(_) => false,
            UnifiedReason::DataError => false,
            UnifiedReason::LogicError => false,
        }
    }

    /// Check if this error should be logged with high severity
    /// 检查错误是否需要高优先级记录
    pub fn is_high_severity(&self) -> bool {
        match self {
            // System and infrastructure issues are high severity
            UnifiedReason::SystemError => true,
            UnifiedReason::ResourceError => true,
            UnifiedReason::ConfigError(_) => true,

            // Others are normal business operations
            _ => false,
        }
    }

    /// Get error category name for monitoring and metrics
    /// 获取错误类别名称用于监控和指标
    pub fn category_name(&self) -> &'static str {
        match self {
            UnifiedReason::ValidationError => "validation",
            UnifiedReason::BusinessError => "business",
            UnifiedReason::RunRuleError => "runrule",
            UnifiedReason::NotFoundError => "not_found",
            UnifiedReason::PermissionError => "permission",
            UnifiedReason::DataError => "data",
            UnifiedReason::SystemError => "system",
            UnifiedReason::NetworkError => "network",
            UnifiedReason::ResourceError => "resource",
            UnifiedReason::TimeoutError => "timeout",
            UnifiedReason::ConfigError(_) => "config",
            UnifiedReason::ExternalError => "external",
            UnifiedReason::LogicError => "logic",
        }
    }
}

/// Deprecated: use [`UnifiedReason`] instead.
#[deprecated(since = "0.9.0", note = "renamed to UnifiedReason")]
pub type UvsReason = UnifiedReason;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_ranges() {
        // Business layer (100-199)
        assert_eq!(UnifiedReason::validation_error().error_code(), 100);
        assert_eq!(UnifiedReason::business_error().error_code(), 101);
        assert_eq!(UnifiedReason::not_found_error().error_code(), 102);
        assert_eq!(UnifiedReason::permission_error().error_code(), 103);

        // Infrastructure layer (200-299)
        assert_eq!(UnifiedReason::data_error().error_code(), 200);
        assert_eq!(UnifiedReason::system_error().error_code(), 201);
        assert_eq!(UnifiedReason::network_error().error_code(), 202);
        assert_eq!(UnifiedReason::resource_error().error_code(), 203);
        assert_eq!(UnifiedReason::timeout_error().error_code(), 204);

        // Configuration & external layer (300-399)
        assert_eq!(UnifiedReason::core_conf().error_code(), 300);
        assert_eq!(UnifiedReason::external_error().error_code(), 301);
    }

    #[test]
    fn test_retryable_errors() {
        assert!(UnifiedReason::network_error().is_retryable());
        assert!(UnifiedReason::timeout_error().is_retryable());
        assert!(!UnifiedReason::validation_error().is_retryable());
        assert!(!UnifiedReason::business_error().is_retryable());
    }

    #[test]
    fn test_high_severity_errors() {
        assert!(UnifiedReason::system_error().is_high_severity());
        assert!(UnifiedReason::resource_error().is_high_severity());
        assert!(!UnifiedReason::validation_error().is_high_severity());
        assert!(!UnifiedReason::NotFoundError.is_high_severity());
    }

    #[test]
    fn test_category_names() {
        assert_eq!(UnifiedReason::network_error().category_name(), "network");
        assert_eq!(UnifiedReason::business_error().category_name(), "business");
        assert_eq!(UnifiedReason::core_conf().category_name(), "config");
    }

    #[test]
    fn test_stable_code_values() {
        assert_eq!(
            UnifiedReason::validation_error().stable_code(),
            "biz.validation_error"
        );
        assert_eq!(UnifiedReason::system_error().stable_code(), "sys.io_error");
        assert_eq!(
            UnifiedReason::core_conf().stable_code(),
            "conf.core_invalid"
        );
        assert_eq!(
            UnifiedReason::logic_error().stable_code(),
            "logic.internal_invariant_broken"
        );
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(
            UnifiedReason::validation_error().error_category(),
            ErrorCategory::Biz
        );
        assert_eq!(
            UnifiedReason::system_error().error_category(),
            ErrorCategory::Sys
        );
        assert_eq!(
            UnifiedReason::core_conf().error_category(),
            ErrorCategory::Conf
        );
        assert_eq!(
            UnifiedReason::logic_error().error_category(),
            ErrorCategory::Logic
        );
        assert_eq!(ErrorCategory::Biz.as_str(), "biz");
    }

    #[test]
    fn test_trait_implementations() {
        let reason = UnifiedReason::network_error();
        assert_eq!(reason.error_code(), 202);

        let reason = UnifiedReason::validation_error();
        assert_eq!(reason.error_code(), 100);

        let reason = UnifiedReason::external_error();
        assert_eq!(reason.error_code(), 301);
        assert_eq!(reason.error_category(), ErrorCategory::Sys);
    }
}
