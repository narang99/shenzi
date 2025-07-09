use anyhow::Result;
use blake3::Hasher;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

fn blake3_hash_file(path: &PathBuf) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Hasher::new();

    let mut buffer = [0u8; 8192];
    while let Ok(n) = reader.read(&mut buffer) {
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let hash = hasher.finalize();
    Ok(hash.to_hex().to_string())
}

pub fn make_digest(path: &PathBuf) -> Result<String> {
    blake3_hash_file(path)
}