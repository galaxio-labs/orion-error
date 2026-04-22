mod case;
mod context;
mod domain;
mod error;
mod metadata;
mod reason;
mod report;
mod snapshot;
mod universal;
use std::fmt::Display;

pub use context::ContextAdd;
pub use context::{ContextRecord, OperationContext, OperationScope, WithContext};
pub use domain::DomainReason;
pub use error::{
    convert_error, IntoSourcePayload, OwnedDynStdStructError, OwnedStdStructError, SourceFrame,
    SourcePayload, SourcePayloadKind, SourcePayloadRef, StdStructRef, StructError,
    StructErrorBuilder, StructErrorTrait,
};
pub use metadata::{ErrorMetadata, MetadataValue};
pub use reason::ErrorCode;
pub use report::{ErrorReport, RedactPolicy, RenderMode};
pub use snapshot::{
    SnapshotCompat, SnapshotContextFrame, SnapshotSourceFrame, StableSnapshotCompat,
    StableSnapshotContextFrame, StableSnapshotSourceFrame, StableStructErrorSnapshot,
    StructErrorSnapshot, STABLE_SNAPSHOT_CONTEXT_FIELDS, STABLE_SNAPSHOT_SCHEMA_VERSION,
    STABLE_SNAPSHOT_SOURCE_FRAME_FIELDS, STABLE_SNAPSHOT_TOP_LEVEL_FIELDS,
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

pub fn print_error<R: DomainReason + ErrorCode + Display>(err: &StructError<R>) {
    println!("[error code{}] \n{err}", err.reason().error_code());
    for ctx in err.context().iter() {
        println!("context: {ctx}", ctx = ctx.context());
    }
    println!("{}", "-".repeat(50));
}

pub fn print_error_zh<R: DomainReason + ErrorCode + Display>(err: &StructError<R>) {
    println!("[错误代码 {}] \n{err}", err.reason().error_code());
    for ctx in err.context().iter() {
        println!("上下文: {ctx}", ctx = ctx.context());
    }
    println!("{}", "-".repeat(50));
}
