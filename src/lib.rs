use std::hash::{Hash, Hasher};
use std::fs::{self};
use std::io::{Read, BufReader};
use std::path::PathBuf;
use std::cmp::Ordering;
use serde_derive::{Serialize, Deserialize};
use siphasher::sip128::Hasher128;

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

#[derive(PartialEq)]
pub enum HashMode{
    Full,
    Partial
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Fileinfo{
    full_hash: Option<u128>,
    partial_hash: Option<u128>,
    file_length: u64,
    pub file_paths: Vec<PathBuf>,
}

impl Fileinfo{
    pub fn new(hash: Option<u128>, partial_hash: Option<u128>, length: u64, path: PathBuf) -> Self{
        let mut set = Vec::<PathBuf>::new();
        set.push(path);
        Fileinfo{full_hash: hash, partial_hash: partial_hash, file_length: length, file_paths: set}
    }
    pub fn get_length(&self) -> u64{
        self.file_length
    }
    pub fn get_full_hash(&self) -> Option<u128>{
        self.full_hash
    }
    pub fn set_full_hash(&mut self, hash: Option<u128>) -> (){
        self.full_hash = hash
    }
    pub fn set_partial_hash(&mut self, hash: Option<u128>) -> (){
        self.partial_hash = hash
    }
    pub fn get_partial_hash(&self) -> Option<u128>{
        self.partial_hash
    }
    pub fn get_file_name(&self) -> &str{
        self.file_paths
        .iter()
        .next()
        .unwrap()
        .to_str()
        .unwrap()
        .rsplit("/")
        .next()
        .unwrap()
    }

    pub fn generate_hash(&mut self, mode: HashMode) -> Option<u128>{
        let mut hasher = siphasher::sip128::SipHasher::new();
        match fs::File::open(
            self.file_paths
            .iter()
            .next()
            .expect("Cannot read file path from struct")
            ) {
            Ok(f) => {
                let mut buffer_reader = BufReader::new(f);
                let mut hash_buffer = [0;4096];
                loop {
                    match buffer_reader.read(&mut hash_buffer) {
                        Ok(n) if n>0 => hasher.write(&hash_buffer[0..]),
                        Ok(n) if n==0 => break,
                        Err(e) => {
                            println!("{:?} reading {:?}", e, 
                                self.file_paths
                                .iter()
                                .next()
                                .expect("Cannot read file path from struct"));
                            return None
                        },
                        _ => panic!("Negative length read in hashing"),
                        }
                    if mode == HashMode::Partial{
                        return Some(hasher.finish128().into());
                    }
                }
                return Some(hasher.finish128().into());
            }
            Err(e) => {
                println!("Error:{} when opening {:?}. Skipping.", e, 
                    self.file_paths
                    .iter()
                    .next()
                    .expect("Cannot read file path from struct"));
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
