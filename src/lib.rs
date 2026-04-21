mod core;
mod testcase;
mod traits;

pub use core::ErrStrategy;
pub use core::{
    print_error, print_error_zh, ConfErrReason, DomainReason, ErrorCode, ErrorMetadata,
    ErrorReport, MetadataValue, RedactPolicy, RenderMode, SourceFrame, StructErrorTrait, UvsFrom,
    UvsReason,
};
pub use core::{ContextRecord, OperationContext, OperationScope, WithContext};
pub use core::{StructError, StructErrorBuilder};
pub use testcase::{TestAssert, TestAssertWithMsg};
pub use traits::{
    raw_source, ConvStructError, ErrorConv, ErrorWith, ErrorWrap, ErrorWrapAs, IntoAs, RawSource,
    RawStdError, ToStructError, WrapStructError, WrapStructErrorAs,
};
pub use traits::{ErrorOwe, ErrorOweBase, ErrorOweSource, ErrorOweSourceBase};

/// V1 primary-path traits and types for convenient wildcard imports.
///
/// # Example
/// ```rust,ignore
/// use orion_error::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        raw_source, ErrorMetadata, ErrorReport, MetadataValue, OperationContext, OperationScope,
        RawSource, RawStdError, RedactPolicy, RenderMode, SourceFrame, StructError,
        StructErrorBuilder, UvsReason,
    };
    pub use crate::{
        ContextRecord, ErrorCode, ErrorConv, ErrorWith, ErrorWrapAs, IntoAs, ToStructError,
        UvsFrom, WrapStructErrorAs,
    };
}

/// Compatibility wildcard imports for legacy conversion APIs.
///
/// Use this only when maintaining older `owe_*()` / `err_wrap(...)` call paths.
pub mod compat_prelude {
    pub use crate::{
        ErrorOwe, ErrorOweBase, ErrorOweSource, ErrorOweSourceBase, ErrorWrap, WrapStructError,
    };
}

/// Grouped core types and enums.
pub mod types {
    pub use crate::{
        ConfErrReason, ErrStrategy, ErrorMetadata, ErrorReport, MetadataValue, OperationContext,
        OperationScope, RedactPolicy, RenderMode, SourceFrame, StructError, StructErrorBuilder,
        UvsReason, WithContext,
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
