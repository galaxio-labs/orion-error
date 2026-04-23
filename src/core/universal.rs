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
pub enum UvsReason {
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

impl DomainReason for UvsReason {}

impl UvsReason {
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

/// Unified constructor helpers for types that can be converted from `UvsReason`.
pub trait UvsFrom: From<UvsReason> + Sized {
    fn from_conf() -> Self {
        Self::from(UvsReason::core_conf())
    }

    fn from_conf_reason(reason: ConfErrReason) -> Self {
        Self::from(UvsReason::ConfigError(reason))
    }

    fn from_data() -> Self {
        Self::from(UvsReason::data_error())
    }

    fn from_sys() -> Self {
        Self::from(UvsReason::system_error())
    }

    fn from_biz() -> Self {
        Self::from(UvsReason::business_error())
    }

    fn from_logic() -> Self {
        Self::from(UvsReason::logic_error())
    }

    fn from_rule() -> Self {
        Self::from(UvsReason::rule_error())
    }

    fn from_res() -> Self {
        Self::from(UvsReason::resource_error())
    }

    fn from_net() -> Self {
        Self::from(UvsReason::network_error())
    }

    fn from_timeout() -> Self {
        Self::from(UvsReason::timeout_error())
    }

    fn from_validation() -> Self {
        Self::from(UvsReason::validation_error())
    }

    fn from_not_found() -> Self {
        Self::from(UvsReason::not_found_error())
    }

    fn from_permission() -> Self {
        Self::from(UvsReason::permission_error())
    }

    fn from_external() -> Self {
        Self::from(UvsReason::external_error())
    }
}

impl<T> UvsFrom for T where T: From<UvsReason> {}

impl ErrorCode for UvsReason {
    fn error_code(&self) -> i32 {
        match self {
            // === Business Layer Errors (100-199) ===
            UvsReason::ValidationError => 100,
            UvsReason::BusinessError => 101,
            UvsReason::NotFoundError => 102,
            UvsReason::PermissionError => 103,
            UvsReason::LogicError => 104,
            UvsReason::RunRuleError => 105,

            // === Infrastructure Layer Errors (200-299) ===
            UvsReason::DataError => 200,
            UvsReason::SystemError => 201,
            UvsReason::NetworkError => 202,
            UvsReason::ResourceError => 203,
            UvsReason::TimeoutError => 204,

            // === Configuration & External Layer Errors (300-399) ===
            UvsReason::ConfigError(_) => 300,
            UvsReason::ExternalError => 301,
        }
    }
}

impl ErrorIdentityProvider for UvsReason {
    fn stable_code(&self) -> &'static str {
        match self {
            UvsReason::ValidationError => "biz.validation_error",
            UvsReason::BusinessError => "biz.business_error",
            UvsReason::RunRuleError => "biz.run_rule_error",
            UvsReason::NotFoundError => "biz.not_found",
            UvsReason::PermissionError => "biz.permission_denied",
            UvsReason::DataError => "sys.data_error",
            UvsReason::SystemError => "sys.io_error",
            UvsReason::NetworkError => "sys.network_error",
            UvsReason::ResourceError => "sys.resource_exhausted",
            UvsReason::TimeoutError => "sys.timeout",
            UvsReason::ConfigError(ConfErrReason::Core) => "conf.core_invalid",
            UvsReason::ConfigError(ConfErrReason::Feature) => "conf.feature_invalid",
            UvsReason::ConfigError(ConfErrReason::Dynamic) => "conf.dynamic_invalid",
            UvsReason::ExternalError => "sys.external_service_error",
            UvsReason::LogicError => "logic.internal_invariant_broken",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        match self {
            UvsReason::ConfigError(_) => ErrorCategory::Conf,
            UvsReason::LogicError => ErrorCategory::Logic,
            UvsReason::ValidationError
            | UvsReason::BusinessError
            | UvsReason::RunRuleError
            | UvsReason::NotFoundError
            | UvsReason::PermissionError => ErrorCategory::Biz,
            UvsReason::DataError
            | UvsReason::SystemError
            | UvsReason::NetworkError
            | UvsReason::ResourceError
            | UvsReason::TimeoutError
            | UvsReason::ExternalError => ErrorCategory::Sys,
        }
    }
}

impl UvsReason {
    /// Check if this error is retryable
    /// 检查错误是否可重试
    pub fn is_retryable(&self) -> bool {
        match self {
            // Infrastructure errors are often retryable
            UvsReason::NetworkError => true,
            UvsReason::TimeoutError => true,
            UvsReason::ResourceError => true,
            UvsReason::SystemError => true,
            UvsReason::ExternalError => true,

            // Business logic errors are generally not retryable
            UvsReason::ValidationError => false,
            UvsReason::BusinessError => false,
            UvsReason::RunRuleError => false,
            UvsReason::NotFoundError => false,
            UvsReason::PermissionError => false,

            // Configuration errors require manual intervention
            UvsReason::ConfigError(_) => false,
            UvsReason::DataError => false,
            UvsReason::LogicError => false,
        }
    }

    /// Check if this error should be logged with high severity
    /// 检查错误是否需要高优先级记录
    pub fn is_high_severity(&self) -> bool {
        match self {
            // System and infrastructure issues are high severity
            UvsReason::SystemError => true,
            UvsReason::ResourceError => true,
            UvsReason::ConfigError(_) => true,

            // Others are normal business operations
            _ => false,
        }
    }

    /// Get error category name for monitoring and metrics
    /// 获取错误类别名称用于监控和指标
    pub fn category_name(&self) -> &'static str {
        match self {
            UvsReason::ValidationError => "validation",
            UvsReason::BusinessError => "business",
            UvsReason::RunRuleError => "runrule",
            UvsReason::NotFoundError => "not_found",
            UvsReason::PermissionError => "permission",
            UvsReason::DataError => "data",
            UvsReason::SystemError => "system",
            UvsReason::NetworkError => "network",
            UvsReason::ResourceError => "resource",
            UvsReason::TimeoutError => "timeout",
            UvsReason::ConfigError(_) => "config",
            UvsReason::ExternalError => "external",
            UvsReason::LogicError => "logic",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_ranges() {
        // Business layer (100-199)
        assert_eq!(UvsReason::validation_error().error_code(), 100);
        assert_eq!(UvsReason::business_error().error_code(), 101);
        assert_eq!(UvsReason::not_found_error().error_code(), 102);
        assert_eq!(UvsReason::permission_error().error_code(), 103);

        // Infrastructure layer (200-299)
        assert_eq!(UvsReason::data_error().error_code(), 200);
        assert_eq!(UvsReason::system_error().error_code(), 201);
        assert_eq!(UvsReason::network_error().error_code(), 202);
        assert_eq!(UvsReason::resource_error().error_code(), 203);
        assert_eq!(UvsReason::timeout_error().error_code(), 204);

        // Configuration & external layer (300-399)
        assert_eq!(UvsReason::core_conf().error_code(), 300);
        assert_eq!(UvsReason::external_error().error_code(), 301);
    }

    #[test]
    fn test_retryable_errors() {
        assert!(UvsReason::network_error().is_retryable());
        assert!(UvsReason::timeout_error().is_retryable());
        assert!(!UvsReason::validation_error().is_retryable());
        assert!(!UvsReason::business_error().is_retryable());
    }

    #[test]
    fn test_high_severity_errors() {
        assert!(UvsReason::system_error().is_high_severity());
        assert!(UvsReason::resource_error().is_high_severity());
        assert!(!UvsReason::validation_error().is_high_severity());
        assert!(!UvsReason::NotFoundError.is_high_severity());
    }

    #[test]
    fn test_category_names() {
        assert_eq!(UvsReason::network_error().category_name(), "network");
        assert_eq!(UvsReason::business_error().category_name(), "business");
        assert_eq!(UvsReason::core_conf().category_name(), "config");
    }

    #[test]
    fn test_stable_code_values() {
        assert_eq!(
            UvsReason::validation_error().stable_code(),
            "biz.validation_error"
        );
        assert_eq!(UvsReason::system_error().stable_code(), "sys.io_error");
        assert_eq!(UvsReason::core_conf().stable_code(), "conf.core_invalid");
        assert_eq!(
            UvsReason::logic_error().stable_code(),
            "logic.internal_invariant_broken"
        );
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(
            UvsReason::validation_error().error_category(),
            ErrorCategory::Biz
        );
        assert_eq!(
            UvsReason::system_error().error_category(),
            ErrorCategory::Sys
        );
        assert_eq!(UvsReason::core_conf().error_category(), ErrorCategory::Conf);
        assert_eq!(
            UvsReason::logic_error().error_category(),
            ErrorCategory::Logic
        );
        assert_eq!(ErrorCategory::Biz.as_str(), "biz");
    }

    #[test]
    fn test_trait_implementations() {
        let reason: UvsReason = <UvsReason as UvsFrom>::from_net();
        assert_eq!(reason.error_code(), 202);

        let reason: UvsReason = <UvsReason as UvsFrom>::from_validation();
        assert_eq!(reason.error_code(), 100);

        let reason: UvsReason = <UvsReason as UvsFrom>::from_external();
        assert_eq!(reason.error_code(), 301);
        assert_eq!(reason.error_category(), ErrorCategory::Sys);
    }
}
