use std::fs::File;
use std::io::{Read, self};

pub struct ChunkIter{
    f: File,
    chunk_len: usize,
}

impl ChunkIter{
    pub fn new(f: File, len: usize) -> Self{
        ChunkIter{f: f, chunk_len: len}
    }
}

impl Iterator for ChunkIter{
    type Item = Result<Vec<u8>, io::Error>;
    fn next(&mut self) -> Option<Result<Vec<u8>, io::Error>>{
        let mut buffer = Vec::with_capacity(self.chunk_len);
        match self.f.by_ref()
            .take(self.chunk_len as u64)
            .read_to_end(&mut buffer){
            Ok(i) => {
                if i == 0 {
                    return None
                } else {
                    Some(Ok(buffer))}
                },
            Err(e) => Some(Err(e))
        }
    }
}
