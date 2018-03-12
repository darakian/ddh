//Std imports
use std::io::Read;
use std::io::BufReader;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::collections::hash_map::DefaultHasher;
use std::cmp::Ordering;
use std::fs::{self, File};

//External imports
extern crate clap;
extern crate rayon;
extern crate flame;
use clap::{Arg, App};
use rayon::prelude::*;

#[derive(Debug)]
struct Fileinfo{
    file_hash: u64,
    file_len: u64,
    file_paths: Vec<PathBuf>,
}

impl Fileinfo{
    fn new(hash: u64, length: u64, path: PathBuf) -> Self{
        let mut set = Vec::<PathBuf>::new();
        set.push(path);
        Fileinfo{file_hash: hash, file_len: length, file_paths: set}
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
                        .version("0.9.3")
                        .author("Jon Moroney jmoroney@hawaii.edu")
                        .about("Compare and contrast directories.\nExample invocation: ddh /home/jon/downloads /home/jon/documents -p shared")
                        .arg(Arg::with_name("directories")
                               .short("d")
                               .long("directories")
                               .case_insensitive(true)
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
                                .possible_values(&["single", "shared", "csv", "stats"])
                                .case_insensitive(true)
                                .takes_value(true)
                                .help("Print Single Instance or Shared Instance files."))
                        .get_matches();

    let blocksize = match arguments.value_of("Blocksize").unwrap_or(""){"B" => "Bytes", "K" => "Kilobytes", "M" => "Megabytes", "G" => "Gigabytes", _ => "Kilobytes"};
    let display_power = match blocksize{"Bytes" => 0, "Kilobytes" => 1, "Megabytes" => 2, "Gigabytes" => 3, _ => 1};
    let display_divisor =  1024u64.pow(display_power);
    let (sender, receiver) = channel();
    let search_dirs: Vec<_> = arguments.values_of("directories").unwrap().collect();
    flame::start("Directory traversal");
    search_dirs.par_iter().for_each_with(sender.clone(), |s, search_dir| {
        traverse_and_spawn(Path::new(&search_dir), s.clone());
    });
    let mut complete_files: Vec<Fileinfo> = Vec::<Fileinfo>::new();
    drop(sender);
    flame::end("Directory traversal");
    flame::start("Collect file entries.");
    for entry in receiver.iter(){
        complete_files.push(entry);
    }
    flame::end("Collect file entries.");

    flame::start("Sort file entries by length.");
    complete_files.par_sort_unstable_by(|a, b| b.file_len.cmp(&a.file_len)); //O(nlog(n))
    flame::end("Sort file entries by length.");

    flame::start("Sweep and mark for hashing.");
    //Sweep and mark for hashing
    complete_files.dedup_by(|a, b| if a.file_len==b.file_len { //O(n)
        a.file_hash=1;
        b.file_hash=1;
        false
    } else {false});
    flame::end("Sweep and mark for hashing.");

    flame::start("Hash files of the same length.");
    //flame::span_of("database query", || query_database());
    complete_files.par_iter_mut().filter(|a| a.file_hash==1).for_each(|b| {flame::start("Hashing");hash_and_update(b);flame::end("Hashing");}); //O(n)
    flame::end("Hash files of the same length.");

    flame::start("Sort file entries by hash.");
    complete_files.par_sort_unstable_by(|a, b| b.file_hash.cmp(&a.file_hash)); //O(nlog(n))
    flame::end("Sort file entries by hash.");

    flame::start("Sweep and dedup by hash");
    complete_files.dedup_by(|a, b| if a==b{ //O(n)
        b.file_paths.extend(a.file_paths.drain(0..));
        true
    }else{false});
    //O(2nlog(n)+2n) :(
    flame::end("Sweep and dedup by hash");


    flame::start("Partition into duplicates and singletons.");
    let (shared_files, unique_files): (Vec<&Fileinfo>, Vec<&Fileinfo>) = complete_files.par_iter().partition(|&x| x.file_paths.len()>1);
    flame::end("Partition into duplicates and singletons.");

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

    match arguments.value_of("Print").unwrap_or(""){
        "single" => {println!("Single instance files"); unique_files.par_iter().for_each(|x| println!("{}", x.file_paths.iter().next().unwrap().file_name().unwrap().to_str().unwrap()))},
        "shared" => {println!("Shared instance files and instances"); shared_files.iter().for_each(|x| {
            println!("instances of {:x} with file length {}:", x.file_hash, x.file_len);
            x.file_paths.par_iter().for_each(|y| println!("{:x}, {}", x.file_hash, y.to_str().unwrap()));
            println!("Total disk usage {} {}", ((x.file_paths.len() as u64)*x.file_len)/display_divisor, blocksize)})
        },
        "csv" => {unique_files.par_iter().for_each(|x| {
                println!(/*"{:x}, */"{}, {}", x.file_paths.iter().next().unwrap().canonicalize().unwrap().to_str().unwrap(), x.file_len)});
            shared_files.iter().for_each(|x| {
                x.file_paths.par_iter().for_each(|y| println!(/*"{:x}, */"{}, {}", y.canonicalize().unwrap().to_str().unwrap(), x.file_len));})
        },
        "stats" => {flame::dump_stdout();},
        _ => {}};


}

fn hash_and_update(input: &mut Fileinfo) -> (){
    let mut hasher = DefaultHasher::new();
    match fs::File::open(input.file_paths.iter().next().expect("Error opening file for hashing")) {
        Ok(f) => {
            let mut buffer_reader = BufReader::new(f);
            let mut hash_buffer = [0;32768];
            //flame::start("Begin Hash loop");
            loop {
                match buffer_reader.read(&mut hash_buffer) {
                    Ok(n) if n>0 => hasher.write(&hash_buffer[0..n]),
                    Ok(n) if n==0 => break,
                    Err(e) => println!("{:?} reading {:?}", e, input.file_paths.iter().next().expect("Error opening file for hashing")),
                    _ => println!("Should not be here"),
                }
            }
            //flame::end("End Hash loop");
            input.file_hash=hasher.finish();
            assert_ne!(input.file_hash, 0);
        }
        Err(e) => {println!("Error:{} when opening {:?}. Skipping.", e, input.file_paths.iter().next().expect("Error opening file for hashing"))}
    }
}

fn traverse_and_spawn(current_path: &Path, sender: Sender<Fileinfo>) -> (){
    if current_path.is_dir(){
        let paths: Vec<_> = fs::read_dir(current_path).unwrap().map(|a| a.ok().expect("Unable to open directory for traversal")).collect();
        paths.par_iter().for_each_with(sender, |s, dir_entry| {
            traverse_and_spawn(dir_entry.path().as_path(), s.clone());
        });
    } else if current_path.is_file() {
        sender.send(Fileinfo::new(0, current_path.metadata().unwrap().len(), current_path.to_path_buf())).unwrap();
    } else {println!("Cannot open {:?}. Skipping.", current_path);}
}
