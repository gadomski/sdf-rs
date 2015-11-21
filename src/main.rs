//! Executable toolkit for working with sdf files.

extern crate docopt;
extern crate rustc_serialize;
extern crate sdf;

use std::process::exit;
use std::u32;

use docopt::Docopt;

const USAGE: &'static str = "
Read and process .sdf files.

Usage:
    sdf info <infile> \
                             [--brief]
    sdf (-h | --help)
    sdf --version

Options:
    -h \
                             --help   Show this screen.
    --version   Show sdf-rs and sdfifc \
                             library versions.
    --brief     Only provide file information from \
                             the header, do not inspect the file itself.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_brief: bool,
    flag_version: bool,
    arg_infile: String,
    cmd_info: bool,
}

#[cfg_attr(test, allow(dead_code))]
fn main() {
    let args: Args = Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());

    if args.flag_version {
        let library_version = sdf::library_version().unwrap_or_else(|e| {
            println!("ERROR: Unable to get library version: {}", e);
            exit(1);
        });
        println!("      sdf-rs version: {}", env!("CARGO_PKG_VERSION"));
        println!("  sdfifc api version: {}.{}",
                 library_version.api_major,
                 library_version.api_minor);
        println!("sdfifc build version: {}", library_version.build_version);
        println!("    sdfifc build tag: {}", library_version.build_tag);
        exit(0);
    } else if args.cmd_info {
        let mut file = sdf::File::open(args.arg_infile).unwrap_or_else(|e| {
            println!("ERROR: Unable to open file: {}", e);
            exit(1)
        });
        let info = file.info().unwrap_or_else(|e| {
            println!("ERROR: Unable to retrieve file info: {}", e);
            exit(1);
        });
        println!("      instrument: {}", info.instrument);
        println!("          serial: {}", info.serial);
        println!("           epoch: {}", info.epoch);
        println!("  group velocity: {}", info.v_group);
        println!("   sampling time: {}", info.sampling_time);
        println!("gps synchronized: {}", info.gps_synchronized);
        println!("number of facets: {}", info.num_facets);
        if args.flag_brief {
            exit(0);
        }

        file.reindex().unwrap_or_else(|e| {
            println!("ERROR: Unable to reindex file: {}", e);
            exit(1);
        });
        let record = file.read().unwrap_or_else(|e| {
            println!("ERROR: Unable to read first point: {}", e);
            exit(1);
        });
        let start_time = record.time_external;
        println!("      start time: {}", start_time);
        file.seek(u32::MAX).unwrap_or_else(|e| {
            println!("ERROR: Unable to seek to end of file: {}", e);
            exit(1);
        });
        let record = file.read().unwrap_or_else(|e| {
            println!("ERROR: Unable to read last point: {}", e);
            exit(1);
        });
        let end_time = record.time_external;
        println!("        end time: {}", end_time);
        let npoints = file.tell().unwrap_or_else(|e| {
            println!("ERROR: Unable to get index of next record: {}", e);
            exit(1);
        });
        println!("number of points: {}", npoints);

        exit(0);
    }
    unreachable!()
}
