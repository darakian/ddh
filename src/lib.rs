//! # ddh
//!
//! `ddh` is a collection of functions and structs to aid in analysing filesystem directories.

use std::hash::{Hash, Hasher};
use std::fs::{self};
use std::io::{Read, BufReader};
use std::path::{PathBuf, Path};
use std::cmp::Ordering;
use serde_derive::{Serialize, Deserialize};
use siphasher::sip128::Hasher128;
use rayon::prelude::*;
use std::sync::mpsc::{Sender, channel};
use std::collections::hash_map::{HashMap, Entry};

#[derive(PartialEq)]
enum HashMode{
    Full,
    Partial
}

enum ChannelPackage{
    Success(Fileinfo),
    Fail(PathBuf), 
}

/// Serializable struct containing entries for a specific file.
#[derive(Debug, Serialize, Deserialize)]
pub struct Fileinfo{
    full_hash: Option<u128>,
    partial_hash: Option<u128>,
    file_length: u64,
    file_paths: Vec<PathBuf>,
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
    fn set_full_hash(&mut self, hash: Option<u128>) -> (){
        self.full_hash = hash
    }
    fn set_partial_hash(&mut self, hash: Option<u128>) -> (){
        self.partial_hash = hash
    }
    pub fn get_partial_hash(&self) -> Option<u128>{
        self.partial_hash
    }
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
    pub fn get_paths(&self) -> &Vec<PathBuf>{
        return &self.file_paths
    }

    fn generate_hash(&mut self, mode: HashMode) -> Option<u128>{
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
        (self.file_length==other.file_length)&&
        (self.partial_hash==other.partial_hash)&&
        (self.full_hash==other.full_hash)
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

/// Constructs a list of unique files from a list of directories.
///
/// # Examples
/// ```
/// let directories = vec!["/home/jon", "/home/doe"];
/// let (files, errors) = ddh::deduplicate_dirs(directories);
/// ```
pub fn deduplicate_dirs(search_dirs: Vec<&str>) -> (Vec<Fileinfo>, Vec<PathBuf>){
    let (sender, receiver) = channel();
    search_dirs.par_iter().for_each_with(sender, |s, search_dir| {
        stacker::maybe_grow(32 * 1024, 1024 * 1024, || {
            traverse_and_spawn(Path::new(&search_dir), s.clone());
        });
    });
    let mut files_of_lengths: HashMap<u64, Vec<Fileinfo>> = HashMap::new();
    let mut errors = Vec::new();
    for pkg in receiver.iter(){
        match pkg{
            ChannelPackage::Success(entry) => {
                match files_of_lengths.entry(entry.get_length()) {
                    Entry::Vacant(e) => { e.insert(vec![entry]); },
                    Entry::Occupied(mut e) => { e.get_mut().push(entry); }
                }
            },
            ChannelPackage::Fail(entry) => {
                errors.push(entry);
            },
        }
    }
    let complete_files: Vec<Fileinfo> = files_of_lengths.into_par_iter()
        .map(|x|differentiate_and_consolidate(x.0, x.1))
        .flatten()
        .collect();
    (complete_files, errors)
}

fn traverse_and_spawn(current_path: &Path, sender: Sender<ChannelPackage>) -> (){
    if !current_path.exists(){
        return
    }
    if current_path.symlink_metadata().expect("Error reading Symlink Metadata").file_type().is_dir(){
        match fs::read_dir(current_path) {
                Ok(read_dir_results) => read_dir_results
                .filter(|x| x.is_ok())
                .for_each(|x| {
                    stacker::maybe_grow(32 * 1024, 1024 * 1024, || {
                        traverse_and_spawn(x.unwrap().path().as_path(), sender.clone());
                    })
                }),
                Err(e) => {
                    println!("Skipping {:?}. {:?}", current_path, e.kind());
                    sender.send(
                        ChannelPackage::Fail(current_path.to_path_buf())
                        ).expect("Error sending new cpkg::fail");
                },
            }
    } else if current_path
    .symlink_metadata()
    .expect("Error reading Symlink Metadata")
    .file_type()
    .is_file(){
        sender.send(ChannelPackage::Success(
            Fileinfo::new(
                None,
                None,
                current_path.metadata().expect("Error reading path length").len(),
                current_path.to_path_buf()
                ))
            ).expect("Error sending new cpkg::success");
    } else {}
}

fn differentiate_and_consolidate(file_length: u64, mut files: Vec<Fileinfo>) -> Vec<Fileinfo>{
    if file_length==0 || files.len()==0{
        return files
    }
    match files.len(){
        1 => return files,
        n if n>1 => {
            files.par_iter_mut().for_each(|file_ref| {
                let hash = file_ref.generate_hash(HashMode::Partial);
                file_ref.set_partial_hash(hash);
            });
            files.par_sort_unstable_by(|a, b| b.get_partial_hash().cmp(&a.get_partial_hash()));
            if file_length>4096 /*4KB*/ { //only hash again if we are not done hashing
                files.dedup_by(|a, b| if a==b{
                    a.set_full_hash(Some(1));
                    b.set_full_hash(Some(1));
                    false
                }else{false});
                files.par_iter_mut().filter(|x| x.get_full_hash().is_some()).for_each(|file_ref| {
                    let hash = file_ref.generate_hash(HashMode::Full);
                    file_ref.set_full_hash(hash);
                });
            }
        },
        _ => {panic!("Somehow a vector of negative length was created. Please report this as a bug");}
    }
    files.dedup_by(|a, b| if a==b{
        b.file_paths.extend(a.file_paths.drain(0..));
        true
    }else{false});
    files
}
