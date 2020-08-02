mod args;

use std::io::{stdin};
use std::fs::{self};
use std::io::prelude::*;
use rayon::prelude::*;
use ddh::{Fileinfo};
use std::path::{PathBuf};

#[derive(Debug, Copy, Clone)]
pub enum PrintFmt{
    Standard,
    Json,
    Off,
}

pub enum Verbosity{
    Quiet,
    Duplicates,
    All,
}

fn main() {
    let mut args = pico_args::Arguments::from_env();
    let args = args::Args {
        help: args.contains(["-h", "--help"]),
        verbosity: args.opt_value_from_str(["-v", "--verbosity"]).unwrap_or(None),
        blocksize: args.opt_value_from_str(["-bs", "--blocksize"]).unwrap_or(None),
        output: args.opt_value_from_str(["-o", "--output"]).unwrap_or(None),
        format: args.opt_value_from_str(["-f", "--format"]).unwrap_or(None),
        dirs: args.values_from_str(["-d", "--directories"]).unwrap_or(vec![]),
    };
    match args.help {
        true => {
            println!("Directory Difference hTool\n{}\n{}\n{}",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_AUTHORS"),
            args::example,);
        },
        false => {}
    }
    println!("Dirs: {:?}", args.dirs);
    println!("Verbosity: {:?}", args.verbosity);
    println!("Blocksize: {:?}", args.blocksize);
    println!("Format: {:?}", args.format);


    //let search_dirs: Vec<_> = args.dirs;

    // let search_dirs: Vec<_> = arguments.values_of("directories").unwrap()
    // .collect();

    let (complete_files, read_errors): (Vec<Fileinfo>, Vec<(_, _)>) = ddh::deduplicate_dirs(vec!["."]);
    let (shared_files, unique_files): (Vec<&Fileinfo>, Vec<&Fileinfo>) = complete_files.par_iter().partition(|&x| x.get_paths().len()>1);
    //process_full_output(&shared_files, &unique_files, &complete_files, &read_errors, &arguments);
}

fn process_full_output(shared_files: &Vec<&Fileinfo>, unique_files: &Vec<&Fileinfo>, complete_files: &Vec<Fileinfo>, error_paths: &Vec<(PathBuf, std::io::Error)>, arguments: args::Args) ->(){
    let blocksize = match arguments.blocksize.unwrap_or(args::Blocksize::Mega){args::Blocksize::Byte => "Bytes", args::Blocksize::Kilo => "Kilobytes", args::Blocksize::Mega => "Megabytes", args::Blocksize::Giga => "Gigabytes", _ => "Megabytes"};
    let display_power = match blocksize{"Bytes" => 0, "Kilobytes" => 1, "Megabytes" => 2, "Gigabytes" => 3, _ => 2};
    let display_divisor =  1024u64.pow(display_power);
    let fmt = arguments.format.unwrap_or(args::Format::Standard);
    let verbosity = arguments.verbosity.unwrap_or(args::Verbosity::Quiet);

    println!("{} Total files (with duplicates): {} {}", complete_files.par_iter()
    .map(|x| x.get_paths().len() as u64)
    .sum::<u64>(),
    complete_files.par_iter()
    .map(|x| (x.get_paths().len() as u64)*x.get_length())
    .sum::<u64>()/(display_divisor),
    blocksize);
    println!("{} Total files (without duplicates): {} {}", complete_files.len(), complete_files.par_iter()
    .map(|x| x.get_length())
    .sum::<u64>()/(display_divisor),
    blocksize);
    println!("{} Single instance files: {} {}",unique_files.len(), unique_files.par_iter()
    .map(|x| x.get_length())
    .sum::<u64>()/(display_divisor),
    blocksize);
    println!("{} Shared instance files: {} {} ({} instances)", shared_files.len(), shared_files.par_iter()
    .map(|x| x.get_length())
    .sum::<u64>()/(display_divisor),
    blocksize, shared_files.par_iter()
    .map(|x| x.get_paths().len() as u64)
    .sum::<u64>());

    match (&fmt, verbosity) {
        (_, args::Verbosity::Quiet) => {},
        (args::Format::Standard, args::Verbosity::Duplicates) => {
            println!("Shared instance files and instance locations"); shared_files.iter().for_each(|x| {
            println!("instances of {} with file length {}:", x.get_candidate_name(), x.get_length());
            x.get_paths().par_iter().for_each(|y| println!("\t{}", y.canonicalize().unwrap().to_str().unwrap()));})
        },
        (args::Format::Standard, args::Verbosity::All) => {
            println!("Single instance files"); unique_files.par_iter()
            .for_each(|x| println!("{}", x.get_paths().iter().next().unwrap().canonicalize().unwrap().to_str().unwrap()));
            println!("Shared instance files and instance locations"); shared_files.iter().for_each(|x| {
            println!("instances of {} with file length {}:", x.get_candidate_name(), x.get_length());
            x.get_paths().par_iter().for_each(|y| println!("\t{}", y.canonicalize().unwrap().to_str().unwrap()));});
            error_paths.iter().for_each(|x|{
                println!("Could not process {:#?} due to error {:#?}", x.0, x.1.kind());
            })
        },
        (args::Format::Json, args::Verbosity::Duplicates) => {
            println!("{}", serde_json::to_string(shared_files).unwrap_or("".to_string()));
        },
        (args::Format::Json, args::Verbosity::All) => {
            println!("{}", serde_json::to_string(complete_files).unwrap_or("".to_string()));
        },
        _ => {},
    }

    match arguments.output{
        None => {},
        Some(destination_string) => {
            match fs::File::open(&destination_string) {
                    Ok(_f) => {
                    println!("---");
                    println!("File {} already exists.", &destination_string);
                    println!("Overwrite? Y/N");
                    let mut input = String::new();
                    match stdin().read_line(&mut input) {
                        Ok(_n) => {
                            match input.chars().next().unwrap_or(' ') {
                                'n' | 'N' => {println!("Exiting."); return;}
                                'y' | 'Y' => {println!("Over writing {}", &destination_string);}
                                _ => {println!("Exiting."); return;}
                            }
                        }
                        Err(_e) => {println!("Error encountered reading user input. Err: {}", _e);},
                    }
                },
                Err(_e) => {
                    match fs::File::create(&destination_string) {
                        Ok(_f) => {},
                        Err(_e) => {
                            println!("Error encountered opening file {}. Err: {}", &destination_string, _e);
                            println!("Exiting."); return;
                        }
                    }
                },
            }
            write_results_to_file(&fmt, &shared_files, &unique_files, &complete_files, &destination_string);
        },
    }
}

fn write_results_to_file(fmt: &args::Format, shared_files: &Vec<&Fileinfo>, unique_files: &Vec<&Fileinfo>, complete_files: &Vec<Fileinfo>, file: &str) {
    let mut output = fs::File::create(file).expect("Error opening output file for writing");
    match fmt {
        args::Format::Standard => {
            output.write_fmt(format_args!("Duplicates:\n")).unwrap();
            for file in shared_files.into_iter(){
                let title = file.get_paths().get(0).unwrap().file_name().unwrap().to_str().unwrap();
                output.write_fmt(format_args!("{}\n", title)).unwrap();
                for entry in file.get_paths().iter(){
                    output.write_fmt(format_args!("\t{}\n", entry.as_path().to_str().unwrap())).unwrap();
                }
            }
            output.write_fmt(format_args!("Singletons:\n")).unwrap();
            for file in unique_files.into_iter(){
                let title = file.get_paths().get(0).unwrap().file_name().unwrap().to_str().unwrap();
                output.write_fmt(format_args!("{}\n", title)).unwrap();
                for entry in file.get_paths().iter(){
                    output.write_fmt(format_args!("\t{}\n", entry.as_path().to_str().unwrap())).unwrap();
                }
            }
        },
        args::Format::Json => {
            output.write_fmt(format_args!("{}", serde_json::to_string(complete_files).unwrap_or("Error deserializing".to_string()))).unwrap();
        },
        args::Format::None =>{return},
    }
    println!("Results written to {}", file);
}
