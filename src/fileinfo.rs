use serde::ser::{Serialize, Serializer, SerializeStruct};
use siphasher::sip128::Hasher128;
use std::hash::Hasher;
use std::path::PathBuf;
use std::cmp::Ordering;
use std::io::Read;
use std::fs::{self, Metadata};

const BLOCK_SIZE: usize = 4096;

#[derive(PartialEq)]
pub enum HashMode{
    Full,
    Partial
}

/// Serializable struct containing entries for a specific file. These structs will identify individual files as a collection of paths and associated hash and length data.
#[derive(Debug)]
pub struct Fileinfo{
    full_hash: Option<u128>,
    partial_hash: Option<u128>,
    metadata: Metadata,
    pub(crate) file_paths: Vec<PathBuf>,
}

impl Fileinfo{
    /// Creates a new Fileinfo collection struct.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    /// use std::fs;
    ///
    /// fn main() -> std::io::Result<()> {
    /// Fileinfo::new(
    ///         None,
    ///         None,
    ///         fs::metadata("./foo/bar.txt")?,
    ///         Path::new("./foo/bar.txt").to_path_buf()
    ///         );
    /// Ok(())
    /// }
    /// ```
    pub fn new(full: Option<u128>, partial: Option<u128>, meta: Metadata, path: PathBuf) -> Self{
        Fileinfo{full_hash: full, partial_hash: partial, metadata: meta, file_paths: vec![path]}
    }
    /// Gets the length of the files in the current collection.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    /// use std::fs;
    ///
    /// fn main() -> std::io::Result<()> {
    /// let fi = Fileinfo::new(None, None, fs::metadata("./foo/bar.txt")?, Path::new("./foo/bar.txt").to_path_buf());
    /// let len = fi.get_length();
    /// assert_eq!(3, len);
    /// Ok(())
    /// }
    /// ```
    pub fn get_length(&self) -> u64{
        self.metadata.len()
    }
    /// Gets the hash of the full file if available.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    /// use std::fs;
    ///
    /// fn main() -> std::io::Result<()> {
    /// let fi = Fileinfo::new(Some(123), None, fs::metadata("./foo/bar.txt")?, Path::new("./foo/bar.txt").to_path_buf());
    /// let f_hash = fi.get_full_hash();
    /// assert_eq!(Some(123), f_hash);
    /// Ok(())
    /// }
    /// ```
    pub fn get_full_hash(&self) -> Option<u128>{
        self.full_hash
    }
    pub(crate) fn set_full_hash(&mut self, hash: Option<u128>) {
        self.full_hash = hash
    }
    /// Gets the hash of the partially read file if available.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    /// use std::fs;
    ///
    /// fn main() -> std::io::Result<()> {
    /// let fi = Fileinfo::new(None, Some(123), fs::metadata("./foo/bar.txt")?, Path::new("./foo/bar.txt").to_path_buf());
    /// let p_hash = fi.get_partial_hash();
    /// assert_eq!(Some(123), p_hash);
    /// Ok(())
    /// }
    /// ```
    pub fn get_partial_hash(&self) -> Option<u128>{
        self.partial_hash
    }
    pub(crate) fn set_partial_hash(&mut self, hash: Option<u128>) {
        self.partial_hash = hash
    }
    /// Gets a candidate name. This will be the name of the first file inserted into the collection and so can vary.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    /// use std::fs;
    ///
    /// fn main() -> std::io::Result<()> {
    /// let fi = Fileinfo::new(None, None, fs::metadata("./foo/bar.txt")?, Path::new("./foo/bar.txt").to_path_buf());
    /// let some_name = fi.get_candidate_name();
    /// assert_eq!("bar.txt", some_name);
    /// Ok(())
    /// }
    /// ```
    pub fn get_candidate_name(&self) -> &str{
        self.file_paths
        .get(0)
        .unwrap()
        .to_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
    }
    /// Gets all paths in the current collection. This can be used to get the names of each file with the string `rsplit("/")` method.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    /// use std::fs;
    ///
    /// fn main() -> std::io::Result<()> {
    /// let fi = Fileinfo::new(None, None, fs::metadata("./foo/bar.txt")?, Path::new("./foo/bar.txt").to_path_buf());
    /// let all_files = fi.get_paths();
    /// assert_eq!(&vec![Path::new("./foo/bar.txt").to_path_buf()],
    ///            all_files);
    /// Ok(())
    /// }
    /// ```
    pub fn get_paths(&self) -> &Vec<PathBuf>{
        &self.file_paths
    }

    pub fn generate_hash(&mut self, mode: HashMode) -> Option<u128>{
        let mut hasher = siphasher::sip128::SipHasher::new();
        match fs::File::open(
            self.file_paths
            .get(0)
            .expect("Cannot read file path from struct")
            ) {
            Ok(mut f) => {
                /* We want a read call to be "large" for two reasons
                1) Force filesystem read ahead behavior
                2) Fewer system calls for a given file.
                Currently 16KB  */
                let mut hash_buffer = [0;BLOCK_SIZE * 4];
                loop {
                    match f.read(&mut hash_buffer) {
                        Ok(n) if n>0 => hasher.write(&hash_buffer),
                        Ok(n) if n==0 => break,
                        Err(_e) => {
                            return None
                        },
                        _ => panic!("Negative length read in hashing"),
                        }
                    if mode == HashMode::Partial{
                        return Some(hasher.finish128().into());
                    }
                }
                Some(hasher.finish128().into())
            }
            Err(_e) => {
                None
            }
        }
    }
}

impl Serialize for Fileinfo{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Fileinfo", 4)?;
        state.serialize_field("partial_hash", &self.partial_hash)?;
        state.serialize_field("full_hash", &self.full_hash)?;
        state.serialize_field("file_length", &self.get_length())?;
        state.serialize_field("file_paths", &self.file_paths)?;
        state.end()
    }
}

impl PartialEq for Fileinfo{
    fn eq(&self, other: &Fileinfo) -> bool {
        (self.get_length()==other.get_length())&&
        (self.partial_hash==other.partial_hash)&&
        (self.full_hash==other.full_hash)
    }
}
impl Eq for Fileinfo{}

impl PartialOrd for Fileinfo{
    fn partial_cmp(&self, other: &Fileinfo) -> Option<Ordering>{
         if self.full_hash.is_some() && other.full_hash.is_some(){
            Some(self.full_hash.cmp(&other.full_hash))
        } else if self.partial_hash.is_some() && other.partial_hash.is_some(){
            Some(self.partial_hash.cmp(&other.partial_hash))
        } else {
            Some(self.get_length().cmp(&other.get_length()))
        }
    }
}

impl Ord for Fileinfo{
    fn cmp(&self, other: &Fileinfo) -> Ordering {
        if self.full_hash.is_some() && other.full_hash.is_some(){
            self.full_hash.cmp(&other.full_hash)
        } else if self.partial_hash.is_some() && other.partial_hash.is_some(){
            self.partial_hash.cmp(&other.partial_hash)
        } else {
            self.get_length().cmp(&other.get_length())
        }
    }
}
