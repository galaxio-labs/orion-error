//! Error carrier module.
//!
//! Module structure:
//!
//! - [`carrier`] — `StructError<T>`, `StructErrorImpl<T>`
//! - [`builder`] — `StructErrorBuilder<T>`
//! - [`source_chain`] — source payload types (`SourceFrame`, `InternalSourcePayload`, …)
//! - [`std_bridge`] — `std::error::Error` ecosystem wrappers

pub mod builder;
pub mod carrier;
pub mod identity;
pub mod source_chain;
pub mod std_bridge;

#[macro_export]
macro_rules! location {
    () => {
        format!("{}:{}:{}", file!(), line!(), column!())
    };
}

// Re-exports from carrier
pub use carrier::{convert_error, StructError};

// Re-exports from identity
pub use identity::ErrorIdentity;

// Re-exports from builder
pub use builder::StructErrorBuilder;

// Re-exports from source_chain
pub use source_chain::{SourceFrame, SourcePayloadKind, SourcePayloadRef};

// Re-exports from std_bridge
pub use std_bridge::{OwnedDynStdStructError, OwnedStdStructError, StdStructRef};

#[cfg(all(test, feature = "serde"))]
mod tests;
