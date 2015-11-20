//! Public, safe wrappers around `fwifc_file` and its member functions.

use std::ffi::{CStr, CString};
use std::ptr;

use libc::c_char;

use error::SdfError;
use ffi::{fwifc_close, fwifc_file, fwifc_get_calib, fwifc_get_info, fwifc_open, fwifc_read,
          fwifc_reindex, fwifc_sbl_t, fwifc_seek, fwifc_seek_time, fwifc_seek_time_external,
          fwifc_tell, fwifc_set_sosbl_relative};
use result::Result;

/// An .sdf file.
#[derive(Debug)]
pub struct File {
    handle: fwifc_file,
}

impl File {
    /// Opens an .sdf data file.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::File;
    /// let file = File::open("data/110630_174316.sdf").unwrap();
    /// ```
    pub fn open<T: Into<Vec<u8>>>(path: T) -> Result<File> {
        unsafe {
            let path = try!(CString::new(path));
            let mut file: fwifc_file = ptr::null_mut();
            sdftry!(fwifc_open(path.as_ptr(), &mut file));
            Ok(File { handle: file })
        }
    }

    /// (Re-)Creates the index file.
    ///
    /// The index file is required for navigating the file. This is a blocking operation and may
    /// take some time. The index file is placed in the same directory as the data file.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::remove_file;
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// remove_file("data/110630_174316.idx").unwrap();
    /// ```
    pub fn reindex(&mut self) -> Result<()> {
        unsafe { Ok(sdftry!(fwifc_reindex(self.handle))) }
    }

    /// Sets the mode timestamp of the start of the sample block.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::{File, SosblMode};
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.set_sosbl_mode(SosblMode::Relative).unwrap();
    /// file.set_sosbl_mode(SosblMode::Absolute).unwrap();
    /// ```
    pub fn set_sosbl_mode(&mut self, mode: SosblMode) -> Result<()> {
        unsafe {
            let value = match mode {
                SosblMode::Absolute => 0,
                SosblMode::Relative => 1,
            };
            Ok(sdftry!(fwifc_set_sosbl_relative(self.handle, value)))
        }
    }

    /// Gets information about the file.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// let file_info = file.info();
    /// ```
    pub fn info(&mut self) -> Result<FileInfo> {
        unsafe {
            let mut instrument: *const c_char = ptr::null_mut();
            let mut serial: *const c_char = ptr::null_mut();
            let mut epoch: *const c_char = ptr::null_mut();
            let mut v_group = 0f64;
            let mut sampling_time = 0f64;
            let mut flags = 0u16;
            let mut num_facets = 0u16;
            sdftry!(fwifc_get_info(self.handle,
                                   &mut instrument,
                                   &mut serial,
                                   &mut epoch,
                                   &mut v_group,
                                   &mut sampling_time,
                                   &mut flags,
                                   &mut num_facets));
            Ok(FileInfo {
                instrument: try!(CStr::from_ptr(instrument).to_str()).to_string(),
                serial: try!(CStr::from_ptr(serial).to_str()).to_string(),
                epoch: try!(CStr::from_ptr(epoch).to_str()).to_string(),
                v_group: v_group,
                sampling_time: sampling_time,
                gps_synchronized: flags & 0x01 == 1,
                num_facets: num_facets,
            })
        }
    }

    /// Gets the calibration info for the file.
    ///
    /// We manually copy all of the calibration info into new vectors because we can't really trust
    /// the memory behind the fwifc call.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::{File, CalibrationTableKind};
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// let calibration = file.calibration(CalibrationTableKind::Amplitude(0)).unwrap();
    /// ```
    pub fn calibration(&mut self, kind: CalibrationTableKind) -> Result<Calibration> {
        unsafe {
            let mut count = 0u32;
            let mut abscissa: *const f64 = ptr::null_mut();
            let mut ordinate: *const f64 = ptr::null_mut();
            sdftry!(fwifc_get_calib(self.handle,
                                    try!(kind.as_u16()),
                                    &mut count,
                                    &mut abscissa,
                                    &mut ordinate));
            let mut abscissa_vec = Vec::with_capacity(count as usize);
            let mut ordinate_vec = Vec::with_capacity(count as usize);
            for i in 0..count {
                abscissa_vec.push(*abscissa.offset(i as isize));
                ordinate_vec.push(*ordinate.offset(i as isize));
            }
            Ok(Calibration {
                abscissa: abscissa_vec,
                ordinate: ordinate_vec,
            })
        }
    }

    /// Reads a sample data record from the file.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::fs::remove_file;
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// let record = file.read().unwrap();
    /// # remove_file("data/110630_174316.idx").unwrap();
    /// ```
    pub fn read(&mut self) -> Result<Record> {
        unsafe {
            let mut time_sorg = 0.0;
            let mut time_external = 0.0;
            let mut origin = [0.0f64; 3];
            let mut direction = [0.0f64; 3];
            let mut flags = 0;
            let mut facet = 0;
            let mut sbl_count = 0;
            let mut sbl_size = 0;
            let mut sbl: *mut fwifc_sbl_t = ptr::null_mut();
            sdftry!(fwifc_read(self.handle,
                               &mut time_sorg,
                               &mut time_external,
                               origin.as_mut_ptr(),
                               direction.as_mut_ptr(),
                               &mut flags,
                               &mut facet,
                               &mut sbl_count,
                               &mut sbl_size,
                               &mut sbl));
            let mut blocks = Vec::with_capacity(sbl_count as usize);
            for i in 0..sbl_count {
                let ref block = *sbl.offset(i as isize);
                let mut samples = Vec::with_capacity(block.sample_count as usize);
                for j in 0..block.sample_count {
                    samples.push(*block.sample.offset(j as isize));
                }
                blocks.push(SampleBlock {
                    time_sosbl: block.time_sosbl,
                    channel: block.channel,
                    samples: samples,
                })
            }
            Ok(Record {
                time_sorg: time_sorg,
                time_external: time_external,
                origin: origin,
                direction: direction,
                synchronized: flags & 0x01 == 1,
                sync_lastsec: flags & 0x02 == 2,
                housekeeping: flags & 0x04 == 4,
                facet: facet,
                blocks: blocks,
            })
        }
    }

    /// Seeks to a record index in the file.
    ///
    /// # Examples
    ///
    /// Seeks to the first record.
    ///
    /// ```
    /// # use std::fs::remove_file;
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// file.seek(1).unwrap();
    /// # remove_file("data/110630_174316.idx").unwrap();
    /// ```
    ///
    /// Seeks to the end of the file.
    ///
    /// ```
    /// # use std::fs::remove_file;
    /// use std::u32;
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// file.seek(u32::MAX).unwrap();
    /// # remove_file("data/110630_174316.idx").unwrap();
    /// ```
    pub fn seek(&mut self, index: u32) -> Result<()> {
        unsafe { Ok(sdftry!(fwifc_seek(self.handle, index))) }
    }

    /// Seeks to an internal timestamp, in seconds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::fs::remove_file;
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// file.seek_time(1.0).unwrap();
    /// # remove_file("data/110630_174316.idx").unwrap();
    /// ```
    pub fn seek_time(&mut self, time: f64) -> Result<()> {
        unsafe { Ok(sdftry!(fwifc_seek_time(self.handle, time))) }
    }

    /// Seeks to an external time in seconds.
    ///
    /// Either day or week seconds. This requires GPS-synchronized data.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::fs::remove_file;
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// file.seek_time_external(1.0).unwrap();
    /// # remove_file("data/110630_174316.idx").unwrap();
    /// ```
    pub fn seek_time_external(&mut self, time: f64) -> Result<()> {
        unsafe { Ok(sdftry!(fwifc_seek_time_external(self.handle, time))) }
    }

    /// Returns the index of the next record to be read.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::fs::remove_file;
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// assert_eq!(1, file.tell().unwrap());
    /// file.read().unwrap();
    /// assert_eq!(2, file.tell().unwrap());
    /// # remove_file("data/110630_174316.idx").unwrap();
    /// ```
    pub fn tell(&mut self) -> Result<u32> {
        let mut index = 0u32;
        unsafe { sdftry!(fwifc_tell(self.handle, &mut index)) }
        Ok(index)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe {
            let result = fwifc_close(self.handle);
            if result != 0 {
                panic!("Error when closing file: {}", SdfError::from_i32(result));
            }
        }
    }
}

/// The timestamp of the start of the sample block can be relative or absolute.
///
/// If absolute, large values could lose precision.
#[derive(Clone, Copy, Debug)]
pub enum SosblMode {
    /// The time of the start of the sample block is given relative to the file, to preserve
    /// maximum precision.
    Relative,
    /// The time of the start of the sample block is given in absolute time, which may not provide
    /// enough precision under some circumstances.
    Absolute,
}

/// A container for information about a file.
#[derive(Debug)]
pub struct FileInfo {
    /// The instrument name, e.g. "Q680I".
    pub instrument: String,
    /// The instrument serial number.
    pub serial: String,
    /// The type of external time, or "UNKNOWN".
    pub epoch: String,
    /// The group velocity in m/s.
    pub v_group: f64,
    /// The sampling interval in seconds.
    pub sampling_time: f64,
    /// True if this file's time was synchronized with GPS time.
    pub gps_synchronized: bool,
    /// The numbe rof mirror facets in the intrument.
    pub num_facets: u16,
}

/// A container for calibration information.
#[derive(Debug)]
pub struct Calibration {
    /// The abscissa can be assumed to monotonically increase. These are in pairs with the
    /// ordinates.
    pub abscissa: Vec<f64>,
    /// Ordinate calibration values.
    pub ordinate: Vec<f64>,
}

/// A type of calibration table.
///
/// Really a pair between type and channel number.
#[derive(Clone, Copy, Debug)]
pub enum CalibrationTableKind {
    /// An amplitude calibration table.
    Amplitude(u8),
    /// A range calibration table.
    Range(u8),
}

impl CalibrationTableKind {
    /// Returns this calibration table kind as a `u16`.
    ///
    /// Returns an error if the combination is not valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::CalibrationTableKind;
    /// assert_eq!(0, CalibrationTableKind::Amplitude(0).as_u16().unwrap());
    /// assert_eq!(3, CalibrationTableKind::Range(1).as_u16().unwrap());
    /// assert!(CalibrationTableKind::Amplitude(2).as_u16().is_err());
    /// ```
    pub fn as_u16(&self) -> Result<u16> {
        match *self {
            CalibrationTableKind::Amplitude(n) if n == 0 || n == 1 => Ok(n as u16),
            CalibrationTableKind::Range(n) if n == 0 || n == 1 => Ok((n + 2) as u16),
            CalibrationTableKind::Amplitude(n) | CalibrationTableKind::Range(n) =>
                Err(SdfError::InvalidChannel(n)),
        }
    }
}

/// A sample data record.
#[derive(Debug)]
pub struct Record {
    /// The start of the range gate, in second.
    pub time_sorg: f64,
    /// The external time in seconds relative to epoch.
    pub time_external: f64,
    /// The origin vector, in meters.
    pub origin: [f64; 3],
    /// The direction vector (dimensionless).
    pub direction: [f64; 3],
    /// Is this record GPS synchronized?
    pub synchronized: bool,
    /// Has this record been synchronized within the last second?
    pub sync_lastsec: bool,
    /// Is this a housekeeping block?
    pub housekeeping: bool,
    /// The mirror fact number.
    pub facet: u16,
    /// The size of sample block in bytes.
    pub blocks: Vec<SampleBlock>,
}

/// A sample block.
#[derive(Debug)]
pub struct SampleBlock {
    /// The start of the sample block, in seconds.
    pub time_sosbl: f64,
    /// The channel: 0:high, 1:low, 2:saturation, 3:reference.
    pub channel: u32,
    /// The actual data samples.
    pub samples: Vec<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use error::SdfError;

    #[test]
    fn open_throws_on_bad_filename() {
        let err = File::open("notafile.sdf").unwrap_err();
        assert_eq!(SdfError::UnknownException("Unable to open file \"notafile.sdf\"".to_string()),
                   err);
    }

    #[test]
    fn file_info() {
        let mut file = File::open("data/110630_174316.sdf").unwrap();
        let info = file.info().unwrap();
        assert_eq!("Q680I", info.instrument);
        assert_eq!("9998212", info.serial);
        assert_eq!("UNKNOWN", info.epoch);
        assert_eq!(299707502.1266937, info.v_group);
        assert_eq!(0.000000001, info.sampling_time);
        assert!(info.gps_synchronized);
        assert_eq!(4, info.num_facets);
    }

    #[test]
    fn file_calibration() {
        let mut file = File::open("data/110630_174316.sdf").unwrap();
        let calib = file.calibration(CalibrationTableKind::Amplitude(0)).unwrap();
        assert_eq!(256, calib.abscissa.len());
        assert_eq!(calib.ordinate.len(), calib.abscissa.len());
    }
}
