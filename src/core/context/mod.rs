#[cfg(all(feature = "log", not(feature = "tracing")))]
use log::{debug, error, info, trace, warn};
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use super::metadata::{ErrorMetadata, MetadataValue};

include!("types.rs");
include!("convert.rs");

#[cfg(test)]
mod tests;
