mod core;
mod testcase;
mod traits;

extern crate self as orion_error;

#[cfg(feature = "derive")]
pub use orion_error_derive::{ErrorCode, ErrorIdentityProvider, OrionError};

pub use core::ErrStrategy;
pub use core::{
    print_error, print_error_zh, ConfErrReason, DefaultErrorPolicy, DomainReason, ErrorCategory,
    ErrorCliResponse, ErrorCode, ErrorHttpResponse, ErrorIdentity, ErrorIdentityProvider,
    ErrorLogResponse, ErrorMetadata, ErrorPolicy, ErrorPolicyDecision, ErrorPolicyInput,
    ErrorProtocolSnapshot, ErrorRenderer, ErrorReport, ErrorRpcResponse, ErrorSnapshot,
    IntoSourcePayload, MetadataValue, OwnedDynStdStructError, OwnedStdStructError, RedactPolicy,
    RenderMode, SnapshotContextFrame, SnapshotSourceFrame, SourceFrame, SourcePayload,
    SourcePayloadKind, SourcePayloadRef, StableErrorSnapshot, StableSnapshotContextFrame,
    StableSnapshotSourceFrame, StdStructRef, StructErrorTrait, TextReportRenderer, UvsFrom,
    UvsReason, Visibility, CLI_ERROR_RESPONSE_FIELDS, HTTP_ERROR_RESPONSE_FIELDS,
    LOG_ERROR_RESPONSE_FIELDS, POLICY_DECISION_FIELDS, POLICY_SNAPSHOT_TOP_LEVEL_FIELDS,
    RPC_ERROR_RESPONSE_FIELDS, STABLE_SNAPSHOT_CONTEXT_FIELDS, STABLE_SNAPSHOT_SCHEMA_VERSION,
    STABLE_SNAPSHOT_SOURCE_FRAME_FIELDS, STABLE_SNAPSHOT_TOP_LEVEL_FIELDS,
};
pub use core::{ContextRecord, OperationContext, OperationScope, WithContext};
pub use core::{StructError, StructErrorBuilder};
pub use testcase::{
    assert_err_category, assert_err_code, assert_err_identity, assert_err_operation,
    assert_err_path, TestAssert, TestAssertWithMsg,
};
pub use traits::{
    raw_source, ConvStructError, ErrorConv, ErrorWith, ErrorWrap, ErrorWrapAs, IntoAs, RawSource,
    RawStdError, ToStructError, WrapStructError, WrapStructErrorAs,
};
pub use traits::{ErrorOwe, ErrorOweBase, ErrorOweSource, ErrorOweSourceBase};

/// Primary-path traits and types for convenient wildcard imports.
///
/// # Example
/// ```rust,ignore
/// use orion_error::prelude::*;
/// ```
pub mod prelude {
    #[cfg(feature = "derive")]
    pub use crate::OrionError;
    pub use crate::{DefaultErrorPolicy, ErrorWith, ErrorWrapAs, IntoAs, StructError};
}

/// Full wildcard imports for advanced examples and migration work.
///
/// Prefer [`prelude`] for new application code. Use this module when working on
/// protocol projections, snapshot/schema checks, bridge internals, or broad
/// migration tests.
pub mod full_prelude {
    #[cfg(feature = "derive")]
    pub use crate::OrionError;
    pub use crate::{
        raw_source, ContextRecord, ConvStructError, DefaultErrorPolicy, ErrorCategory,
        ErrorCliResponse, ErrorCode, ErrorConv, ErrorHttpResponse, ErrorIdentity,
        ErrorIdentityProvider, ErrorLogResponse, ErrorMetadata, ErrorPolicy, ErrorPolicyDecision,
        ErrorPolicyInput, ErrorProtocolSnapshot, ErrorRenderer, ErrorReport, ErrorRpcResponse,
        ErrorSnapshot, ErrorWith, ErrorWrapAs, IntoAs, IntoSourcePayload, MetadataValue,
        OperationContext, OperationScope, OwnedDynStdStructError, OwnedStdStructError, RawSource,
        RawStdError, RedactPolicy, RenderMode, SnapshotContextFrame, SnapshotSourceFrame,
        SourceFrame, SourcePayload, SourcePayloadKind, SourcePayloadRef, StableErrorSnapshot,
        StableSnapshotContextFrame, StableSnapshotSourceFrame, StdStructRef, StructError,
        StructErrorBuilder, TextReportRenderer, ToStructError, UvsFrom, UvsReason, Visibility,
        WrapStructErrorAs, CLI_ERROR_RESPONSE_FIELDS, HTTP_ERROR_RESPONSE_FIELDS,
        LOG_ERROR_RESPONSE_FIELDS, POLICY_DECISION_FIELDS, POLICY_SNAPSHOT_TOP_LEVEL_FIELDS,
        RPC_ERROR_RESPONSE_FIELDS, STABLE_SNAPSHOT_CONTEXT_FIELDS, STABLE_SNAPSHOT_SCHEMA_VERSION,
        STABLE_SNAPSHOT_SOURCE_FRAME_FIELDS, STABLE_SNAPSHOT_TOP_LEVEL_FIELDS,
    };
}

/// Compatibility wildcard imports for legacy conversion APIs.
///
/// Use this only when maintaining older `owe(...)` call paths.
pub mod compat_prelude {
    pub use crate::{
        ErrorOwe, ErrorOweBase, ErrorOweSource, ErrorOweSourceBase, ErrorWrap, WrapStructError,
    };
}

/// Grouped core types and enums.
pub mod types {
    pub use crate::{
        ConfErrReason, DefaultErrorPolicy, ErrStrategy, ErrorCategory, ErrorCliResponse,
        ErrorHttpResponse, ErrorIdentity, ErrorIdentityProvider, ErrorLogResponse, ErrorMetadata,
        ErrorPolicy, ErrorPolicyDecision, ErrorPolicyInput, ErrorProtocolSnapshot, ErrorRenderer,
        ErrorReport, ErrorRpcResponse, ErrorSnapshot, IntoSourcePayload, MetadataValue,
        OperationContext, OperationScope, OwnedDynStdStructError, OwnedStdStructError,
        RedactPolicy, RenderMode, SnapshotContextFrame, SnapshotSourceFrame, SourceFrame,
        SourcePayload, SourcePayloadKind, SourcePayloadRef, StableErrorSnapshot,
        StableSnapshotContextFrame, StableSnapshotSourceFrame, StdStructRef, StructError,
        StructErrorBuilder, TextReportRenderer, UvsReason, Visibility, WithContext,
        CLI_ERROR_RESPONSE_FIELDS, HTTP_ERROR_RESPONSE_FIELDS, LOG_ERROR_RESPONSE_FIELDS,
        POLICY_DECISION_FIELDS, POLICY_SNAPSHOT_TOP_LEVEL_FIELDS, RPC_ERROR_RESPONSE_FIELDS,
        STABLE_SNAPSHOT_CONTEXT_FIELDS, STABLE_SNAPSHOT_SCHEMA_VERSION,
        STABLE_SNAPSHOT_SOURCE_FRAME_FIELDS, STABLE_SNAPSHOT_TOP_LEVEL_FIELDS,
    };
}

/// Runtime-layer types.
///
/// These are the primary carriers used while an error is still moving through
/// application code.
pub mod runtime {
    pub use crate::{
        ContextRecord, ErrorMetadata, MetadataValue, OperationContext, OperationScope, SourceFrame,
        SourcePayload, SourcePayloadKind, SourcePayloadRef, StructError, StructErrorBuilder,
        WithContext,
    };
}

/// Explicit bridge types for entering the standard error ecosystem.
pub mod bridge {
    pub use crate::{
        raw_source, IntoSourcePayload, OwnedDynStdStructError, OwnedStdStructError, RawSource,
        RawStdError, SourcePayload, SourcePayloadKind, SourcePayloadRef, StdStructRef,
    };
}

/// Snapshot-layer types and stable snapshot schema exports.
pub mod snapshot {
    pub use crate::{
        ErrorIdentity, ErrorSnapshot, SnapshotContextFrame, SnapshotSourceFrame,
        StableErrorSnapshot, StableSnapshotContextFrame, StableSnapshotSourceFrame,
        STABLE_SNAPSHOT_CONTEXT_FIELDS, STABLE_SNAPSHOT_SCHEMA_VERSION,
        STABLE_SNAPSHOT_SOURCE_FRAME_FIELDS, STABLE_SNAPSHOT_TOP_LEVEL_FIELDS,
    };
}

/// Report-layer types for rendering and redaction.
pub mod report {
    pub use crate::{
        DefaultErrorPolicy, ErrorCliResponse, ErrorHttpResponse, ErrorLogResponse, ErrorPolicy,
        ErrorPolicyDecision, ErrorPolicyInput, ErrorProtocolSnapshot, ErrorRenderer, ErrorReport,
        ErrorRpcResponse, RedactPolicy, RenderMode, TextReportRenderer, Visibility,
        CLI_ERROR_RESPONSE_FIELDS, HTTP_ERROR_RESPONSE_FIELDS, LOG_ERROR_RESPONSE_FIELDS,
        POLICY_DECISION_FIELDS, POLICY_SNAPSHOT_TOP_LEVEL_FIELDS, RPC_ERROR_RESPONSE_FIELDS,
    };
}

/// Reason-layer enums and traits.
pub mod reason {
    pub use crate::{
        ConfErrReason, ErrorCategory, ErrorCode, ErrorIdentityProvider, UvsFrom, UvsReason,
    };
}

/// Conversion traits for the current primary paths.
pub mod conversion {
    pub use crate::{
        ConvStructError, ErrorConv, ErrorWith, ErrorWrap, ErrorWrapAs, IntoAs, ToStructError,
        WrapStructError, WrapStructErrorAs,
    };
}

/// Grouped conversion and context extension traits.
pub mod traits_ext {
    pub use crate::{
        ContextRecord, ConvStructError, ErrorCode, ErrorConv, ErrorWith, ErrorWrapAs, IntoAs,
        ToStructError, UvsFrom, WrapStructErrorAs,
    };
}

/// Compatibility trait exports for legacy conversion helpers.
pub mod compat_traits {
    pub use crate::{
        ErrorOwe, ErrorOweBase, ErrorOweSource, ErrorOweSourceBase, ErrorWrap, WrapStructError,
    };
}
