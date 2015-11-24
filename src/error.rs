//! Wrapper around fwifc's error handling routines.

use std::error;
use std::ffi::{CStr, NulError};
use std::fmt;
use std::ptr;
use std::str::Utf8Error;

use libc::c_char;
use sdc;

use ffi::fwifc_get_last_error;
use file::Channel;

/// Our error type.
#[derive(Debug)]
pub enum Error {
    /// A bad argument has been passed to sdfifc.
    BadArg(String),
    /// The end of an sdf file has been reached.
    EndOfFile(String),
    /// The specified channel is invalid.
    InvalidChannel(u32),
    /// The channel is a valid channel, but we couldn't find it when we tried.
    MissingChannel(Channel),
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
    /// A wrapper around `sdc::Error`.
    Sdc(sdc::Error),
    /// Either zero or more than one reference peak.
    NeedSingleReferencePeak(usize),
    /// A wrapper around `std::str::Utf8Error`.
    Utf8(Utf8Error),
    /// An unknown code has been provided to an error-mapping routine.
    UnknownCode(i32),
    /// An unknown exception has occurred inside sdfifc.
    UnknownException(String),
    /// The given sdf file is not in a supported format.
    UnsupportedFormat(String),
}

impl Error {
    /// Converts an i32 error code to an `Error`.
    ///
    /// This function also calls `last_error` to get the error message from fwifc.
    ///
    /// # Panics
    ///
    /// Panics if you pass in zero. That's because zero is not an error, and your code should not
    /// be trying to create an error if there isn't one.
    pub fn from_i32(code: i32) -> Error {
        match code {
            -1 => Error::EndOfFile(last_error().to_string()),
            0 => panic!("Refusing to create an error with code zero"),
            1 => Error::BadArg(last_error().to_string()),
            2 => Error::UnsupportedFormat(last_error().to_string()),
            3 => Error::MissingIndex(last_error().to_string()),
            4 => Error::UnknownException(last_error().to_string()),
            5 => Error::NotImplemented(last_error().to_string()),
            6 => Error::Runtime(last_error().to_string()),
            _ => Error::UnknownCode(code),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::BadArg(_) => "bad argument",
            Error::EndOfFile(_) => "end of file",
            Error::InvalidChannel(_) => "invalid channel",
            Error::MissingChannel(_) => "missing channel",
            Error::MissingIndex(_) => "missing index",
            Error::NeedSingleReferencePeak(_) => "zero or more than one reference peaks",
            Error::NoCalibrationTableForChannel(_) => "no calibration table for channel",
            Error::NotImplemented(_) => "not implemented",
            Error::Nul(ref err) => err.description(),
            Error::Runtime(_) => "runtime error",
            Error::Sdc(ref err) => err.description(),
            Error::Utf8(ref err) => err.description(),
            Error::UnknownCode(_) => "unknown code",
            Error::UnknownException(_) => "unknown exception",
            Error::UnsupportedFormat(_) => "unsupported format",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Nul(ref err) => Some(err),
            Error::Utf8(ref err) => Some(err),
            _ => None,
        }
    }
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::BadArg(ref msg) => write!(f, "Bad argument: {}", msg),
            Error::EndOfFile(ref msg) => write!(f, "End of file: {}", msg),
            Error::InvalidChannel(u8) => write!(f, "Invalid channel: {}", u8),
            Error::MissingChannel(ref channel) => write!(f, "Missing channel: {}", channel),
            Error::MissingIndex(ref msg) => write!(f, "Missing index: {}", msg),
            Error::NeedSingleReferencePeak(n) =>
                write!(f, "Wanted one reference peak, got {}", n),
            Error::NoCalibrationTableForChannel(channel) =>
                write!(f, "No calibration table for channel: {}", channel),
            Error::NotImplemented(ref msg) => write!(f, "Not implemented: {}", msg),
            Error::Nul(ref err) => write!(f, "Nul error: {}", err),
            Error::Runtime(ref msg) => write!(f, "Runtime error: {}", msg),
            Error::Sdc(ref err) => write!(f, "Sdc error: {}", err),
            Error::Utf8(ref err) => write!(f, "Utf8 error: {}", err),
            Error::UnknownCode(code) => write!(f, "Unknown code: {}", code),
            Error::UnknownException(ref msg) => write!(f, "Unknown exception: {}", msg),
            Error::UnsupportedFormat(ref msg) => write!(f, "Unsupported format: {}", msg),
        }
    }
}

impl From<NulError> for Error {
    fn from(err: NulError) -> Error {
        Error::Nul(err)
    }
}

impl From<sdc::Error> for Error {
    fn from(err: sdc::Error) -> Error {
        Error::Sdc(err)
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Error {
        Error::Utf8(err)
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
        assert_eq!("(no error)", message);
    }
}
