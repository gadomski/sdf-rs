//! Our custom result handling.

use std::result;

use error::Error;

/// Our custom result type.
pub type Result<T> = result::Result<T, Error>;
