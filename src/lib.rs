//! # orion-error — structured error governance for large Rust codebases
//!
//! ## Decision flow
//!
//! When you have an error, the question is: **what do you need to do with it?**
//!
//! ```text
//! ┌─ I have an error ──────────────────────────────────────────┐
//! │                                                             │
//! │  Need to print it for a human?                              │
//! │    → err.report().render()                                  │
//! │                                                             │
//! │  Need to return it to an HTTP/RPC/CLI boundary?             │
//! │    → err.exposure_snapshot(&policy).to_http_error_json()    │
//! │    → err.exposure_snapshot(&policy).to_rpc_error_json()     │
//! │    → err.exposure_snapshot(&policy).to_cli_error_json()     │
//! │                                                             │
//! │  Need a stable machine-readable snapshot?                   │
//! │    → err.snapshot().stable_export()                         │
//! │                                                             │
//! │  Need to bridge to std::error::Error?                       │
//! │    → err.as_std() / err.into_std() / err.into_dyn_std()    │
//! │                                                             │
//! │  Just need to log and move on?                              │
//! │    → err.display_chain()                                    │
//! │    → report::print_error(&err)                              │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! The key boundary:
//!
//! - [`StructError::report()`] gives you a [`DiagnosticReport`] — human diagnostics,
//!   redaction, text rendering. Only requires [`DomainReason`].
//! - [`StructError::exposure_snapshot()`] gives you an [`ErrorProtocolSnapshot`] —
//!   identity + exposure decision + report, the unified protocol input.
//!   Requires [`DomainReason`] + [`ErrorIdentityProvider`].
//!
//! If you only have [`DomainReason`], you can always `report()`. If you
//! also implement [`ErrorIdentityProvider`] (via `#[derive(OrionError)]`),
//! you can use `exposure_snapshot()` and the full protocol projection stack.
mod core;
pub mod testcase;
mod traits;

extern crate self as orion_error;

#[cfg(feature = "derive")]
pub use orion_error_derive::{ErrorCode, ErrorIdentityProvider, OrionError};

pub use core::{DefaultExposurePolicy, DomainReason, OperationContext, StructError, UvsReason};
pub use traits::{ErrorWith, ErrorWrapAs, IntoAs};

#[doc(hidden)]
pub use core::{
    ErrorCategory, ErrorCode, ErrorIdentityProvider, UvsFrom, Visibility,
};
#[doc(hidden)]
pub use traits::ErrorConv;

/// Primary-path traits and types for convenient wildcard imports.
///
/// # Example
/// ```rust,ignore
/// use orion_error::prelude::*;
/// ```
pub mod prelude {
    pub use crate::core::{DefaultExposurePolicy, StructError};
    pub use crate::traits::{ErrorWith, ErrorWrapAs, IntoAs};
    #[cfg(feature = "derive")]
    pub use crate::OrionError;
}

/// Wildcard imports for protocol/schema checks and migration-oriented tests.
///
/// Prefer [`prelude`] for new application code. Use this module when a test or
/// verification task intentionally needs broad access to projection, snapshot,
/// bridge, and conversion surfaces in one place.
pub mod advanced_prelude {
    pub use crate::core::{
        DefaultExposurePolicy, DiagnosticReport, ErrorCategory, ErrorCode,
        ErrorIdentity, ErrorIdentityProvider, ErrorMetadata,
        ErrorProtocolSnapshot, ErrorSnapshot, ExposureDecision,
        ExposurePolicy, IntoSourcePayload, OwnedDynStdStructError,
        OwnedStdStructError, RedactPolicy, SnapshotContextFrame, SnapshotSourceFrame,
        SourceFrame, SourcePayload, SourcePayloadKind, SourcePayloadRef, StableErrorSnapshot,
        StableSnapshotContextFrame, StableSnapshotSourceFrame, StdStructRef, StructError,
        UvsFrom, UvsReason, Visibility, STABLE_SNAPSHOT_SCHEMA_VERSION,
    };
    pub use crate::traits::{
        raw_source, ConvStructError, ErrorConv, ErrorWith, ErrorWrapAs, IntoAs, RawSource,
        RawStdError, ToStructError, WrapStructErrorAs,
    };
    #[cfg(feature = "derive")]
    pub use crate::OrionError;
}

/// Compatibility wildcard imports for legacy conversion APIs.
///
/// Use this only when maintaining older `owe(...)` call paths.
pub mod compat_prelude {
    #![allow(deprecated)]
    pub use crate::traits::{
        ErrorOwe, ErrorOweBase, ErrorOweSource, ErrorOweSourceBase,
    };
}

/// Shared data carriers and enums that are still convenient outside the
/// layered namespaces.
pub mod types {
    pub use crate::core::{
        ConfErrReason, DiagnosticReport, ErrStrategy, ErrorCategory, ErrorIdentity,
        ErrorIdentityProvider, ErrorMetadata, ErrorProtocolSnapshot, ExposureDecision,
        MetadataValue, OperationContext, OperationScope, StructError, StructErrorBuilder,
        UvsReason, Visibility, WithContext,
    };
}

/// Runtime-layer types.
///
/// These are the primary carriers used while an error is still moving through
/// application code.
pub mod runtime {
    pub use crate::core::{
        ContextRecord, ErrorMetadata, MetadataValue, OperationContext, OperationScope, SourceFrame,
        SourcePayload, SourcePayloadKind, SourcePayloadRef, StructError, StructErrorBuilder,
        WithContext,
    };
}

/// Explicit bridge types for entering the standard error ecosystem.
pub mod bridge {
    pub use crate::core::{
        IntoSourcePayload, OwnedDynStdStructError, OwnedStdStructError, SourcePayload,
        SourcePayloadKind, SourcePayloadRef, StdStructRef,
    };
    pub use crate::traits::{raw_source, RawSource, RawStdError};
}

/// Snapshot-layer types and stable snapshot schema exports.
pub mod snapshot {
    pub use crate::core::{
        ErrorIdentity, ErrorSnapshot, SnapshotContextFrame, SnapshotSourceFrame,
        StableErrorSnapshot, StableSnapshotContextFrame, StableSnapshotSourceFrame,
        STABLE_SNAPSHOT_SCHEMA_VERSION,
    };
}

/// Report-layer types for rendering and redaction.
pub mod report {
    pub use crate::core::{
        DefaultExposurePolicy, DiagnosticReport, ErrorProtocolSnapshot, ExposureDecision,
        ExposurePolicy, RedactPolicy, Visibility,
    };
    pub use crate::core::cli::print_error;
}

/// Reason-layer enums and traits.
pub mod reason {
    pub use crate::core::{
        ConfErrReason, DomainReason, ErrorCategory, ErrorCode, ErrorIdentityProvider, UvsFrom,
        UvsReason,
    };
}

/// Conversion traits for the current primary paths.
pub mod conversion {
    pub use crate::traits::{
        ErrorConv, ErrorWith, ErrorWrapAs, IntoAs, ToStructError, WrapStructErrorAs,
    };
}

/// Advanced conversion helpers that are not part of the default import path.
pub mod conversion_ext {
    pub use crate::traits::ConvStructError;
}

/// Grouped conversion and context extension traits.
pub mod traits_ext {
    pub use crate::runtime::ContextRecord;
    pub use crate::traits::{
        ConvStructError, ErrorConv, ErrorWith, ErrorWrapAs, IntoAs, ToStructError,
        WrapStructErrorAs,
    };
    pub use crate::{ErrorCode, UvsFrom};
}

/// Compatibility trait exports for legacy conversion helpers.
pub mod compat_traits {
    #![allow(deprecated)]
    pub use crate::traits::{
        ErrorOwe, ErrorOweBase, ErrorOweSource, ErrorOweSourceBase,
    };
}
