use std::fmt::{Debug, Display};

use derive_more::From;
use thiserror::Error;

use super::UvsReason;

pub trait DomainReason:
    PartialEq + Display + Debug + Send + Sync + 'static
{
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Error, From)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum NullReason {
    #[allow(dead_code)]
    #[error("null")]
    Null,
    #[error("{0}")]
    Uvs(UvsReason),
}

impl DomainReason for NullReason {}
