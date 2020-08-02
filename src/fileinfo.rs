use serde::ser::{Serialize, Serializer, SerializeStruct};
use siphasher::sip128::Hasher128;
use std::hash::Hasher;
use std::path::PathBuf;
use std::cmp::Ordering;
use std::io::Read;
use std::fs;

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
    file_length: u64,
    pub(crate) file_paths: Vec<PathBuf>,
}

impl Fileinfo{
    /// Creates a new Fileinfo collection struct.
    ///
    /// # Examples
    /// ```
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    ///
    /// Fileinfo::new(
    ///         None,
    ///         None,
    ///         3,
    ///         Path::new("./foo/bar.txt").to_path_buf()
    ///         );
    /// ```
    pub fn new(full_hash: Option<u128>, partial_hash: Option<u128>, length: u64, path: PathBuf) -> Self{
        Fileinfo{full_hash: full_hash, partial_hash: partial_hash, file_length: length, file_paths: vec![path]}
    }
    /// Gets the length of the files in the current collection.
    ///
    /// # Examples
    /// ```
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    ///
    /// let fi = Fileinfo::new(None, None, 3, Path::new("./foo/bar.txt").to_path_buf());
    /// let len = fi.get_length();
    /// assert_eq!(3, len);
    /// ```
    pub fn get_length(&self) -> u64{
        self.file_length
    }
    /// Gets the hash of the full file if available.
    ///
    /// # Examples
    /// ```
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    ///
    /// let fi = Fileinfo::new(Some(123), None, 3, Path::new("./foo/bar.txt").to_path_buf());
    /// let f_hash = fi.get_full_hash();
    /// assert_eq!(Some(123), f_hash);
    /// ```
    pub fn get_full_hash(&self) -> Option<u128>{
        self.full_hash
    }
    pub(crate) fn set_full_hash(&mut self, hash: Option<u128>) -> (){
        self.full_hash = hash
    }
    /// Gets the hash of the partially read file if available.
    ///
    /// # Examples
    /// ```
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    ///
    /// let fi = Fileinfo::new(None, Some(123), 3, Path::new("./foo/bar.txt").to_path_buf());
    /// let p_hash = fi.get_partial_hash();
    /// assert_eq!(Some(123), p_hash);
    /// ```
    pub fn get_partial_hash(&self) -> Option<u128>{
        self.partial_hash
    }
    pub(crate) fn set_partial_hash(&mut self, hash: Option<u128>) -> (){
        self.partial_hash = hash
    }
    /// Gets a candidate name. This will be the name of the first file inserted into the collection and so can vary.
    ///
    /// # Examples
    /// ```
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    ///
    /// let fi = Fileinfo::new(None, None, 3, Path::new("./foo/bar.txt").to_path_buf());
    /// let some_name = fi.get_candidate_name();
    /// assert_eq!("bar.txt", some_name)
    /// ```
    pub fn get_candidate_name(&self) -> &str{
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
    /// Gets all paths in the current collection. This can be used to get the names of each file with the string `rsplit("/")` method.
    ///
    /// # Examples
    /// ```
    /// use std::path::Path;
    /// use ddh::fileinfo::Fileinfo;
    ///
    /// let fi = Fileinfo::new(None, None, 3, Path::new("./foo/bar.txt").to_path_buf());
    /// let all_files = fi.get_paths();
    /// assert_eq!(&vec![Path::new("./foo/bar.txt").to_path_buf()],
    ///            all_files);
    /// ```
    pub fn get_paths(&self) -> &Vec<PathBuf>{
        return &self.file_paths
    }

    pub fn generate_hash(&mut self, mode: HashMode) -> Option<u128>{
        let mut hasher = siphasher::sip128::SipHasher::new();
        match fs::File::open(
            self.file_paths
            .iter()
            .next()
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
                return Some(hasher.finish128().into());
            }
            Err(_e) => {
                return None
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
        state.serialize_field("file_length", &self.file_length)?;
        state.serialize_field("file_paths", &self.file_paths)?;
        state.end()
    }
}

impl PartialEq for Fileinfo{
    fn eq(&self, other: &Fileinfo) -> bool {
        (self.file_length==other.file_length)&&
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
            Some(self.file_length.cmp(&other.file_length))
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
            self.file_length.cmp(&other.file_length)
        }
    }
}
