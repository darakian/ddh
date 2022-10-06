use clap::{Parser, ValueEnum};
use ddh::fileinfo::Fileinfo;
use rayon::prelude::*;
use std::fs::{self};
use std::io::prelude::*;
use std::io::stdin;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about=DDH_ABOUT)]
struct Args {
    /// Minimum file size in bytes to consider
    #[arg(short, long("minimum"), num_args(0..=1), default_value_t = 0)]
    min_size: u64,
    /// Set the display blocksize to Bytes, Kilobytes, Megabytes or Gigabytes
    #[arg(long, short, ignore_case(true), value_enum, num_args(0..=1), default_value_t = Blocksize::Kilobytes)]
    blocksize: Blocksize,
    /// Set verbosity for printed output
    #[arg(long, short, ignore_case(true), value_enum, num_args(0..=1), default_value_t = Verbosity::Quiet)]
    verbosity: Verbosity,
    ///Set file to save all output. Use 'no' for no file output
    #[arg(long, short, num_args(0..=1), default_value = "Results.txt")]
    output: String,
    /// Set output format
    #[arg(short('f'), long("format"), ignore_case(true), value_enum, num_args(0..=1), default_value_t = PrintFmt::Standard)]
    fmt: PrintFmt,
    /// Directories to ignore (comma separated list)
    #[arg(short, long("ignore"), value_delimiter(','))]
    ignore_dirs: Vec<String>,
    /// Directories to parse
    #[arg(value_parser, required = true)]
    directories: Vec<String>,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum PrintFmt {
    Standard,
    Json,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum Verbosity {
    Quiet,
    Duplicates,
    All,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum Blocksize {
    #[clap(name("B"), alias("Bytes"))]
    Bytes,
    #[clap(name("K"), alias("Kilobytes"))]
    Kilobytes,
    #[clap(name("M"), alias("Megabytes"))]
    Megabytes,
    #[clap(name("G"), alias("Gigabytes"))]
    Gigabytes,
}

static DDH_ABOUT: &str = "Compare and contrast directories.\nExample invocation: ddh -d /home/jon/downloads /home/jon/documents -v duplicates\nExample pipe: ddh -d ~/Downloads/ -o no -v all -f json | someJsonParser.bin";

fn main() {

    let arguments = Args::parse();

    let (complete_files, read_errors): (Vec<Fileinfo>, Vec<(_, _)>) =
        ddh::deduplicate_dirs(arguments.directories, arguments.ignore_dirs, arguments.min_size);
    let (shared_files, unique_files): (Vec<&Fileinfo>, Vec<&Fileinfo>) = complete_files
        .par_iter()
        .partition(|&x| x.get_paths().len() > 1);
    process_full_output(
        &shared_files,
        &unique_files,
        &complete_files,
        &read_errors,
        arguments.output.as_str(),
        arguments.blocksize,
        arguments.fmt,
        arguments.verbosity,
    );
}

fn process_full_output(
    shared_files: &[&Fileinfo],
    unique_files: &[&Fileinfo],
    complete_files: &[Fileinfo],
    error_paths: &[(PathBuf, std::io::Error)],
    output: &str,
    blocksize: Blocksize,
    fmt: PrintFmt,
    verbosity: Verbosity,
) {
    let display_power = match blocksize {
        Blocksize::Bytes => 0,
        Blocksize::Kilobytes => 1,
        Blocksize::Megabytes => 2,
        Blocksize::Gigabytes => 3,
    };
    let display_divisor = 1024u64.pow(display_power);

    println!(
        "{} Total files (with duplicates): {} {:?}",
        complete_files
            .par_iter()
            .map(|x| x.get_paths().len() as u64)
            .sum::<u64>(),
        complete_files
            .par_iter()
            .map(|x| (x.get_paths().len() as u64) * x.get_length())
            .sum::<u64>()
            / (display_divisor),
        blocksize
    );
    println!(
        "{} Total files (without duplicates): {} {:?}",
        complete_files.len(),
        complete_files
            .par_iter()
            .map(|x| x.get_length())
            .sum::<u64>()
            / (display_divisor),
        blocksize
    );
    println!(
        "{} Single instance files: {} {:?}",
        unique_files.len(),
        unique_files.par_iter().map(|x| x.get_length()).sum::<u64>() / (display_divisor),
        blocksize
    );
    println!(
        "{} Shared instance files: {} {:?} ({} instances)",
        shared_files.len(),
        shared_files.par_iter().map(|x| x.get_length()).sum::<u64>() / (display_divisor),
        blocksize,
        shared_files
            .par_iter()
            .map(|x| x.get_paths().len() as u64)
            .sum::<u64>()
    );

    match (fmt, verbosity) {
        (_, Verbosity::Quiet) => {}
        (PrintFmt::Standard, Verbosity::Duplicates) => {
            println!("Shared instance files and instance locations");
            shared_files.iter().for_each(|x| {
                println!(
                    "instances of {} with file length {}:",
                    x.get_candidate_name(),
                    x.get_length()
                );
                x.get_paths()
                    .par_iter()
                    .for_each(|y| println!("\t{}", y.canonicalize().unwrap().to_str().unwrap()));
            })
        }
        (PrintFmt::Standard, Verbosity::All) => {
            println!("Single instance files");
            unique_files.par_iter().for_each(|x| {
                println!(
                    "{}",
                    x.get_paths()
                        .iter()
                        .next()
                        .unwrap()
                        .canonicalize()
                        .unwrap()
                        .to_str()
                        .unwrap()
                )
            });
            println!("Shared instance files and instance locations");
            shared_files.iter().for_each(|x| {
                println!(
                    "instances of {} with file length {}:",
                    x.get_candidate_name(),
                    x.get_length()
                );
                x.get_paths()
                    .par_iter()
                    .for_each(|y| println!("\t{}", y.canonicalize().unwrap().to_str().unwrap()));
            });
            error_paths.iter().for_each(|x| {
                println!(
                    "Could not process {:#?} due to error {:#?}",
                    x.0,
                    x.1.kind()
                );
            })
        }
        (PrintFmt::Json, Verbosity::Duplicates) => {
            println!(
                "{}",
                serde_json::to_string(shared_files).unwrap_or_else(|_| "".to_string())
            );
        }
        (PrintFmt::Json, Verbosity::All) => {
            println!(
                "{}",
                serde_json::to_string(complete_files).unwrap_or_else(|_| "".to_string())
            );
        }
    }

    match output {
        "no" => {}
        destination_string => {
            match fs::File::open(destination_string) {
                Ok(_f) => {
                    println!("---");
                    println!("File {} already exists.", destination_string);
                    println!("Overwrite? Y/N");
                    let mut input = String::new();
                    match stdin().read_line(&mut input) {
                        Ok(_n) => match input.chars().next().unwrap_or(' ') {
                            'n' | 'N' => {
                                println!("Exiting.");
                                return;
                            }
                            'y' | 'Y' => {
                                println!("Over writing {}", destination_string);
                            }
                            _ => {
                                println!("Exiting.");
                                return;
                            }
                        },
                        Err(_e) => {
                            println!("Error encountered reading user input. Err: {}", _e);
                        }
                    }
                }
                Err(_e) => match fs::File::create(destination_string) {
                    Ok(_f) => {}
                    Err(_e) => {
                        println!(
                            "Error encountered opening file {}. Err: {}",
                            destination_string, _e
                        );
                        println!("Exiting.");
                        return;
                    }
                },
            }
            write_results_to_file(
                fmt,
                shared_files,
                unique_files,
                complete_files,
                destination_string,
            );
        }
    }
}

fn write_results_to_file(
    fmt: PrintFmt,
    shared_files: &[&Fileinfo],
    unique_files: &[&Fileinfo],
    complete_files: &[Fileinfo],
    file: &str,
) {
    let mut output = fs::File::create(file).expect("Error opening output file for writing");
    match fmt {
        PrintFmt::Standard => {
            output.write_fmt(format_args!("Duplicates:\n")).unwrap();
            for file in shared_files.iter() {
                let title = file.get_candidate_name();
                output.write_fmt(format_args!("{}\n", title)).unwrap();
                for entry in file.get_paths().iter() {
                    output
                        .write_fmt(format_args!("\t{}\n", entry.as_path().to_str().unwrap()))
                        .unwrap();
                }
            }
            output.write_fmt(format_args!("Singletons:\n")).unwrap();
            for file in unique_files.iter() {
                let title = file.get_candidate_name();
                output.write_fmt(format_args!("{}\n", title)).unwrap();
                for entry in file.get_paths().iter() {
                    output
                        .write_fmt(format_args!("\t{}\n", entry.as_path().to_str().unwrap()))
                        .unwrap();
                }
            }
        }
        PrintFmt::Json => {
            output
                .write_fmt(format_args!(
                    "{}",
                    serde_json::to_string(complete_files)
                        .unwrap_or_else(|_| "Error deserializing".to_string())
                ))
                .unwrap();
        }
    }
    println!("{:#?} results written to {}", fmt, file);
}
