//! # ddh
//!
//! `ddh` is a collection of functions and structs to aid in analysing filesystem directories.

pub mod fileinfo;
use fileinfo::{Fileinfo, HashMode};

use std::fs::{self, DirEntry};
use std::path::{PathBuf, Path};
use rayon::prelude::*;
use std::sync::mpsc::{Sender, channel};
use std::collections::hash_map::{HashMap, Entry};
use std::io::{Error, ErrorKind};
use nohash_hasher::IntMap;

enum ChannelPackage{
    Success(Fileinfo),
    Fail(PathBuf, std::io::Error),
}

/// Constructs a list of unique files from a list of directories.
///
/// # Examples
/// ```
/// let directories = vec!["/home/jon", "/home/doe"];
/// let (files, errors) = ddh::deduplicate_dirs(directories);
/// ```
pub fn deduplicate_dirs<P: AsRef<Path> + Sync>(search_dirs: Vec<P>) -> (Vec<Fileinfo>, Vec<(PathBuf, std::io::Error)>){
    let (sender, receiver) = channel();
    search_dirs.par_iter().for_each_with(sender, |s, search_dir| {
            traverse_and_spawn(search_dir.as_ref(), s.clone());
    });
    let mut files_of_lengths: IntMap<u64, Vec<Fileinfo>> = IntMap::default();
    let mut errors = Vec::new();
    for pkg in receiver.iter(){
        match pkg{
            ChannelPackage::Success(entry) => {
                match files_of_lengths.entry(entry.get_length()) {
                    Entry::Vacant(e) => { e.insert(vec![entry]); },
                    Entry::Occupied(mut e) => { e.get_mut().push(entry); }
                }
            },
            ChannelPackage::Fail(entry, error) => {
                errors.push((entry, error));
            },
        }
    }
    let complete_files: Vec<Fileinfo> = files_of_lengths.into_par_iter()
        .map(|x| differentiate_and_consolidate(x.0, x.1))
        .flatten()
        .collect();
    (complete_files, errors)
}

fn traverse_and_spawn(current_path: &Path, sender: Sender<ChannelPackage>) -> (){
    let current_path_metadata = match fs::symlink_metadata(current_path) {
        Err(e) =>{
            sender.send(
            ChannelPackage::Fail(current_path.to_path_buf(), e)
            ).expect("Error sending new ChannelPackage::Fail");
            return
        },
        Ok(meta) => meta,
    };

    if current_path_metadata.file_type().is_symlink(){
        sender.send(
        ChannelPackage::Fail(current_path.to_path_buf(), Error::new(ErrorKind::Other, "Path is symlink"))
        ).expect("Error sending new ChannelPackage::Fail");
        return
    }

    if current_path_metadata.file_type().is_file(){
            sender.send(ChannelPackage::Success(
            Fileinfo::new(
                None,
                None,
                current_path.metadata().expect("Error reading path length").len(),
                current_path.to_path_buf()
                ))
            ).expect("Error sending new ChannelPackage::Success");
        return
    }

    if current_path_metadata.file_type().is_dir(){
        match fs::read_dir(current_path) {
                Ok(read_dir_results) => {
                    let good_entries: Vec<_> = read_dir_results
                    .filter(|x| x.is_ok())
                    .map(|x| x.unwrap())
                    .collect();
                    let (files, dirs): (Vec<&DirEntry>, Vec<&DirEntry>) = good_entries.par_iter().partition(|&x|
                        x.file_type()
                        .expect("Error reading DirEntry file type")
                        .is_file()
                        );
                    files.par_iter().for_each_with(sender.clone(), |sender, x|
                        sender.send(ChannelPackage::Success(
                            Fileinfo::new(
                                None,
                                None,
                                x.metadata().expect("Error reading path length").len(),
                                x.path()))
                                ).expect("Error sending new ChannelPackage::Success")
                            );
                    dirs.into_par_iter()
                    .for_each_with(sender, |sender, x| {
                        traverse_and_spawn(x.path().as_path(), sender.clone());
                    })
                },
                Err(e) => {
                    sender.send(
                        ChannelPackage::Fail(current_path.to_path_buf(), e)
                        ).expect("Error sending new ChannelPackage::Fail");
                },
            }
    } else {}
}

fn differentiate_and_consolidate(file_length: u64, mut files: Vec<Fileinfo>) -> Vec<Fileinfo>{
    if file_length==0{
        return files
    }
    if files.len()<=0{
        panic!("Invalid length vector");
    }
    match files.len(){
        1 => return files,
        n if n>1 => {
            files.par_iter_mut().for_each(|file_ref| {
                let hash = file_ref.generate_hash(HashMode::Partial);
                file_ref.set_partial_hash(hash);
            });
            if file_length<=4096{
                files.par_iter_mut().for_each(|x|{
                    x.set_full_hash(x.get_partial_hash()) ;
                });
                return dedupe(files)
            }
            let mut partial_hashes: HashMap<Option<u128>, u64> = HashMap::new();
            for file in files.iter(){
                match partial_hashes.entry(file.get_partial_hash()){
                    Entry::Vacant(e) => { e.insert(0); },
                    Entry::Occupied(mut e) => {*e.get_mut()+=1;}
                }
            }
            let dedupe_hashes: Vec<_> = partial_hashes
                .into_iter()
                .filter(|x| x.1>0)
                .map(|y| y.0)
                .collect();
            files.par_iter_mut().for_each(|x|
                if dedupe_hashes.contains(&x.get_partial_hash()){
                    let hash = x.generate_hash(HashMode::Full);
                    x.set_full_hash(hash);
                }
            );
        },
        _ => {panic!("Somehow a vector of negative length was created. Please report this as a bug");}
    }
    dedupe(files)
}

fn dedupe(mut files: Vec<Fileinfo>) -> Vec<Fileinfo>{
    let mut cache: HashMap<(Option<u128>, Option<u128>), &mut Fileinfo> = HashMap::new();
    for file in files.iter_mut(){
        match cache.entry((file.get_partial_hash(), file.get_full_hash())){
                    Entry::Vacant(e) => {
                        e.insert(file);
                    },
                    Entry::Occupied(mut e) => {
                        e.get_mut()
                        .file_paths
                        .append(&mut file.file_paths);
                    }
                }
    }
    files.retain(|x| x.get_paths().len()>0);
    files
}
