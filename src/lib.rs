#[macro_use]
extern crate serde_derive;

//Std imports
use std::hash::{Hash, Hasher};
use std::path::{PathBuf};
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
    pub file_hash: u64,
    file_length: u64,
    pub file_paths: Vec<PathBuf>,
    pub second_hash: bool,
}

impl Fileinfo{
    pub fn new(hash: u64, length: u64, path: PathBuf) -> Self{
        let mut set = Vec::<PathBuf>::new();
        set.push(path);
        Fileinfo{file_hash: hash, file_length: length, file_paths: set, second_hash: false}
    }

    pub fn get_length(&self) -> u64{
        self.file_length
    }

    pub fn get_hash() -> u64{
        self.file_hash
    }
}

impl PartialEq for Fileinfo{
    fn eq(&self, other: &Fileinfo) -> bool {
        (self.file_hash==other.file_hash)&&(self.file_length==other.file_length)
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
        self.file_hash.hash(state);
    }
}
