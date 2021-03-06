//! Public, safe wrappers around `fwifc_file` and its member functions.

use std::ffi::{CStr, CString, OsString};
use std::fmt;
use std::fs::{metadata, remove_file};
use std::iter::{Iterator, IntoIterator};
use std::path::Path;
use std::ptr;

use libc::c_char;

use Result;
use error::Error;
use ffi::{fwifc_close, fwifc_file, fwifc_get_calib, fwifc_get_info, fwifc_open, fwifc_read,
          fwifc_reindex, fwifc_sbl_t, fwifc_seek, fwifc_seek_time, fwifc_seek_time_external,
          fwifc_tell, fwifc_set_sosbl_relative};

/// An .sdf file.
///
/// The file is mostly a simple wrapper around an `fwifc_file` handle, but we do a bit of extra
/// smarts (or dumbs) to help other users.
///
/// - We ensure that we reindex the file only once, regardless of the number of times that
/// `reindex` has been called.
#[derive(Debug)]
pub struct File {
    handle: fwifc_file,
    index_path: OsString,
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
            let index_path = Path::new(try!(path.to_str())).with_extension("idx").into_os_string();
            Ok(File {
                handle: file,
                index_path: index_path,
            })
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
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// ```
    pub fn reindex(&mut self) -> Result<()> {
        if !self.indexed() {
            info!("Reindexing");
            unsafe { sdftry!(fwifc_reindex(self.handle)) }
        }
        Ok(())
    }

    /// Remove this file's index from the filesystem.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// file.remove_index().unwrap();
    /// ```
    pub fn remove_index(&self) -> Result<()> {
        remove_file(&self.index_path).map_err(|e| Error::from(e))
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
    /// use sdf::file::{File, CalibrationTableKind, Channel};
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// let calibration = file.calibration(CalibrationTableKind::Amplitude(Channel::High)).unwrap();
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
    /// # Panics
    ///
    /// Panics if the underlying sdfifc library returns a record with two blocks with the same
    /// channel. We assume that this can't happen, and so we panic (rather than returning an error)
    /// to indicate that this is a very exceptional case.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// let record = file.read().unwrap();
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
                let channel = try!(Channel::from_u32(block.channel));
                blocks.push(Block {
                    time_sosbl: block.time_sosbl,
                    channel: channel,
                    samples: samples,
                });
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
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// file.seek(1).unwrap();
    /// ```
    ///
    /// Seeks to the end of the file.
    ///
    /// ```
    /// use std::u32;
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// file.seek(u32::MAX).unwrap();
    /// ```
    pub fn seek(&mut self, index: u32) -> Result<()> {
        unsafe { Ok(sdftry!(fwifc_seek(self.handle, index))) }
    }

    /// Seeks to an internal timestamp, in seconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// file.seek_time(1.0).unwrap();
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
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// file.seek_time_external(1.0).unwrap();
    /// ```
    pub fn seek_time_external(&mut self, time: f64) -> Result<()> {
        unsafe { Ok(sdftry!(fwifc_seek_time_external(self.handle, time))) }
    }

    /// Returns the index of the next record to be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::File;
    /// let mut file = File::open("data/110630_174316.sdf").unwrap();
    /// file.reindex().unwrap();
    /// assert_eq!(1, file.tell().unwrap());
    /// file.read().unwrap();
    /// assert_eq!(2, file.tell().unwrap());
    /// ```
    pub fn tell(&mut self) -> Result<u32> {
        let mut index = 0u32;
        unsafe { sdftry!(fwifc_tell(self.handle, &mut index)) }
        Ok(index)
    }

    /// Returns true if this file is indexed.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::File;
    /// let file = File::open("data/110630_174316.sdf").unwrap();
    /// file.indexed();
    /// ```
    pub fn indexed(&self) -> bool {
        metadata(&self.index_path).map(|m| m.is_file()).unwrap_or(false)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe {
            let result = fwifc_close(self.handle);
            if result != 0 {
                panic!("Error when closing file: {}", Error::from_i32(result));
            }
        }
    }
}

impl IntoIterator for File {
    type Item = Record;
    type IntoIter = FileIterator;
    fn into_iter(mut self) -> Self::IntoIter {
        self.reindex().unwrap();
        FileIterator { file: self }
    }
}

/// An iterator over a file.
///
/// Note that this iterator will panic on any underlying sdfifc library errors. If you need more
/// robust error handling, do the iteration yourself.
#[derive(Debug)]
pub struct FileIterator {
    file: File,
}

impl Iterator for FileIterator {
    type Item = Record;
    fn next(&mut self) -> Option<Self::Item> {
        match self.file.read() {
            Ok(record) => Some(record),
            Err(Error::EndOfFile(_)) => None,
            Err(err) => panic!("Error when iterating through the file: {}", err),
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
    /// The number of mirror facets in the intrument.
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
    Amplitude(Channel),
    /// A range calibration table.
    Range(Channel),
}

impl CalibrationTableKind {
    /// Returns this calibration table kind as a `u16`.
    ///
    /// Returns an error if the combination is not valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::{CalibrationTableKind, Channel};
    /// assert_eq!(0, CalibrationTableKind::Amplitude(Channel::High).as_u16().unwrap());
    /// assert_eq!(3, CalibrationTableKind::Range(Channel::Low).as_u16().unwrap());
    /// assert!(CalibrationTableKind::Amplitude(Channel::Saturation).as_u16().is_err());
    /// ```
    pub fn as_u16(&self) -> Result<u16> {
        match *self {
            CalibrationTableKind::Amplitude(channel) => {
                match channel {
                    Channel::High => Ok(0),
                    Channel::Low => Ok(1),
                    _ => Err(Error::NoCalibrationTableForChannel(channel)),
                }
            }
            CalibrationTableKind::Range(channel) => {
                match channel {
                    Channel::High => Ok(2),
                    Channel::Low => Ok(3),
                    _ => Err(Error::NoCalibrationTableForChannel(channel)),
                }
            }
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
    pub blocks: Vec<Block>,
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "time_sorg: {}\ntime_external: {}\norigin: {} {} {}\ndirection: {} {} \
                {}\nsynchronized: {}\nsync_lastsec: {}\nhousekeeping: {}\nfacet: {}\nnblocks: {}",
               self.time_sorg,
               self.time_external,
               self.origin[0],
               self.origin[1],
               self.origin[2],
               self.direction[0],
               self.direction[1],
               self.direction[2],
               self.synchronized,
               self.sync_lastsec,
               self.housekeeping,
               self.facet,
               self.blocks.len())
    }
}

/// A sample block.
#[derive(Debug)]
pub struct Block {
    /// The start of the sample block, in seconds.
    pub time_sosbl: f64,
    /// The channel: 0:high, 1:low, 2:saturation, 3:reference.
    pub channel: Channel,
    /// The actual data samples.
    pub samples: Vec<u16>,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f,
                    "time_sosbl: {}\nchannel: {}\nsamples: ",
                    self.time_sosbl,
                    self.channel));
        for (i, ref sample) in self.samples.iter().enumerate() {
            let mut seperator = ", ";
            if i == self.samples.len() - 1 {
                seperator = ""
            }
            try!(write!(f, "{}{}", sample, seperator));
        }
        Ok(())
    }
}

/// Information from one detector or set of detectors.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Channel {
    /// The high channel.
    High,
    /// The low channel.
    Low,
    /// The saturation channel.
    Saturation,
    /// The reference channel.
    Reference,
}

impl Channel {
    /// Returns the appropriate channel for the given u32.
    ///
    /// # Examples
    ///
    /// ```
    /// use sdf::file::Channel;
    /// assert_eq!(Channel::High, Channel::from_u32(0).unwrap());
    /// assert_eq!(Channel::Low, Channel::from_u32(1).unwrap());
    /// assert_eq!(Channel::Saturation, Channel::from_u32(2).unwrap());
    /// assert_eq!(Channel::Reference, Channel::from_u32(3).unwrap());
    /// assert!(Channel::from_u32(4).is_err());
    /// ```
    pub fn from_u32(n: u32) -> Result<Channel> {
        match n {
            0 => Ok(Channel::High),
            1 => Ok(Channel::Low),
            2 => Ok(Channel::Saturation),
            3 => Ok(Channel::Reference),
            _ => Err(Error::InvalidChannel(n)),
        }
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match *self {
            Channel::High => "high",
            Channel::Low => "low",
            Channel::Saturation => "saturation",
            Channel::Reference => "reference",
        };
        write!(f, "{}", name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::remove_file;

    #[test]
    fn open_throws_on_bad_filename() {
        assert!(File::open("notafile.sdf").is_err());
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
        let calib = file.calibration(CalibrationTableKind::Amplitude(Channel::High)).unwrap();
        assert_eq!(256, calib.abscissa.len());
        assert_eq!(calib.ordinate.len(), calib.abscissa.len());
    }

    #[test]
    fn smart_index() {
        remove_file("data/110630_174316.idx").unwrap_or(());
        {
            let mut file = File::open("data/110630_174316.sdf").unwrap();
            assert!(!file.indexed());
            file.reindex().unwrap();
        }
        let file = File::open("data/110630_174316.sdf").unwrap();
        assert!(file.indexed());
    }
}
