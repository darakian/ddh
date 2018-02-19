//Std imports
use std::io::Read;
use std::hash::Hash;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::thread;
use std::collections::HashSet;
use std::path::PathBuf;
use std::cmp::Ordering;
use std::fs::{self};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

//External imports
extern crate clap;
extern crate rayon;
use clap::{Arg, App};
use rayon::prelude::*;

#[derive(Debug)]
struct Fileinfo{
    file_hash: u64,
    file_len: u64,
    file_paths: HashSet<PathBuf>,
}

impl Fileinfo{
    fn new(hash: u64, length: u64, path: PathBuf) -> Self{
        let mut set = HashSet::<PathBuf>::new();
        set.insert(path);
        Fileinfo{file_hash: hash, file_len: length, file_paths: set}
    }
}

impl PartialEq for Fileinfo{
    fn eq(&self, other: &Fileinfo) -> bool {
        self.file_hash==other.file_hash
    }
}
impl Eq for Fileinfo{}

impl PartialOrd for Fileinfo{
    fn partial_cmp(&self, other: &Fileinfo) -> Option<Ordering>{
        self.file_hash.partial_cmp(&other.file_hash)
    }
}

impl Ord for Fileinfo{
    fn cmp(&self, other: &Fileinfo) -> Ordering {
        self.file_hash.cmp(&other.file_hash)
    }
}

impl Hash for Fileinfo{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.file_hash.hash(state);
    }
}

fn main() {
    let arguments = App::new("Directory Difference hTool")
                          .version("0.9.0")
                          .author("Jon Moroney jmoroney@cs.ru.nl")
                          .about("Compare and contrast directories.\nExample invocation: ddh /home/jon/downloads /home/jon/documents -p S")
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
                                .possible_values(&["single", "shared", "csv"])
                                .case_insensitive(true)
                                .takes_value(true)
                                .help("Print Single Instance or Shared Instance files.")
                            )
                          .get_matches();

    let blocksize = match arguments.value_of("Blocksize").unwrap_or(""){"B" => "Bytes", "K" => "Kilobytes", "M" => "Megabytes", "G" => "Gigabytes", _ => "Kilobytes"};
    let display_power = match blocksize{"Bytes" => 0, "Kilobytes" => 1, "Megabytes" => 2, "Gigabytes" => 3, _ => 1};
    let display_divisor =  1024u64.pow(display_power);
    let (sender, receiver) = channel();

    for arg in arguments.values_of("directories").unwrap().into_iter(){
        let arg_str = String::from(arg);
        let inner_sender = sender.clone();
        thread::spawn(move|| {
            traverse_and_spawn(Path::new(&arg_str), inner_sender.clone());
        });
    }
    drop(sender);
    let mut complete_files: Vec<Fileinfo> = Vec::<Fileinfo>::new();
    for entry in receiver.iter(){
        complete_files.push(entry);
    }

    complete_files.par_sort_unstable();
    complete_files.dedup_by(|a, b| if a==b {
        b.file_paths.extend(a.file_paths.drain());
        true
    } else {false});
    let shared_files: Vec<_> = complete_files.par_iter().filter(|x| x.file_paths.len()>1).collect();
    let unique_files: Vec<_> = complete_files.par_iter().filter(|x| x.file_paths.len()==1).collect();
    println!("{} Total files (with duplicates): {} {}", complete_files.iter().fold(0, |sum, x| sum+x.file_paths.len()), complete_files.iter().fold(0, |sum, x| sum+(x.file_len*x.file_paths.len() as u64))/display_divisor, blocksize);
    println!("{} Total files (without duplicates): {} {}", complete_files.len(), complete_files.iter().fold(0, |sum, x| sum+(x.file_len)/display_divisor), blocksize);
    println!("{} Single instance files: {} {}", unique_files.len(), unique_files.iter().fold(0, |sum, x| sum+(x.file_len)/display_divisor), blocksize);
    println!("{} Shared instance files: {} {} ({} instances)", shared_files.len(), shared_files.iter().fold(0, |sum, x| sum+(x.file_len)/display_divisor), blocksize, shared_files.iter().fold(0, |sum, x| sum+x.file_paths.len()));
    match arguments.value_of("Print").unwrap_or(""){
        "single" => {println!("Single instance files"); unique_files.iter().for_each(|x| println!("{}", x.file_paths.iter().next().unwrap().file_name().unwrap().to_str().unwrap()))},
        "shared" => {println!("Shared instance files and instances"); shared_files.iter().for_each(|x| {
            println!("{} instances:", x.file_paths.iter().next().unwrap().file_name().unwrap().to_str().unwrap());
            x.file_paths.par_iter().for_each(|y| println!("{} - {:x}", y.to_str().unwrap(), x.file_hash));
            println!("Total disk usage {} {}", ((x.file_paths.len() as u64)*x.file_len)/display_divisor, blocksize)})
        },
        "csv" => {unique_files.iter().for_each(|x| {
                println!("{}; {:x}", x.file_paths.iter().next().unwrap().canonicalize().unwrap().to_str().unwrap(), x.file_hash)});
            shared_files.iter().for_each(|x| {
                x.file_paths.par_iter().for_each(|y| println!("{}; {:x}", y.canonicalize().unwrap().to_str().unwrap(), x.file_hash));})
        },
        _ => {}};
}

fn hash_and_send(file_path: &Path, sender: Sender<Fileinfo>) -> (){
    let mut hasher = DefaultHasher::new();
    match fs::File::open(file_path) {
        Ok(f) => {
            let buffer_reader = BufReader::with_capacity(std::cmp::min(std::cmp::max(4096,(f.metadata().unwrap().len()/8)), 33554432) as usize, f);
            buffer_reader.bytes().for_each(|x| hasher.write(&[x.unwrap()]));
            sender.send(Fileinfo::new(hasher.finish(),file_path.metadata().unwrap().len(), file_path.to_path_buf())).unwrap();
        }
        Err(e) => {println!("Error:{} when opening {:?}. Skipping.", e, file_path);}
    }
}

fn traverse_and_spawn(current_path: &Path, sender: Sender<Fileinfo>) -> (){
    if current_path.is_file(){
        hash_and_send(current_path, sender.clone());
    } else if current_path.is_dir() {
        let paths: Vec<_> = fs::read_dir(current_path).unwrap().map(|r| r.unwrap()).collect();;
        paths.par_iter().for_each_with(sender.clone(), |s, dir_entry| {
            if dir_entry.path().is_dir(){
                    traverse_and_spawn(dir_entry.path().as_path(), s.clone());
                } else if dir_entry.path().is_file(){
                    hash_and_send(dir_entry.path().as_path(), s.clone());
                }
        });
    }
}
