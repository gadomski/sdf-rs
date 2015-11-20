//! The Foreign Function Interface wrappers around libsdfifc.

use libc::{c_char, c_double, uint16_t, uint32_t};

#[allow(non_camel_case_types)]
enum fwifc_file_t {}

#[allow(non_camel_case_types)]
pub type fwifc_file = *mut fwifc_file_t;

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct fwifc_sbl_t {
    pub time_sosbl: c_double,
    pub channel: uint32_t,
    pub sample_count: uint32_t,
    pub sample_size: uint32_t,
    pub sample: *mut uint16_t,
}

#[link(name="sdfifc")]
extern {
    pub fn fwifc_open(path: *const c_char, file: *mut fwifc_file) -> i32;
    pub fn fwifc_close(file: fwifc_file) -> i32;
    pub fn fwifc_get_library_version(api_major: *mut uint16_t,
                                     api_minor: *mut uint16_t,
                                     build_version: *mut *const c_char,
                                     build_tag: *mut *const c_char)
                                     -> i32;
    pub fn fwifc_get_last_error(message: *mut *const c_char) -> i32;
    pub fn fwifc_reindex(file: fwifc_file) -> i32;
    pub fn fwifc_set_sosbl_relative(file: fwifc_file, value: i32) -> i32;
    pub fn fwifc_get_info(file: fwifc_file,
                          instrument: *mut *const c_char,
                          serial: *mut *const c_char,
                          epoch: *mut *const c_char,
                          v_group: *mut c_double,
                          sampling_time: *mut c_double,
                          flags: *mut uint16_t,
                          num_facets: *mut uint16_t)
                          -> i32;
    pub fn fwifc_get_calib(file: fwifc_file,
                           table_kind: uint16_t,
                           count: *mut uint32_t,
                           abscissa: *mut *const c_double,
                           ordinate: *mut *const c_double)
                           -> i32;
    pub fn fwifc_read(file: fwifc_file,
                      time_sorg: *mut c_double,
                      time_external: *mut c_double,
                      origin: *mut c_double,
                      direction: *mut c_double,
                      flags: *mut uint16_t,
                      facet: *mut uint16_t,
                      sbl_count: *mut uint32_t,
                      sbl_size: *mut uint32_t,
                      sbl: *mut *mut fwifc_sbl_t)
                      -> i32;
    pub fn fwifc_seek(file: fwifc_file, index: uint32_t) -> i32;
    pub fn fwifc_seek_time(file: fwifc_file, time: c_double) -> i32;
    pub fn fwifc_seek_time_external(file: fwifc_file, time: c_double) -> i32;
    pub fn fwifc_tell(file: fwifc_file, index: *mut uint32_t) -> i32;
}
