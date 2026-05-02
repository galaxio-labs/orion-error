use crate::reason::ErrorCategory;
use crate::{core::DomainReason, StructError};
use std::sync::Arc;

use super::{
    ErrorIdentity, ErrorIdentityProvider, ErrorMetadata, MetadataValue, OperationContext,
    SourceFrame,
};

include!("redaction_impl.rs");
include!("diagnostic.rs");
include!("protocol.rs");

#[cfg(test)]
mod tests;
