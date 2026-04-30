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
//! │    → cli::print_error(&err)                                 │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! The key boundary:
//!
//! - [`StructError::report()`] gives you a [`DiagnosticReport`] — human diagnostics,
//!   redaction, text rendering. Only requires [`reason::DomainReason`].
//! - [`StructError::exposure_snapshot()`] gives you an [`ErrorProtocolSnapshot`] —
//!   identity + exposure decision + report, the unified protocol input.
//!   Requires [`reason::DomainReason`] + [`reason::ErrorIdentityProvider`].
//!
//! If you only have [`reason::DomainReason`], you can always `report()`. If you
//! also implement [`reason::ErrorIdentityProvider`] (via `#[derive(OrionError)]`),
//! you can use `exposure_snapshot()` and the full protocol projection stack.
//!
//! Module split:
//!
//! - [`report`] is the human-facing diagnostics layer
//! - [`protocol`] is the protocol/exposure projection layer
//!
//! Root-surface guardrails:
//!
//! - derive macros stay importable from the crate root
//! - runtime identity traits live under [`reason`]
//! - removed root trait/type re-exports and old extension modules must not drift back
//!
//! ```compile_fail
//! use orion_error::DomainReason;
//! ```
//!
//! ```compile_fail
//! use orion_error::ErrorCode;
//!
//! trait NeedsRootTrait: ErrorCode {}
//! ```
//!
//! ```compile_fail
//! use orion_error::ErrorIdentityProvider;
//!
//! fn accepts_root_trait<T: ErrorIdentityProvider>(_value: &T) {}
//! ```
//!
//! ```compile_fail
//! use orion_error::bridge::*;
//! ```
//!
//! ```compile_fail
//! use orion_error::testing::*;
//! ```
//!
//! ```compile_fail
//! use orion_error::test_prelude::*;
//! ```
//!
//! ```compile_fail
//! use orion_error::ErrorWith;
//! ```
//!
//! ```compile_fail
//! use orion_error::ErrorWrapAs;
//! ```
//!
//! ```compile_fail
//! use orion_error::IntoAs;
//! ```
//!
//! ```compile_fail
//! use orion_error::{StructError, UvsReason};
//!
//! let _ = StructError::from(UvsReason::system_error())
//!     .attach_source(std::io::Error::other("disk offline"));
//! ```
//!
//! ```compile_fail
//! use orion_error::types::ErrorIdentity;
//! ```
//!
//! ```compile_fail
//! use orion_error::DefaultExposurePolicy;
//! ```
//!
//! ```compile_fail
//! use orion_error::traits_ext::*;
//! ```
//!
//! ```compile_fail
//! use orion_error::{StructError, UvsReason};
//!
//! let report = StructError::from(UvsReason::system_error()).report();
//! let _ = report.projection;
//! ```
//!
//! ```compile_fail
//! use orion_error::report::print_error;
//! ```
//!
//! ```compile_fail
//! use orion_error::{StructError, UvsReason};
//!
//! let report = StructError::from(UvsReason::system_error()).report();
//! let _ = report.path();
//! ```
//!
//! ```compile_fail
//! use orion_error::{StructError, UvsReason};
//!
//! let report = StructError::from(UvsReason::system_error()).report();
//! let _ = report.root_metadata();
//! ```
//!
//! ```compile_fail
//! use orion_error::{StructError, UvsReason};
//!
//! let report = StructError::from(UvsReason::system_error()).report();
//! let _ = report.source_frames();
//! ```
//!
//! ```compile_fail
//! use orion_error::{OperationContext, StructError, UvsReason};
//!
//! let _ = OperationContext::doing("load config").target();
//! let _ = StructError::from(UvsReason::system_error()).target_main();
//! ```
//!
//! ```compile_fail
//! use orion_error::protocol::DefaultExposurePolicy;
//! use orion_error::{StructError, UvsReason};
//!
//! let proto = StructError::from(UvsReason::system_error())
//!     .exposure_snapshot(&DefaultExposurePolicy);
//! let _ = proto.report();
//! ```
//!
//! ```rust
//! use orion_error::OrionError;
//! use orion_error::reason::{ErrorCategory, ErrorCode, ErrorIdentityProvider};
//!
//! #[derive(Debug, Clone, PartialEq, OrionError)]
//! enum DemoReason {
//!     #[orion_error(identity = "logic.demo_reason")]
//!     Demo,
//! }
//!
//! let reason = DemoReason::Demo;
//! assert_eq!(reason.error_code(), 500);
//! assert_eq!(reason.stable_code(), "logic.demo_reason");
//! assert_eq!(reason.error_category(), ErrorCategory::Logic);
//! ```
mod core;
mod testing;
mod traits;

extern crate self as orion_error;

#[cfg(feature = "derive")]
pub use orion_error_derive::{ErrorCode, ErrorIdentityProvider, OrionError};

pub use core::{OperationContext, StructError, UvsReason};

/// Primary-path traits and types for convenient wildcard imports.
///
/// # Example
/// ```rust,ignore
/// use orion_error::prelude::*;
/// ```
pub mod prelude {
    pub use crate::core::StructError;
    pub use crate::traits::{ErrorWith, ErrorWrapAs, IntoAs};
    #[cfg(feature = "derive")]
    pub use crate::OrionError;
}

/// Runtime-layer types.
///
/// These are the primary carriers used while an error is still moving through
/// application code.
pub mod runtime {
    pub use crate::core::{
        ErrorMetadata, MetadataValue, OperationContext, OperationScope, StructError,
        StructErrorBuilder, WithContext,
    };

    /// Source observation models attached to runtime errors.
    ///
    /// Keep source payload inspection under this submodule so the top-level
    /// `runtime::*` surface stays centered on the main carrier and context
    /// APIs.
    pub mod source {
        pub use crate::core::{
            SourceFrame, SourcePayloadKind, SourcePayloadRef,
        };
    }
}

/// Snapshot-layer types and stable snapshot schema exports.
pub mod snapshot {
    pub use crate::core::{
        ErrorIdentity, ErrorSnapshot, SnapshotContextFrame, SnapshotSourceFrame,
        StableErrorSnapshot, STABLE_SNAPSHOT_SCHEMA_VERSION,
    };
}

/// Report-layer types for rendering and redaction.
pub mod report {
    pub use crate::core::{
        DiagnosticReport, RedactPolicy,
    };
}

/// CLI-side output helpers.
pub mod cli {
    pub use crate::core::cli::print_error;
}

/// Standard-error ecosystem interop: bridge types for entering the standard
/// `std::error::Error` ecosystem.
///
/// Provides owned and borrowed wrappers that implement `StdError` and delegate
/// to the underlying [`StructError`]. Use these when you need to pass an
/// orion-error through an interface that expects `dyn Error`.
pub mod interop {
    pub use crate::core::{OwnedDynStdStructError, OwnedStdStructError, StdStructRef};
    pub use crate::traits::{raw_source, RawSource, RawStdError};
}

/// Protocol/exposure-layer types for boundary projections.
pub mod protocol {
    pub use crate::core::{
        DefaultExposurePolicy, ErrorProtocolSnapshot, ExposureDecision, ExposurePolicy, Visibility,
    };
}

/// Reason-layer enums and traits.
pub mod reason {
    pub use crate::core::{
        ConfErrReason, DomainReason, ErrorCategory, ErrorCode, ErrorIdentityProvider,
        UvsReason,
    };
}

/// Conversion traits for the current primary paths.
pub mod conversion {
    pub use crate::traits::{
        ConvStructError, ErrorConv, ErrorWith, ErrorWrapAs, IntoAs, ToStructError,
    };
}

/// Development and validation-only helpers.
pub mod dev {
    /// Test assertion helpers and testing-only utility traits.
    pub mod testing {
        pub use crate::testing::*;
    }

    /// Wildcard imports for tests, schema checks, and migration-oriented validation.
    pub mod prelude {
        pub use crate::core::{
            DiagnosticReport, ErrorIdentity, ErrorProtocolSnapshot, ErrorSnapshot,
            ExposureDecision, ExposurePolicy, RedactPolicy, StableErrorSnapshot, Visibility,
            STABLE_SNAPSHOT_SCHEMA_VERSION,
        };
        #[cfg(feature = "derive")]
        pub use crate::OrionError;
    }
}
