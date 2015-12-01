//! Executable toolkit for working with sdf files.

extern crate docopt;
extern crate env_logger;
extern crate rustc_serialize;
extern crate sdf;

use std::process::exit;
use std::u32;

use docopt::Docopt;

use sdf::error::Error;

const USAGE: &'static str = "
Read and process .sdf files.

Usage:
    sdf convert <infile> <outfile>
    sdf info <infile> \
                             [--brief]
    sdf record <infile> <index>
    sdf (-h | --help)
    \
                             sdf --version

Options:
    -h --help   Show this screen.
    \
                             --version   Show sdf-rs and sdfifc library versions.
    --brief     \
                             Only provide file information from the header, do not inspect the \
                             file itself.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_brief: bool,
    flag_version: bool,
    arg_index: u32,
    arg_infile: String,
    arg_outfile: String,
    cmd_convert: bool,
    cmd_info: bool,
    cmd_record: bool,
}

fn error_exit(message: &str, err: Error) -> ! {
    println!("ERROR: {}: {}", message, err);
    exit(1);
}

#[cfg_attr(test, allow(dead_code))]
fn main() {
    env_logger::init().unwrap();
    let args: Args = Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());

    if args.flag_version {
        let library_version = sdf::library_version().unwrap_or_else(|e| {
            error_exit("Unable to get library version", e)
        });
        println!("      sdf-rs version: {}", env!("CARGO_PKG_VERSION"));
        println!("  sdfifc api version: {}.{}",
                 library_version.api_major,
                 library_version.api_minor);
        println!("sdfifc build version: {}", library_version.build_version);
        println!("    sdfifc build tag: {}", library_version.build_tag);
        exit(0);
    }

    if args.cmd_convert {
        sdf::File::convert(args.arg_infile, args.arg_outfile)
            .unwrap_or_else(|e| error_exit("Problem when converting file", e));
        exit(0);
    }

    let mut file = sdf::File::open(args.arg_infile.clone())
                       .unwrap_or_else(|e| error_exit("Unable to open file", e));
    if !args.flag_brief {
        file.reindex().unwrap_or_else(|e| error_exit("Unable to reindex file", e));
    }

    if args.cmd_info {
        let info = file.info().unwrap_or_else(|e| error_exit("Unable to retrieve file info", e));
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

        let record = file.read().unwrap_or_else(|e| error_exit("Unable to read first record", e));
        let start_time = record.time_external;
        println!("      start time: {}", start_time);
        file.seek(u32::MAX).unwrap_or_else(|e| error_exit("Unable to seek to end of file", e));
        let record = file.read().unwrap_or_else(|e| error_exit("Unable to read last record", e));
        let end_time = record.time_external;
        println!("        end time: {}", end_time);
        let records = file.tell()
                          .unwrap_or_else(|e| error_exit("Unable to get index of next record", e));
        println!("number of records: {}", records);

        exit(0);
    }

    if args.cmd_record {
        file.seek(args.arg_index)
            .unwrap_or_else(|e| {
                error_exit(&format!("Unable to seek to index {}", args.arg_index)[..],
                           e)
            });
        let record = file.read().unwrap_or_else(|e| error_exit("Unable to read record", e));
        println!("{}", record);
        for (i, block) in record.blocks.iter().enumerate() {
            println!("\nBlock {}", i);
            println!("{}", block);
        }
        exit(0);
    }

    unreachable!()
}
