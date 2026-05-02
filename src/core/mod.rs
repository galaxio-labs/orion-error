mod case;
pub mod cli;
mod context;
mod domain;
mod error;
mod metadata;
mod reason;
mod report;
#[cfg(feature = "serde")]
mod serde;
mod universal;

pub use context::ContextAdd;
pub use context::{OperationContext, OperationScope, WithContext};
pub use domain::DomainReason;
pub use error::{
    convert_error, ErrorIdentity, OwnedDynStdStructError, OwnedStdStructError, SourceFrame,
    SourcePayloadKind, SourcePayloadRef, StdStructRef, StructError, StructErrorBuilder,
};
pub use metadata::{ErrorMetadata, MetadataValue};
pub use reason::{ErrorCategory, ErrorCode, ErrorIdentityProvider};
pub use report::{
    DefaultExposurePolicy, DiagnosticReport, ErrorProtocolSnapshot, ExposureDecision,
    ExposurePolicy, RedactPolicy, Visibility,
};
// snapshot module removed in 0.9 — ErrorIdentity is re-exported from error
pub use universal::{ConfErrReason, UnifiedReason};
