use std::{error::Error as StdError, fmt::Display, sync::Arc};

use super::{
    context::OperationContext, domain::DomainReason, metadata::ErrorMetadata, ContextAdd,
    ErrorCategory, ErrorIdentityProvider,
};
use crate::traits::ErrorWith;
#[macro_export]
macro_rules! location {
    () => {
        format!("{}:{}:{}", file!(), line!(), column!())
    };
}

include!("source.rs");
include!("runtime.rs");

#[cfg(all(test, feature = "serde"))]
mod tests;
