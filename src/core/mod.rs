mod case;
mod context;
mod domain;
mod error;
mod metadata;
mod reason;
mod report;
mod snapshot;
mod universal;

pub use context::ContextAdd;
pub use context::{ContextRecord, OperationContext, OperationScope, WithContext};
pub use domain::DomainReason;
pub use error::{
    convert_error, IntoSourcePayload, OwnedDynStdStructError, OwnedStdStructError, SourceFrame,
    SourcePayload, SourcePayloadKind, SourcePayloadRef, StdStructRef, StructError,
    StructErrorBuilder,
};
pub use metadata::{ErrorMetadata, MetadataValue};
pub use reason::{ErrorCategory, ErrorCode, ErrorIdentityProvider};
pub use report::{
    DefaultExposurePolicy, DiagnosticReport, ErrorCliResponse, ErrorHttpResponse, ErrorLogResponse,
    ErrorProtocolSnapshot, ErrorRenderer, ErrorRpcResponse, ExposureDecision, ExposurePolicy,
    ExposureView, RedactPolicy, RenderMode, TextDiagnosticRenderer, TextReportRenderer, Visibility,
};
pub use snapshot::{
    ErrorIdentity, ErrorSnapshot, SnapshotContextFrame, SnapshotSourceFrame, StableErrorSnapshot,
    StableSnapshotContextFrame, StableSnapshotSourceFrame, STABLE_SNAPSHOT_SCHEMA_VERSION,
};
pub use universal::{ConfErrReason, UvsFrom, UvsReason};

pub enum ErrStrategy {
    /// 带退避策略的重试（包含基本参数）
    Retry,
    /// 静默忽略错误
    Ignore,
    /// 传播错误（默认行为）
    Throw,
}
