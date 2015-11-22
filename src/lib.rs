//! Read .sdf files, full waveform data files from Riegl Laser Measurement Systems.
//!
//! This is in part a wrapper library around `libsdfifc.so`, Riegl's sdf library. This library also
//! provides functions to convert .sdf files to discrete return .sdc files using Gaussian
//! decomposition.
//!
//! The `sdfifc` library **is not thread-safe**, and so this library should only be used in
//! single-threaded applications. When running this library's test suite, you must set
//! `RUST_TEST_THREADS=1` or else you most likely will get a segfault.

#![deny(box_pointers, fat_ptr_transmutes, missing_copy_implementations, missing_debug_implementations, missing_docs, trivial_casts, trivial_numeric_casts, unused_extern_crates, unused_import_braces, unused_qualifications, unused_results, variant_size_differences)]

extern crate libc;
#[macro_use]
extern crate log;
extern crate peakbag;
extern crate sdc;

macro_rules! sdftry {
    ($expr:expr) => {{
        match $expr {
            0 => {},
            code @ _ => return Err(SdfError::from_i32(code)),
        }
    }}
}

pub mod convert;
pub mod error;
mod ffi;
pub mod file;
pub mod result;

pub use file::File;

use std::ffi::CStr;
use std::ptr;

use libc::c_char;

use error::SdfError;

/// Container structure for information about the library.
#[derive(Debug)]
pub struct LibraryVersion {
    /// The library's major api version.
    pub api_major: u16,
    /// The library's minor api version.
    pub api_minor: u16,
    /// The library's build version.
    pub build_version: String,
    /// The library's build tag.
    pub build_tag: String,
}

/// Returns information about the fwifc library.
///
/// # Examples
///
/// ```
/// let library_version = sdf::library_version();
/// ```
pub fn library_version() -> result::Result<LibraryVersion> {
    unsafe {
        let mut api_major = 0u16;
        let mut api_minor = 0u16;
        let mut build_version: *const c_char = ptr::null_mut();
        let mut build_tag: *const c_char = ptr::null_mut();
        sdftry!(ffi::fwifc_get_library_version(&mut api_major, &mut api_minor, &mut build_version, &mut build_tag));
        Ok(LibraryVersion {
            api_major: api_major,
            api_minor: api_minor,
            build_version: try!(CStr::from_ptr(build_version).to_str()).to_string(),
            build_tag: try!(CStr::from_ptr(build_tag).to_str()).to_string(),
        })
    }
}
