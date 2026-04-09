//! BLAKE3 hash computation

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use crate::Result;

/// Chunk size for streaming hash computation (64KB)
const HASH_CHUNK_SIZE: usize = 64 * 1024;

/// Compute BLAKE3 hash of a file using streaming
pub fn hash_file(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(HASH_CHUNK_SIZE, file);
    let mut hasher = blake3::Hasher::new();

    let mut buffer = [0u8; HASH_CHUNK_SIZE];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Compute hashes for all files in a directory
pub fn hash_files_in_dir(dir: &Path, progress_cb: impl Fn(&Path, u64)) -> Result<std::collections::HashMap<std::path::PathBuf, String>> {
    use walkdir::WalkDir;
    let mut hashes = std::collections::HashMap::new();

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let size = path.metadata()?.len();
            progress_cb(path, size);
            let hash = hash_file(path)?;
            hashes.insert(path.to_path_buf(), hash);
        }
    }

    Ok(hashes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_hash_file() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"hello world").unwrap();
        temp.flush().unwrap();

        let hash = hash_file(temp.path()).unwrap();
        assert_eq!(hash.len(), 64); // BLAKE3 outputs 256 bits = 64 hex chars
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_hash_consistency() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"test content").unwrap();
        temp.flush().unwrap();

        let hash1 = hash_file(temp.path()).unwrap();
        let hash2 = hash_file(temp.path()).unwrap();
        assert_eq!(hash1, hash2);
    }
}