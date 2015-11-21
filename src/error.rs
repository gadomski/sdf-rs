//! Wrapper around fwifc's error handling routines.

use std::error;
use std::ffi::{CStr, NulError};
use std::fmt;
use std::ptr;
use std::str::Utf8Error;

use libc::c_char;

use ffi::fwifc_get_last_error;

use file::Channel;

/// Our error type.
#[derive(Debug, PartialEq)]
pub enum SdfError {
    /// A bad argument has been passed to sdfifc.
    BadArg(String),
    /// The end of an sdf file has been reached.
    EndOfFile(String),
    /// The specified channel is invalid.
    InvalidChannel(u32),
    /// The sdf file is missing an index.
    ///
    /// Some file-based operations, namely reads and seeks, require an index. Use `File::reindex()`
    /// to create one.
    MissingIndex(String),
    /// There is no calibration table for the given channel.
    NoCalibrationTableForChannel(Channel),
    /// The given function is unimplemented by sdfifc.
    NotImplemented(String),
    /// A wrapper around `std::ffi::NulError`.
    Nul(NulError),
    /// A runtime error on the part of sdfifc.
    Runtime(String),
    /// A wrapper around `std::str::Utf8Error`.
    Utf8(Utf8Error),
    /// An unknown code has been provided to an error-mapping routine.
    UnknownCode(i32),
    /// An unknown exception has occurred inside sdfifc.
    UnknownException(String),
    /// The given sdf file is not in a supported format.
    UnsupportedFormat(String),
}

impl SdfError {
    /// Converts an i32 error code to an `SdfError`.
    ///
    /// This function also calls `last_error` to get the error message from fwifc.
    ///
    /// # Panics
    ///
    /// Panics if you pass in zero. That's because zero is not an error, and your code should not
    /// be trying to create an error if there isn't one.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::error::SdfError;
    /// assert_eq!(SdfError::EndOfFile("".to_string()), SdfError::from_i32(-1));
    /// assert_eq!(SdfError::UnknownCode(7), SdfError::from_i32(7));
    /// ```
    pub fn from_i32(code: i32) -> SdfError {
        match code {
            -1 => SdfError::EndOfFile(last_error().to_string()),
            0 => panic!("Refusing to create an error with code zero"),
            1 => SdfError::BadArg(last_error().to_string()),
            2 => SdfError::UnsupportedFormat(last_error().to_string()),
            3 => SdfError::MissingIndex(last_error().to_string()),
            4 => SdfError::UnknownException(last_error().to_string()),
            5 => SdfError::NotImplemented(last_error().to_string()),
            6 => SdfError::Runtime(last_error().to_string()),
            _ => SdfError::UnknownCode(code),
        }
    }
}

impl error::Error for SdfError {
    fn description(&self) -> &str {
        match *self {
            SdfError::BadArg(_) => "bad argument",
            SdfError::EndOfFile(_) => "end of file",
            SdfError::InvalidChannel(_) => "invalid channel",
            SdfError::MissingIndex(_) => "missing index",
            SdfError::NoCalibrationTableForChannel(_) => "no calibration table for channel",
            SdfError::NotImplemented(_) => "not implemented",
            SdfError::Nul(ref err) => err.description(),
            SdfError::Runtime(_) => "runtime error",
            SdfError::Utf8(ref err) => err.description(),
            SdfError::UnknownCode(_) => "unknown code",
            SdfError::UnknownException(_) => "unknown exception",
            SdfError::UnsupportedFormat(_) => "unsupported format",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            SdfError::Nul(ref err) => Some(err),
            SdfError::Utf8(ref err) => Some(err),
            _ => None,
        }
    }
}


impl fmt::Display for SdfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SdfError::BadArg(ref msg) => write!(f, "Bad argument: {}", msg),
            SdfError::EndOfFile(ref msg) => write!(f, "End of file: {}", msg),
            SdfError::InvalidChannel(u8) => write!(f, "Invalid channel: {}", u8),
            SdfError::MissingIndex(ref msg) => write!(f, "Missing index: {}", msg),
            SdfError::NoCalibrationTableForChannel(channel) =>
                write!(f, "No calibration table for channel: {}", channel),
            SdfError::NotImplemented(ref msg) => write!(f, "Not implemented: {}", msg),
            SdfError::Nul(ref err) => write!(f, "Nul error: {}", err),
            SdfError::Runtime(ref msg) => write!(f, "Runtime error: {}", msg),
            SdfError::Utf8(ref err) => write!(f, "Utf8 error: {}", err),
            SdfError::UnknownCode(code) => write!(f, "Unknown code: {}", code),
            SdfError::UnknownException(ref msg) => write!(f, "Unknown exception: {}", msg),
            SdfError::UnsupportedFormat(ref msg) => write!(f, "Unsupported format: {}", msg),
        }
    }
}

impl From<NulError> for SdfError {
    fn from(err: NulError) -> SdfError {
        SdfError::Nul(err)
    }
}

impl From<Utf8Error> for SdfError {
    fn from(err: Utf8Error) -> SdfError {
        SdfError::Utf8(err)
    }
}

/// Retrieves the last error from fwifc and returns it as a `&'static str`.
///
/// # Panics
///
/// This function panics if the error function itself returns an error code, or if the error
/// message cannot be converted into a string. We figure that if the error function is in error,
/// that really is a time for panic.
///
/// # Examples
///
/// ```
/// use sdf::error::last_error;
/// let message = last_error();
/// ```
pub fn last_error() -> &'static str {
    unsafe {
        let mut message: *const c_char = ptr::null_mut();
        let result = fwifc_get_last_error(&mut message);
        if result != 0 {
            panic!("Non-zero return code from `fwifc_get_last_error`: {}",
                   result);
        }
        CStr::from_ptr(message).to_str().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_error_expected() {
        let message = last_error();
        assert_eq!("", message);
    }
}
