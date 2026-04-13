mod core;
mod testcase;
mod traits;

pub use core::ErrStrategy;
pub use core::{
    print_error, print_error_zh, ConfErrReason, DomainReason, ErrorCode, SourceFrame,
    StructErrorTrait, UvsFrom, UvsReason,
};
pub use core::{ContextRecord, OperationContext, OperationScope, WithContext};
pub use core::{StructError, StructErrorBuilder};
pub use testcase::{TestAssert, TestAssertWithMsg};
pub use traits::{
    ConvStructError, ErrorConv, ErrorWith, ErrorWrap, ToStructError, WrapStructError,
};
pub use traits::{ErrorOwe, ErrorOweBase, ErrorOweSource, ErrorOweSourceBase};

/// Commonly used traits and types for convenient wildcard imports.
///
/// # Example
/// ```rust,ignore
/// use orion_error::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        ContextRecord, ErrorCode, ErrorConv, ErrorOwe, ErrorOweBase, ErrorOweSource,
        ErrorOweSourceBase, ErrorWith, ErrorWrap, ToStructError, UvsFrom, WrapStructError,
    };
    pub use crate::{
        OperationContext, OperationScope, SourceFrame, StructError, StructErrorBuilder, UvsReason,
    };
}

/// Grouped core types and enums.
pub mod types {
    pub use crate::{
        ConfErrReason, ErrStrategy, OperationContext, OperationScope, SourceFrame, StructError,
        StructErrorBuilder, UvsReason, WithContext,
    };
}

/// Grouped conversion and context extension traits.
pub mod traits_ext {
    pub use crate::{
        ContextRecord, ConvStructError, ErrorCode, ErrorConv, ErrorOwe, ErrorOweBase,
        ErrorOweSource, ErrorOweSourceBase, ErrorWith, ErrorWrap, ToStructError, UvsFrom,
        WrapStructError,
    };
}
