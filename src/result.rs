//! Our custom result handling.

use std::result;

use error::SdfError;

/// Our custom result type.
pub type Result<T> = result::Result<T, SdfError>;
