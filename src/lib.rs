mod core;
mod testcase;
mod traits;

pub use core::ErrStrategy;
pub use core::{
    print_error, print_error_zh, ConfErrReason, DomainReason, ErrorCode, ErrorMetadata,
    ErrorReport, IntoSourcePayload, MetadataValue, OwnedDynStdStructError, OwnedStdStructError,
    RedactPolicy, RenderMode, SnapshotContextFrame, SnapshotSourceFrame, SourceFrame,
    SourcePayload, SourcePayloadKind, SourcePayloadRef, StableSnapshotContextFrame,
    StableSnapshotSourceFrame, StableStructErrorSnapshot, StdStructRef, StructErrorSnapshot,
    StructErrorTrait, UvsFrom, UvsReason, STABLE_SNAPSHOT_CONTEXT_FIELDS,
    STABLE_SNAPSHOT_SCHEMA_VERSION, STABLE_SNAPSHOT_SOURCE_FRAME_FIELDS,
    STABLE_SNAPSHOT_TOP_LEVEL_FIELDS,
};
pub use core::{ContextRecord, OperationContext, OperationScope, WithContext};
pub use core::{SnapshotCompat, StableSnapshotCompat};
pub use core::{StructError, StructErrorBuilder};
pub use testcase::{TestAssert, TestAssertWithMsg};
#[deprecated(
    since = "0.7.0",
    note = "use orion_error::compat_prelude::* or orion_error::compat_traits::* for legacy owe(...) APIs"
)]
pub use traits::ErrorOweBase;
pub use traits::{
    raw_source, ConvStructError, ErrorConv, ErrorWith, ErrorWrapAs, IntoAs, RawSource, RawStdError,
    ToStructError, WrapStructErrorAs,
};

/// V1 primary-path traits and types for convenient wildcard imports.
///
/// # Example
/// ```rust,ignore
/// use orion_error::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        raw_source, ErrorMetadata, ErrorReport, IntoSourcePayload, MetadataValue, OperationContext,
        OperationScope, OwnedDynStdStructError, OwnedStdStructError, RawSource, RawStdError,
        RedactPolicy, RenderMode, SnapshotContextFrame, SnapshotSourceFrame, SourceFrame,
        SourcePayload, SourcePayloadKind, SourcePayloadRef, StableSnapshotContextFrame,
        StableSnapshotSourceFrame, StableStructErrorSnapshot, StdStructRef, StructError,
        StructErrorBuilder, StructErrorSnapshot, UvsReason, STABLE_SNAPSHOT_CONTEXT_FIELDS,
        STABLE_SNAPSHOT_SCHEMA_VERSION, STABLE_SNAPSHOT_SOURCE_FRAME_FIELDS,
        STABLE_SNAPSHOT_TOP_LEVEL_FIELDS,
    };
    pub use crate::{
        ContextRecord, ErrorCode, ErrorConv, ErrorWith, ErrorWrapAs, IntoAs, ToStructError,
        UvsFrom, WrapStructErrorAs,
    };
}

/// V1 namespace.
///
/// This keeps the historical V1 import surfaces available under an explicit
/// versioned root while root-level `prelude` / `traits_ext` / `compat_*`
/// remain for compatibility.
pub mod v1 {
    /// V1 primary-path wildcard import.
    ///
    /// # Example
    /// ```rust,ignore
    /// use orion_error::v1::prelude::*;
    /// ```
    pub mod prelude {
        pub use crate::prelude::*;
    }

    /// V1 trait-group wildcard import.
    pub mod traits_ext {
        pub use crate::traits_ext::*;
    }

    /// V1 compatibility wildcard import for legacy conversion helpers.
    pub mod compat_prelude {
        pub use crate::compat_prelude::*;
    }

    /// V1 compatibility trait export group.
    pub mod compat_traits {
        pub use crate::compat_traits::*;
    }
}

/// V2 layered namespace.
///
/// This keeps runtime / conversion / reason / snapshot / report / bridge
/// imports grouped under a single root without flattening them back into one
/// wildcard surface.
pub mod v2 {
    pub use crate::{bridge, conversion, reason, report, runtime, snapshot};

    /// V2 convenience wildcard import.
    ///
    /// # Example
    /// ```rust,ignore
    /// use orion_error::v2::prelude::*;
    /// ```
    pub mod prelude {
        pub use crate::bridge::*;
        pub use crate::conversion::*;
        pub use crate::reason::*;
        pub use crate::report::*;
        pub use crate::runtime::*;
        pub use crate::snapshot::*;
    }
}

/// Compatibility wildcard imports for legacy conversion APIs.
///
/// Use this only when maintaining older `owe(...)` call paths.
pub mod compat_prelude {
    pub use crate::traits::ErrorOweBase;
    pub use crate::{SnapshotCompat, StableSnapshotCompat};
}

/// Grouped core types and enums.
pub mod types {
    pub use crate::{
        ConfErrReason, ErrStrategy, ErrorMetadata, ErrorReport, IntoSourcePayload, MetadataValue,
        OperationContext, OperationScope, OwnedDynStdStructError, OwnedStdStructError,
        RedactPolicy, RenderMode, SnapshotContextFrame, SnapshotSourceFrame, SourceFrame,
        SourcePayload, SourcePayloadKind, SourcePayloadRef, StableSnapshotContextFrame,
        StableSnapshotSourceFrame, StableStructErrorSnapshot, StdStructRef, StructError,
        StructErrorBuilder, StructErrorSnapshot, UvsReason, WithContext,
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
        SnapshotContextFrame, SnapshotSourceFrame, StableSnapshotContextFrame,
        StableSnapshotSourceFrame, StableStructErrorSnapshot, StructErrorSnapshot,
        STABLE_SNAPSHOT_CONTEXT_FIELDS, STABLE_SNAPSHOT_SCHEMA_VERSION,
        STABLE_SNAPSHOT_SOURCE_FRAME_FIELDS, STABLE_SNAPSHOT_TOP_LEVEL_FIELDS,
    };
}

/// Report-layer types for rendering and redaction.
pub mod report {
    pub use crate::{ErrorReport, RedactPolicy, RenderMode};
}

/// Reason-layer enums and traits.
pub mod reason {
    pub use crate::{ConfErrReason, ErrorCode, UvsFrom, UvsReason};
}

/// Conversion traits for the current primary paths.
pub mod conversion {
    pub use crate::{
        ConvStructError, ErrorConv, ErrorWith, ErrorWrapAs, IntoAs, ToStructError,
        WrapStructErrorAs,
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
    pub use crate::traits::ErrorOweBase;
    pub use crate::{SnapshotCompat, StableSnapshotCompat};
}
