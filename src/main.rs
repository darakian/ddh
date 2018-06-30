//Std imports
use std::io::{Read, Seek, SeekFrom, BufReader};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::collections::hash_map::{DefaultHasher, HashMap, Entry};
use std::cmp::Ordering;
use std::fs::{self, DirEntry};
use std::io::prelude::*;

//External imports
extern crate clap;
extern crate rayon;
extern crate stacker;
use clap::{Arg, App};
use rayon::prelude::*;

#[derive(Debug)]
struct Fileinfo{
    file_hash: u64,
    file_len: u64,
    file_paths: Vec<PathBuf>,
    mark_rehash: bool,
}

impl Fileinfo{
    fn new(hash: u64, length: u64, path: PathBuf) -> Self{
        let mut set = Vec::<PathBuf>::new();
        set.push(path);
        Fileinfo{file_hash: hash, file_len: length, file_paths: set, mark_rehash: false}
    }
}

impl PartialEq for Fileinfo{
    fn eq(&self, other: &Fileinfo) -> bool {
        (self.file_hash==other.file_hash)&&(self.file_len==other.file_len)
    }
}
impl Eq for Fileinfo{}

impl PartialOrd for Fileinfo{
    fn partial_cmp(&self, other: &Fileinfo) -> Option<Ordering>{
        self.file_len.partial_cmp(&other.file_len)
    }
}

impl Ord for Fileinfo{
    fn cmp(&self, other: &Fileinfo) -> Ordering {
        self.file_len.cmp(&other.file_len)
    }
}

impl Hash for Fileinfo{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.file_hash.hash(state);
    }
}

fn main() {
    let arguments = App::new("Directory Difference hTool")
                        .version("0.9.7")
                        .author("Jon Moroney jmoroney@hawaii.edu")
                        .about("Compare and contrast directories.\nExample invocation: ddh /home/jon/downloads /home/jon/documents -p shared")
                        .arg(Arg::with_name("directories")
                               .short("d")
                               .long("directories")
                               .value_name("Directories")
                               .help("Directories to parse")
                               .min_values(1)
                               .required(true)
                               .takes_value(true)
                               .index(1))
                        .arg(Arg::with_name("Blocksize")
                               .short("b")
                               .long("blocksize")
                               .case_insensitive(true)
                               .takes_value(true)
                               .max_values(1)
                               .possible_values(&["B", "K", "M", "G"])
                               .help("Sets the display blocksize to Bytes, Kilobytes, Megabytes or Gigabytes. Default is Kilobytes."))
                        .arg(Arg::with_name("Print")
                                .short("p")
                                .long("print")
                                .possible_values(&["single", "shared", "csv"])
                                .case_insensitive(true)
                                .takes_value(true)
                                .help("Print Single Instance or Shared Instance files."))
                        .arg(Arg::with_name("Output")
                                .short("o")
                                .long("out")
                                .takes_value(true)
                                .max_values(1)
                                .help("File to output to. Default is Results.txt"))
                        .get_matches();

    let blocksize = match arguments.value_of("Blocksize").unwrap_or(""){"B" => "Bytes", "K" => "Kilobytes", "M" => "Megabytes", "G" => "Gigabytes", _ => "Megabytes"};
    let display_power = match blocksize{"Bytes" => 0, "Kilobytes" => 1, "Megabytes" => 2, "Gigabytes" => 3, _ => 2};
    let out_file = arguments.value_of("Output").unwrap_or("Results.txt");
    let out_file = out_file.rsplit("/").next().unwrap_or("Results.txt");
    match fs::File::open(out_file) {
        Ok(_f) => { //File exists.
            println!("File {} already exists.\nPlease use a different output file. Exiting.", out_file);
            return
        },
        Err(_e) => {}, //File does not exist. Write away.
    }
    let display_divisor =  1024u64.pow(display_power);
    let (sender, receiver) = channel();
    let search_dirs: Vec<_> = arguments.values_of("directories").unwrap()
    .collect();

    //Search over user supplied directories
    search_dirs.par_iter().for_each_with(sender.clone(), |s, search_dir| {
        stacker::maybe_grow(32 * 1024, 1024 * 1024, || {
            traverse_and_spawn(Path::new(&search_dir), s.clone());
        });
    });
    drop(sender); //Drop sender so that reciever closes when all thread closures end
    //Collect Fileinfo entries in a HashMap of vectors. Each vector corrosponds to a specific flie length
    let mut files_of_lengths: HashMap<u64, Vec<Fileinfo>> = HashMap::new();
    for entry in receiver.iter(){
    match files_of_lengths.entry(entry.file_len) {
        Entry::Vacant(e) => { e.insert(vec![entry]); },
        Entry::Occupied(mut e) => { e.get_mut().push(entry); }
        }
    }

    //Compare them files
    let complete_files: Vec<Fileinfo> = files_of_lengths.into_par_iter().map(|x| //For each vector diff and compare on x.0 (length) and x.1 the vector
        differentiate_and_consolidate(x.0, x.1)
    ).flatten().collect();
    //Get duplicates and singletons
    let (shared_files, unique_files): (Vec<&Fileinfo>, Vec<&Fileinfo>) = complete_files.par_iter().partition(|&x| x.file_paths.len()>1);

    //Print main output
    println!("{} Total files (with duplicates): {} {}", complete_files.par_iter().map(|x| x.file_paths.len() as u64).sum::<u64>(),
    complete_files.par_iter().map(|x| (x.file_paths.len() as u64)*x.file_len).sum::<u64>()/(display_divisor),
    blocksize);
    println!("{} Total files (without duplicates): {} {}",
    complete_files.len(),
    (complete_files.par_iter().map(|x| x.file_len).sum::<u64>())/(display_divisor),
    blocksize);
    println!("{} Single instance files: {} {}",
    unique_files.len(),
    unique_files.par_iter().map(|x| x.file_len).sum::<u64>()/(display_divisor),
    blocksize);
    println!("{} Shared instance files: {} {} ({} instances)",
    shared_files.len(),
    shared_files.par_iter().map(|x| x.file_len).sum::<u64>()/(display_divisor),
    blocksize,
    shared_files.par_iter().map(|x| x.file_paths.len() as u64).sum::<u64>());
    write_results_to_file(&shared_files, &unique_files, out_file);

    //Print aux output
    match arguments.value_of("Print").unwrap_or(""){
        "single" => {println!("Single instance files"); unique_files.par_iter()
        .for_each(|x| println!("{}", x.file_paths.iter().next().unwrap().canonicalize().unwrap().to_str().unwrap()))},
        "shared" => {println!("Shared instance files and instances"); shared_files.iter().for_each(|x| {
            println!("instances of {:x} with file length {}:", x.file_hash, x.file_len);
            x.file_paths.par_iter().for_each(|y| println!("{:x}, {}", x.file_hash, y.canonicalize().unwrap().to_str().unwrap()));
            println!("Total disk usage {} {}", ((x.file_paths.len() as u64)*x.file_len)/display_divisor, blocksize)})
        },
        "csv" => {unique_files.par_iter().for_each(|x| {
                println!(/*"{:x}, */"{}, {}", x.file_paths.iter().next().unwrap().canonicalize().unwrap().to_str().unwrap(), x.file_len)});
            shared_files.iter().for_each(|x| {
                x.file_paths.par_iter().for_each(|y| println!(/*"{:x}, */"{}, {}", y.to_str().unwrap(), x.file_len));})
        },
        _ => {println!("Full results written to {}", out_file);}};
}

fn hash_and_update(input: &mut Fileinfo, skip_n_bytes: u64, pre_hash: bool) -> (){
    assert!(input.file_paths.iter().next().expect("Error reading path from struct").is_file());
    let mut hasher = DefaultHasher::new();
    match fs::File::open(input.file_paths.iter().next().expect("Error reading path")) {
        Ok(f) => {
            let mut buffer_reader = BufReader::new(f);
            buffer_reader.seek(SeekFrom::Start(skip_n_bytes)).expect("Error skipping bytes in second hash round");
            let mut hash_buffer = [0;4096];
            loop {
                match buffer_reader.read(&mut hash_buffer) {
                    Ok(n) if n>0 => hasher.write(&hash_buffer[0..]),
                    Ok(n) if n==0 => break,
                    Err(e) => println!("{:?} reading {:?}", e, input.file_paths.iter().next().expect("Error opening file for hashing")),
                    _ => println!("Should not be here"),
                    }
                if pre_hash{break}
            }
            input.file_hash=hasher.finish();
        }
        Err(e) => {println!("Error:{} when opening {:?}. Skipping.", e, input.file_paths.iter().next().expect("Error opening file for hashing"))}
    }
}

fn traverse_and_spawn(current_path: &Path, sender: Sender<Fileinfo>) -> (){
    if !current_path.exists(){
        return
    }

    if current_path.symlink_metadata().expect("Error getting Symlink Metadata").file_type().is_dir(){
        let mut paths: Vec<DirEntry> = Vec::new();
        match fs::read_dir(current_path) {
                Ok(read_dir_results) => read_dir_results.filter(|x| x.is_ok()).for_each(|x| paths.push(x.unwrap())),
                Err(e) => println!("Skipping {:?}. {:?}", current_path, e.kind()),
            }
        paths.into_par_iter().for_each_with(sender, |s, dir_entry| {
            stacker::maybe_grow(32 * 1024, 1024 * 1024, || {
                traverse_and_spawn(dir_entry.path().as_path(), s.clone());
            });
        });
    } else if current_path.symlink_metadata().expect("Error getting Symlink Metadata").file_type().is_file(){
        sender.send(Fileinfo::new(0, current_path.metadata().expect("Error with current path length").len(), /*fs::canonicalize(*/current_path.to_path_buf()/*).expect("Error canonicalizing path in struct creation.")*/)).expect("Error sending new fileinfo");
    } else {}
}

fn differentiate_and_consolidate(file_length: u64, mut files: Vec<Fileinfo>) -> Vec<Fileinfo>{
    if file_length==0 || files.len()==0{
        return files
    }
    match files.len(){
        1 => return files,
        n if n>1 => {
            //Hash stage one
            files.par_iter_mut().for_each(|file_ref| {
                hash_and_update(file_ref, 0, true);
            });
            files.par_sort_unstable_by(|a, b| b.file_hash.cmp(&a.file_hash)); //O(nlog(n))
            if file_length>4096 /*4KB*/ { //only hash again if we are not done hashing
                files.dedup_by(|a, b| if a==b{ //O(n)
                    a.mark_rehash=true;
                    b.mark_rehash=true;
                    false
                }else{false});
                files.par_iter_mut().filter(|x| x.mark_rehash==true).for_each(|file_ref| {
                    hash_and_update(file_ref, 4096, false); //Skip 4KB
                });
            }
        },
        _ => {println!("Somehow a vector of negative length got made. Please resport this as a bug");}
    }
    files.dedup_by(|a, b| if a==b{ //O(n)
        b.file_paths.extend(a.file_paths.drain(0..));
        //drop(a);
        true
    }else{false});
    files
}

fn write_results_to_file(shared_files: &Vec<&Fileinfo>, unique_files: &Vec<&Fileinfo>, file: &str) {
    let mut output = fs::File::create(file).expect("Error opening output file for writing");
    output.write(b"Multiple instance files files:\n").expect("Error writing results");
    for file in shared_files.into_iter(){
        let title = file.file_paths.get(0).unwrap().file_name().unwrap().to_str().unwrap();
        output.write_fmt(format_args!("{}", title)).unwrap();
        //output.write(b"\n").expect("Error writing results");
        for entry in file.file_paths.iter(){
            //output.write(b"\t").expect("Error writing results");
            output.write_fmt(format_args!("\t{}", entry.as_path().to_str().unwrap())).unwrap();
            //output.write(entry.as_path().to_str().unwrap().as_bytes()).expect("Error writing results");
            //output.write(b"\n").expect("Error writing results");

        }
    }

    output.write(b"Single instance files files:\n").expect("Error writing results");
    for file in unique_files.into_iter(){
        let title = file.file_paths.get(0).unwrap().file_name().unwrap().to_str().unwrap();
        output.write(title.as_bytes()).expect("Error writing results");
        output.write(b"\n").expect("Error writing results");
        for entry in file.file_paths.iter(){
            output.write(b"\t").expect("Error writing results");
            output.write(entry.as_path().to_str().unwrap().as_bytes()).expect("Error writing results");
            output.write(b"\n").expect("Error writing results");

        }
    }
}
