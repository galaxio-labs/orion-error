use crate::{core::DomainReason, StructError};
use std::sync::Arc;

use super::{
    context::OperationResult, report::DiagnosticReport, ErrorCategory, ErrorIdentityProvider,
    ErrorMetadata, OperationContext, SourceFrame,
};

include!("types.rs");

#[cfg(test)]
mod tests;

#[cfg(doc)]
mod stable_snapshot_compile_fail_docs {
    //! ```compile_fail
    //! use orion_error::{StableErrorSnapshot, ErrorSnapshot};
    //!
    //! fn must_not_compile(stable: StableErrorSnapshot) -> ErrorSnapshot {
    //!     stable.into()
    //! }
    //! ```
}
