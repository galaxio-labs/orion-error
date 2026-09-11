/// Legacy numeric error code.
///
/// **The authoritative machine identity is
/// [`ErrorIdentityProvider::stable_code`] + [`ErrorIdentityProvider::error_category`].**
/// The numeric `error_code()` is retained only for backward compatibility with
/// older numeric-code integrations (HTTP status mapping, legacy dashboards) and
/// for the built-in [`crate::UnifiedReason`] helpers.
///
/// # Migration guidance
///
/// - Prefer `#[derive(OrionError)]` with `identity = "biz.xxx"` and rely on
///   `stable_code()` for stable, machine-facing identity.
/// - Do **not** introduce new numeric-code semantics on top of `error_code()`;
///   the default (`500`) is a compatibility fallback, not a classification.
/// - If you must expose a numeric value (e.g. for an HTTP status), derive it from
///   `ErrorCategory` / `ExposurePolicy` rather than from `error_code()`.
pub trait ErrorCode {
    fn error_code(&self) -> i32 {
        500
    }
}

/// Categorisation of an error for protocol-level routing and policy decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum ErrorCategory {
    /// Configuration / environment issue (e.g. missing file, bad config).
    Conf,
    /// Business-logic violation (e.g. validation failure, policy reject).
    Biz,
    /// Internal logic error (e.g. unreachable branch, invariant violation).
    Logic,
    /// System / infrastructure error (e.g. network, disk I/O, upstream timeout).
    Sys,
}

impl ErrorCategory {
    /// Return the stable string code for this error variant.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Conf => "conf",
            Self::Biz => "biz",
            Self::Logic => "logic",
            Self::Sys => "sys",
        }
    }
}

/// Runtime identity provider for stable error codes and categories.
///
/// Implemented automatically by `#[derive(OrionError)]`. Used by
/// [`StructError::exposure`](crate::StructError::exposure)
/// and the protocol projection layer to determine visibility and
/// exposure decisions.
pub trait ErrorIdentityProvider {
    fn stable_code(&self) -> &'static str;

    fn error_category(&self) -> ErrorCategory;
}
