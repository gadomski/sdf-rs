//! Convert sdf files to other formats.

use std::iter::repeat;
use std::path::Path;

use peakbag::{PeakDetector, Peak};
use sdc;

use Result;
use error::Error;
use file::{Block, Channel, File, FileInfo, Record};

const HIGH_WIDTH: usize = 2;
const HIGH_FLOOR: u16 = 15;
const HIGH_CEILING: u16 = 255;
const LOW_WIDTHS: (usize, usize) = (3, 2);
const LOW_FLOOR: u16 = 15;
const LOW_CEILINGS: (u16, u16) = (250, 255); // heh
const MIN_HEIGHT_ABOVE_BACKGROUND: f64 = 5.0;
const MAX_KURTOSIS: f64 = 0.04;

/// Turns a single sdf record into zero or more `Point`s.
///
/// At this point, we assume that the timestamps are absolute. TODO make this smarter to handle the
/// case when the user has called `File::set_sosbl_mode(SosblMode::Relative)`.
///
/// # Panics
///
/// Panics if there are zero or more than one reference blocks in the record.
///
/// # Examples
///
/// ```
/// use sdf::convert::discretize;
/// use sdf::file::File;
/// let mut file = File::open("data/110630_174316.sdf").unwrap();
/// let ref file_info = file.info().unwrap();
/// file.reindex().unwrap();
/// let ref record = file.read().unwrap();
/// let points = discretize(record, file_info).unwrap();
/// ```
pub fn discretize(record: &Record, file_info: &FileInfo) -> Result<Vec<Point>> {
    let mut high_blocks = Vec::new();
    let mut low_blocks = Vec::new();
    let mut reference_block = None;
    for block in &record.blocks {
        match block.channel {
            Channel::High => high_blocks.push(block),
            Channel::Low => low_blocks.push(block),
            Channel::Reference => {
                if reference_block.is_some() {
                    panic!("More than one reference block in this record");
                } else {
                    reference_block = Some(block);
                }
            }
            _ => {}
        }
    }
    let reference_block = reference_block.unwrap();
    let high_detector = PeakDetector::new(HIGH_WIDTH, HIGH_FLOOR, HIGH_CEILING)
                            .min_height_above_background(MIN_HEIGHT_ABOVE_BACKGROUND)
                            .max_kurtosis(MAX_KURTOSIS);
    let (low_width, low_ceiling) = match high_blocks.len() {
        0 => (LOW_WIDTHS.0, LOW_CEILINGS.0),
        _ => (LOW_WIDTHS.1, LOW_CEILINGS.1),
    };
    let low_detector = PeakDetector::new(low_width, LOW_FLOOR, low_ceiling)
                           .min_height_above_background(MIN_HEIGHT_ABOVE_BACKGROUND)
                           .max_kurtosis(MAX_KURTOSIS)
                           .saturation(LOW_CEILINGS.1);
    let reference_detector = high_detector;

    let reference_peaks = reference_detector.detect_peaks(&reference_block.samples[..]);
    if reference_peaks.len() != 1 {
        debug!("Could not get a single reference peak out of: {:?}",
               reference_block.samples);
        return Err(Error::NeedSingleReferencePeak(reference_peaks.len()));
    }
    let ref reference_peak = reference_peaks[0];

    let timestamp = |peak: &Peak<_>, block: &Block| {
        block.time_sosbl + peak.index as f64 * file_info.sampling_time
    };

    let t_ref = timestamp(reference_peak, &reference_block);

    let mut points = Vec::new();
    for (block, detector) in low_blocks.iter()
                                       .zip(repeat(low_detector))
                                       .chain(high_blocks.iter().zip(repeat(high_detector))) {
        let peaks = detector.detect_peaks(&block.samples[..]);
        let num_target = peaks.len();
        for (i, peak) in peaks.into_iter().enumerate() {
            let time = timestamp(&peak, block);
            let range = file_info.v_group / 2.0 * (time - t_ref);
            // x is straight out of the scanner, and the mirror pans it along the
            // z axis.
            let theta = (record.direction[2] / record.direction[0])
                            .atan()
                            .to_degrees();
            let point = Point {
                time: time - record.time_sorg + record.time_external,
                range: range as f32,
                theta: theta as f32,
                x: (record.origin[0] + record.direction[0] * range) as f32,
                y: (record.origin[1] + record.direction[1] * range) as f32,
                z: (record.origin[2] + record.direction[2] * range) as f32,
                target: (i + 1) as u8,
                num_target: num_target as u8,
                facet: record.facet,
                peak: peak,
                high_channel: block.channel == Channel::High,
            };
            points.push(point);
        }
    }

    Ok(points)
}

/// A 3D point in the scanner's own coordiante frame.
#[derive(Clone, Copy, Debug)]
pub struct Point {
    /// The time that this point was collected. Its reference frame depends on the time settings
    /// provided to the sdf library, which at this point are poorly defined.
    pub time: f64,
    /// The raw range to the point, which is different than the cartesian xyz distance.
    pub range: f32,
    /// The mirror scan angle in degrees.
    pub theta: f32,
    /// The x coordinate of the point in the scanner's own coordinate system.
    pub x: f32,
    /// The y coordinate of the point in the scanner's own coordinate system.
    pub y: f32,
    /// The z coordinate of the point in the scanner's own coordinate system.
    pub z: f32,
    /// The target number (1-indexed).
    pub target: u8,
    /// The total number of targets in this pulse.
    pub num_target: u8,
    /// The mirror facet used to reflect the laser energy.
    pub facet: u16,
    /// The raw peak information returned from `peakbag`.
    pub peak: Peak<u16>,
    /// Was this point collected on the high channel?
    pub high_channel: bool,
}

impl File {
    /// Writes all points from an .sdf file to an .sdc file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sdf::file::File;
    /// File::convert("110630_174316.sdf", "110630_174316.sdc").unwrap();
    /// ```
    pub fn convert<T: Into<Vec<u8>>, P: AsRef<Path>>(sdf_path: T, sdc_path: P) -> Result<()> {
        let mut sdf_file = try!(File::open(sdf_path));
        let ref file_info = try!(sdf_file.info());
        let mut sdc_file = try!(sdc::Writer::from_path(sdc_path));
        for (i, record) in sdf_file.into_iter().enumerate() {
            let points = match discretize(&record, file_info) {
                Ok(points) => points,
                Err(Error::NeedSingleReferencePeak(_)) => {
                    warn!("No reference peak detected for pulse {}, skipping", i);
                    continue;
                }
                Err(err) => return Err(err),
            };
            for p in points {
                let ref point = sdc::Point {
                    time: p.time,
                    range: p.range,
                    theta: p.theta,
                    x: p.x,
                    y: p.y,
                    z: p.z,
                    amplitude: p.peak.amplitude,
                    // FIXME actually calculate this value
                    width: 1,
                    target_type: sdc::TargetType::Peak,
                    // Riegl one-indexes its target counts. Bah.
                    target: p.target,
                    num_target: p.num_target,
                    // FIXME I have no idea what this means
                    rg_index: 1,
                    facet_number: p.facet as u8,
                    high_channel: p.high_channel,
                    class_id: None,
                    rho: None,
                    reflectance: None,
                };
                try!(sdc_file.write_point(point));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use file::File;

    #[test]
    fn first_point() {
        let mut file = File::open("data/110630_174316.sdf").unwrap();
        file.reindex().unwrap();
        let ref file_info = file.info().unwrap();
        let ref record = file.read().unwrap();
        let points = discretize(record, file_info).unwrap();
        assert_eq!(4, points.len());
        assert_eq!(409397.90336020273, points[0].time);
    }

    #[test]
    fn angles() {
        let mut file = File::open("data/110630_174316.sdf").unwrap();
        file.reindex().unwrap();
        let ref file_info = file.info().unwrap();
        for ref record in file.into_iter().take(10000) {
            for point in discretize(record, file_info).unwrap() {
                assert!((point.theta < 40.0) & (point.theta > -40.0),
                        "Theta: {}",
                        point.theta);
            }
        }
    }
}
