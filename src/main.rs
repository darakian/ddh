//Std imports
use std::io::Read;
use std::hash::Hash;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::collections::HashSet;
use std::path::PathBuf;
use std::cmp::Ordering;
use std::fs::{self};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

//External imports
extern crate clap;
extern crate threadpool;
extern crate rayon;
extern crate num_cpus;
use clap::{Arg, App};
use rayon::prelude::*;
use threadpool::*;

#[derive(Debug)]
struct Fileinfo{
    file_hash: u64,
    file_len: u64,
    file_paths: HashSet<PathBuf>,
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
                          .version("0.8.0")
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
                               .possible_values(&["K", "M", "G"])
                               .help("Sets the display blocksize to Kilobytes, Megabytes or Gigabytes. Default is Bytes."))
                          // .arg(Arg::with_name("Max threads")
                          //      .short("t")
                          //      .long("threads")
                          //      .default_value(10)
                          //      .possible_values(&[2..100])
                          //      .help("Set the maximum number of threads."))
                          .arg(Arg::with_name("Print")
                                .short("p")
                                .long("print")
                                .possible_values(&["single", "shared"])
                                .case_insensitive(true)
                                .takes_value(true)
                                .help("Print Single Instance or Shared Instance files.")
                            )
                          .get_matches();

    let display_power = match arguments.value_of("Blocksize").unwrap_or(""){"K" => 1, "M" => 2, "G" => 3, _ => 0};
    let blocksize = match arguments.value_of("Blocksize").unwrap_or(""){"K" => "Kilobytes", "M" => "Megabytes", "G" => "Gigabytes", _ => "Bytes"};
    let display_divisor =  1024u64.pow(display_power);
    let thread_count = num_cpus::get()*4;
    //println!("thread_count = {:?}", thread_count);
    let pool = ThreadPool::new(thread_count);
    let (sender, receiver) = channel();
    let mut directory_vectors = Vec::<Vec<Fileinfo>>::new();
    for arg in arguments.values_of("directories").unwrap().into_iter(){
        let arg_str = String::from(arg);
        let inner_sender = sender.clone();
        let inner_pool = pool.clone();
        pool.execute(move|| {
            inner_sender.send(collect_files(Path::new(&arg_str), Vec::new(), inner_pool, inner_sender.clone())).unwrap();
        });
    }
    pool.join();
    loop {
        let receiver_value = receiver.try_recv();
        if receiver_value.is_err() {break;}
        else {
            directory_vectors.push(receiver_value.unwrap())
        }
    }
    let mut complete_files: Vec<Fileinfo> = directory_vectors.into_iter().fold(Vec::new(), |mut unifier, element| {unifier.extend(element); unifier});
    complete_files.sort_unstable();
    complete_files.dedup_by(|a, b| if a==b {
        b.file_paths.extend(a.file_paths.drain());
        true
    } else {false});
    let shared_files: Vec<_> = complete_files.par_iter().filter(|x| x.file_paths.len()>1).collect();
    let unique_files: Vec<_> = complete_files.par_iter().filter(|x| x.file_paths.len()==1).collect();
    println!("{} Total files (with duplicates): {} {}", complete_files.iter().fold(0, |sum, x| sum+x.file_paths.len()), complete_files.iter().fold(0, |sum, x| sum+(x.file_len*x.file_paths.len() as u64))/display_divisor, blocksize);
    println!("{} Total files (without duplicates): {} {}", complete_files.len(), complete_files.iter().fold(0, |sum, x| sum+(x.file_len))/display_divisor, blocksize);
    println!("{} Single instance files: {} {}", unique_files.len(), unique_files.iter().fold(0, |sum, x| sum+x.file_len)/display_divisor, blocksize);
    println!("{} Shared instance files: {} {} ({} instances)", shared_files.len(), shared_files.iter().fold(0, |sum, x| sum+x.file_len)/display_divisor, blocksize, shared_files.iter().fold(0, |sum, x| sum+x.file_paths.len()));
    match arguments.value_of("Print").unwrap_or(""){
        "single" => {println!("Single instance files"); unique_files.iter().for_each(|x| println!("{}", x.file_paths.iter().next().unwrap().file_name().unwrap().to_str().unwrap()))},
        "shared" => {println!("Shared instance files and instances"); shared_files.iter().for_each(|x| {
            println!("{} instances:", x.file_paths.iter().next().unwrap().file_name().unwrap().to_str().unwrap());
            x.file_paths.par_iter().for_each(|y| println!("{} - {:x}", y.to_str().unwrap(), x.file_hash));
            println!("Total disk usage {} {}", ((x.file_paths.len() as u64)*x.file_len)/display_divisor, blocksize)})
        },
        _ => {}};
}

fn hash_file(file_path: &Path) -> Option<u64>{
    let mut hasher = DefaultHasher::new();
    //println!("Checking {:?}", file_path);
    match fs::File::open(file_path) {
        Ok(f) => {
            //let buffer_reader = BufReader::with_capacity(1048576, f);
            let buffer_reader = BufReader::with_capacity(std::cmp::min(std::cmp::max(4096,(f.metadata().unwrap().len()/8)), 33554432) as usize, f);
            buffer_reader.bytes().for_each(|x| hasher.write(&[x.unwrap()]));
            Some(hasher.finish())
        }
        Err(e) => {println!("Error:{} when opening {:?}. Skipping.", e, file_path); None}
    }
}

fn collect_files(current_path: &Path, mut file_set: Vec<Fileinfo>, pool: ThreadPool, collection_sender: Sender<Vec<Fileinfo>>) -> Vec<Fileinfo> {
    //println!("Entering {:?}", current_path.to_str());
    if current_path.is_dir(){
        match fs::read_dir(current_path) {
            Err(e) => println!("Reading directory {} has failed with error {:?}", current_path.to_str().unwrap_or("current_path unwrap error"), e.kind()),
            Ok(paths) => for entry in paths {
                //println!("Path entry = {:?}", entry);
                match entry{
                    Ok(item) => {if item.file_type().ok().unwrap().is_dir(){
                        let inner_pool = pool.clone();
                        let inner_sender = collection_sender.clone();
                        pool.execute(move|| {
                            inner_sender.send(collect_files(Path::new(&item.path()), Vec::new(), inner_pool, inner_sender.clone())).unwrap();
                        });
                    } else if item.file_type().ok().unwrap().is_file(){
                        //println!("Hashing {:?}", current_path.to_str());
                        match hash_file(&item.path()){
                            Some(hash_val) => {file_set.push(Fileinfo{file_paths: vec![item.path()].into_par_iter().collect(), file_hash: hash_val, file_len: item.metadata().unwrap().len()})},
                            None => {println!("Error encountered hashing {:?}. Skipping.", item.path())}
                            };
                        }
                    },
                    Err(e) => {println!("Error encountered reading from {:?}\n{:?}", current_path, e.kind())}
                };
            }
        }
    }else if current_path.is_file(){
        //println!("Hashing {:?}", current_path.to_str());
        match hash_file(&current_path){
            Some(hash_val) => {file_set.push(Fileinfo{file_paths: vec![current_path.to_path_buf()].into_par_iter().collect(), file_hash: hash_val, file_len: current_path.metadata().unwrap().len()})},
            None => {println!("Error encountered hashing {:?}. Skipping.", current_path)}
        };
    };
    //println!("Exiting {:?}", current_path.to_str());
    file_set
}
