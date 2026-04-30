use crate::{core::DomainReason, StructError};
use crate::reason::ErrorCategory;
use std::sync::Arc;

use super::{
    snapshot::{ErrorIdentity, ErrorSnapshot, StableErrorSnapshot},
    ErrorIdentityProvider, ErrorMetadata, MetadataValue, OperationContext, SourceFrame,
};

include!("redaction_impl.rs");
include!("diagnostic.rs");
include!("protocol.rs");

#[cfg(test)]
mod tests;
