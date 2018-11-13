#[macro_use]
extern crate serde_derive;

//Std imports
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::fs::{self};
use std::io::{Read, BufReader};
use std::path::PathBuf;
use std::cmp::Ordering;

extern crate serde;
extern crate serde_json;


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

#[derive(Debug, Serialize, Deserialize)]
pub struct Fileinfo{
    full_hash: Option<u64>,
    partial_hash: Option<u64>,
    file_length: u64,
    pub file_paths: Vec<PathBuf>,
}

impl Fileinfo{
    pub fn new(hash: Option<u64>, partial_hash: Option<u64>, length: u64, path: PathBuf) -> Self{
        let mut set = Vec::<PathBuf>::new();
        set.push(path);
        Fileinfo{full_hash: hash, partial_hash: partial_hash, file_length: length, file_paths: set}
    }
    pub fn get_length(&self) -> u64{
        self.file_length
    }
    pub fn get_full_hash(&self) -> Option<u64>{
        self.full_hash
    }
    pub fn set_full_hash(&mut self, hash: u64) -> (){
        self.full_hash = Some(hash)
    }
    pub fn get_partial_hash(&self) -> Option<u64>{
        self.full_hash
    }
    pub fn get_file_name(&self) -> &str{ //Gets the first file name. More useful than a hash value as an identifier.
        self.file_paths.iter().next().unwrap().to_str().unwrap().rsplit("/").next().unwrap()
    }

    pub fn generate_partial_hash(&mut self) -> Option<u64>{
        let mut hasher = DefaultHasher::new();
        match fs::File::open(self.file_paths.iter().next().expect("Error reading path")) {
            Ok(f) => {
                let mut buffer_reader = BufReader::new(f);
                let mut hash_buffer = [0;4096];
                match buffer_reader.read(&mut hash_buffer) {
                    Ok(n) if n>0 => hasher.write(&hash_buffer[0..]),
                    Ok(n) if n==0 => return None,
                    Err(_e) => return None,
                    _ => return None,
                    }
                self.partial_hash = Some(hasher.finish());
            }
            Err(_e) => return None,
        }
        return self.get_partial_hash()
    }

    pub fn generate_hash(&mut self) -> Option<u64>{
        let mut hasher = DefaultHasher::new();
        match fs::File::open(self.file_paths.iter().next().expect("Error reading path")) {
            Ok(f) => {
                let mut buffer_reader = BufReader::new(f);
                let mut hash_buffer = [0;4096];
                loop {
                    match buffer_reader.read(&mut hash_buffer) {
                        Ok(n) if n>0 => hasher.write(&hash_buffer[0..]),
                        Ok(n) if n==0 => break,
                        Err(e) => println!("{:?} reading {:?}", e, self.file_paths.iter().next().expect("Error opening file for hashing")),
                        _ => println!("Should not be here"),
                        }
                }
                self.set_full_hash(hasher.finish());
                return self.get_full_hash()
            }
            Err(e) => {
                println!("Error:{} when opening {:?}. Skipping.", e, self.file_paths.iter().next().expect("Error opening file for hashing"));
                return None
            }
        }
    }
}

impl PartialEq for Fileinfo{
    fn eq(&self, other: &Fileinfo) -> bool {
        (self.file_length==other.file_length)&&(self.partial_hash==other.partial_hash)&&(self.full_hash==other.full_hash)
    }
}
impl Eq for Fileinfo{}

impl PartialOrd for Fileinfo{
    fn partial_cmp(&self, other: &Fileinfo) -> Option<Ordering>{
        self.file_length.partial_cmp(&other.file_length)
    }
}

impl Ord for Fileinfo{
    fn cmp(&self, other: &Fileinfo) -> Ordering {
        self.file_length.cmp(&other.file_length)
    }
}

impl Hash for Fileinfo{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.full_hash.hash(state);
    }
}
