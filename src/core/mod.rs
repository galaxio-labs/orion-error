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
mod snapshot;
mod universal;

pub use context::ContextAdd;
pub use context::{OperationContext, OperationScope, WithContext};
pub use domain::DomainReason;
pub use error::{
    convert_error, OwnedDynStdStructError, OwnedStdStructError, SourceFrame, SourcePayloadKind,
    SourcePayloadRef, StdStructRef, StructError, StructErrorBuilder,
};
pub use metadata::{ErrorMetadata, MetadataValue};
pub use reason::{ErrorCategory, ErrorCode, ErrorIdentityProvider};
pub use report::{
    DefaultExposurePolicy, DiagnosticReport, ErrorProtocolSnapshot, ExposureDecision,
    ExposurePolicy, RedactPolicy, Visibility,
};
pub use snapshot::{
    ErrorIdentity, ErrorSnapshot, SnapshotContextFrame, SnapshotSourceFrame, StableErrorSnapshot,
    STABLE_SNAPSHOT_SCHEMA_VERSION,
};
pub use universal::{UnifiedReason, ConfErrReason, UvsReason};
