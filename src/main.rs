//! Executable toolkit for working with sdf files.

extern crate docopt;
extern crate rustc_serialize;
extern crate sdf;

use std::process::exit;

use docopt::Docopt;

const USAGE: &'static str = "
Read and process .sdf files.

Usage:
    sdf info <infile>
    sdf \
                             (-h | --help)
    sdf --version

Options:
    -h --help           \
                             Show this screen.
    --version           Show sdf-rs and sdfifc \
                             library versions.
";

#[derive(Debug, RustcDecodable)]
struct Args {
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
        exit(0);
    }
    unreachable!()
}
