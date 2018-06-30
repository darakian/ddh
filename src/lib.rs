//Std imports
use std::hash::{Hash, Hasher};
use std::path::{PathBuf};
use std::cmp::Ordering;

#[derive(Debug)]
pub struct Fileinfo{
    file_hash: u64,
    file_len: u64,
    pub file_paths: Vec<PathBuf>,
    pub mark_rehash: bool,
}

impl Fileinfo{
    pub fn new(hash: u64, length: u64, path: PathBuf) -> Self{
        let mut set = Vec::<PathBuf>::new();
        set.push(path);
        Fileinfo{file_hash: hash, file_len: length, file_paths: set, mark_rehash: false}
    }

    pub fn len(&self) -> u64{
        self.file_len
    }

    pub fn set_hash(&mut self, value: u64) -> u64{
        self.file_hash = value;
        self.file_hash
    }

    pub fn get_hash(&self) -> u64{
        self.file_hash
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
