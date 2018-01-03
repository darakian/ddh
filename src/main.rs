use std::io::Read;
use std::hash::Hash;
use std::io::BufReader;
use std::path::Path;
use std::thread;
use std::collections::HashSet;
use std::path::PathBuf;
use std::cmp::Ordering;
use std::fs::{self};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

extern crate clap;
use clap::{Arg, App};

#[derive(Debug, Clone)]
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
                          .version("0.6.0")
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
                          // .arg(Arg::with_name("Hidden")
                          //      .short("h")
                          //      .long("hidden")
                          //      .possible_values(&["true", "false"])
                          //      .case_insensitive(true)
                          //      .help("Searches hidden folders. NOT YET IMPLEMENTED. CURRENTLY TRUE."))
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
    let mut directory_results = Vec::new();
    let mut thread_handles = Vec::new();
    for arg in arguments.values_of("directories").unwrap().into_iter(){
        let arg_str = String::from(arg);
        thread_handles.push(thread::spawn(move|| -> Vec<Fileinfo> {
            collect(Path::new(&arg_str), Vec::new())
        }));
    }
    for handle in thread_handles {
        directory_results.push(handle.join().unwrap());
    }
    let mut complete_files: Vec<Fileinfo> = directory_results.into_iter().fold(Vec::new(), |mut unifier, element| {unifier.extend(element); unifier});
    complete_files.sort_unstable();
    complete_files.dedup_by(|a, b| if a==b{
        //println!("{:?} Removed\n{:?} Retained", a, b);
        b.file_paths.extend(a.file_paths.drain());
        true
    } else {false});
    let shared_files: Vec<_> = complete_files.iter().filter(|x| x.file_paths.len()>1).collect();
    let unique_files: Vec<_> = complete_files.iter().filter(|x| x.file_paths.len()==1).collect();
    println!("{} Total files (with duplicates): {} {}", complete_files.iter().fold(0, |sum, x| sum+x.file_paths.len()), complete_files.iter().fold(0, |sum, x| sum+(x.file_len*x.file_paths.len() as u64))/display_divisor, blocksize);
    println!("{} Total files (without duplicates): {} {}", complete_files.len(), complete_files.iter().fold(0, |sum, x| sum+(x.file_len))/display_divisor, blocksize);
    println!("{} Single instance files: {} {}", unique_files.len(), unique_files.iter().fold(0, |sum, x| sum+x.file_len)/display_divisor, blocksize);
    println!("{} Shared instance files: {} {} ({} instances)", shared_files.len(), shared_files.iter().fold(0, |sum, x| sum+x.file_len)/display_divisor, blocksize, shared_files.iter().fold(0, |sum, x| sum+x.file_paths.len()));
    match arguments.value_of("Print").unwrap_or(""){
        "single" => {println!("Single instance files"); unique_files.iter().for_each(|x| println!("{}", x.file_paths.iter().next().unwrap().file_name().unwrap().to_str().unwrap()))},
        "shared" => {println!("Shared instance files and instances"); shared_files.iter().for_each(|x| {
            println!("{} instances:", x.file_paths.iter().next().unwrap().file_name().unwrap().to_str().unwrap());
            x.file_paths.iter().for_each(|y| println!("{} - {:x}", y.to_str().unwrap(), x.file_hash));
            println!("Total disk usage {} {}", ((x.file_paths.len() as u64)*x.file_len)/display_divisor, blocksize)})
        },
        _ => {}};
}

fn hash_file(file_path: &Path) -> Option<u64>{
    let mut hasher = DefaultHasher::new();
    match fs::File::open(file_path) {
        Ok(f) => {
            let buffer_reader = BufReader::with_capacity(1048576, f);
            buffer_reader.bytes().for_each(|x| hasher.write(&[x.unwrap()]));
            Some(hasher.finish())
        }
        Err(e) => {println!("Error:{} when opening {:?}. Skipping.", e, file_path); None}
    }
}

fn collect(current_dir: &Path, mut file_set: Vec<Fileinfo>) -> Vec<Fileinfo> {
    match fs::read_dir(current_dir) {
        Err(e) => println!("Reading directory {} has failed with error {:?}", current_dir.to_str().unwrap(), e.kind()),
        Ok(paths) => for entry in paths {
            let item =  match entry{
                Ok(v) => v,
                Err(e) => {println!("Error encountered reading from {:?}\n{:?}", current_dir, e.kind());continue}
            };
            if item.file_type().unwrap().is_dir(){
                file_set = collect(&item.path(), file_set);
            } else if item.file_type().unwrap().is_file(){
                let hash = match hash_file(&item.path()){
                    Some(v) => v,
                    None => {println!("Error encountered hashing {:?}. Skipping.", item.path());continue}
                };
                //println!("{:?}", item.path());
                file_set.push(Fileinfo{file_paths: vec![item.path()].into_iter().collect(), file_hash: hash, file_len: item.metadata().unwrap().len()});
            }
        }
    }
    file_set
}
