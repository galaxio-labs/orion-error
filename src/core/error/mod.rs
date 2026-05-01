//! Error carrier module.
//!
//! Module structure:
//!
//! - [`carrier`] — `StructError<T>`, `StructErrorImpl<T>`
//! - [`builder`] — `StructErrorBuilder<T>`
//! - [`source_chain`] — source payload types (`SourceFrame`, `InternalSourcePayload`, …)
//! - [`std_bridge`] — `std::error::Error` ecosystem wrappers

pub mod carrier;
pub mod builder;
pub mod source_chain;
pub mod std_bridge;

/// Generate `From<StructError<Source>> for StructError<Target>` impls so that
/// `?` automatically converts between error types without calling `.upcast()`.
///
/// **Note**: this macro only works within the `orion-error` crate itself due
/// to Rust's orphan rule (cannot `impl ForeignTrait<ForeignType<T>>` from a
/// downstream crate). Downstream code should use `.upcast()` instead.
///
/// # Example (internal use only)
/// ```rust,ignore
/// upcast_from!(SubReason => MainReason);
/// ``````
#[macro_export]
macro_rules! upcast_from {
    ($source:ty => $target:ty) => {
        impl ::std::convert::From<$crate::StructError<$source>>
            for $crate::StructError<$target>
        where
            $source: $crate::reason::DomainReason,
            $target: $crate::reason::DomainReason + ::std::convert::From<$source>,
        {
            fn from(other: $crate::StructError<$source>) -> Self {
                $crate::convert_error(other)
            }
        }
    };
    ( $($source:ty),+ => $target:ty ) => {
        $(
            $crate::upcast_from!($source => $target);
        )+
    };
}

#[macro_export]
macro_rules! location {
    () => {
        format!("{}:{}:{}", file!(), line!(), column!())
    };
}

// Re-exports from carrier
pub use carrier::{
    convert_error, StructError,
};
// Re-exports from builder
pub use builder::StructErrorBuilder;

// Re-exports from source_chain
pub use source_chain::{
    SourceFrame, SourcePayloadKind, SourcePayloadRef,
};

// Re-exports from std_bridge
pub use std_bridge::{
    OwnedDynStdStructError, OwnedStdStructError, StdStructRef,
};

#[cfg(all(test, feature = "serde"))]
mod tests;
