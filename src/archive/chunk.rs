use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use zstd::stream::{Decoder, Encoder};

pub const CHUNK_SIZE: usize = 512 * 1024; // 1KB
const COMPRESS_LEVEL: i32 = 12;

/// Calculates the hash of a binary array
///
/// # arguments
///
/// * 'data' - binary array
///
/// # returns
///
/// * '[u8;32]' - 32 bit hash
///
/// # examples
///
/// ```
/// chunk::hash_chunk(&chunk_buf);
/// ```
pub fn hash_chunk(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash_arr = [0u8; 32];
    hash_arr.copy_from_slice(&result);
    hash_arr
}

/// Compress a chunk of data
///
/// # arguments
///
/// * 'data' - buffer representing a chunk
///
/// # returns
///
/// * 'std::io::Result<Vec<u8>>' - compressed chunk or error indicating issue with encoding the chunk
///
/// # examples
///
/// ```
/// chunk::compress_chunk(&chunk_buf);
/// ```
pub fn compress_chunk(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut compressed = Vec::new();
    {
        let mut encoder = Encoder::new(&mut compressed, COMPRESS_LEVEL)?;
        encoder.write_all(data)?;
        encoder.finish()?;
    }
    Ok(compressed)
}

/// Decompress a chunk of data
///
/// # arguments
///
/// * 'data' - buffer representing a chunk
///
/// # returns
///
/// * 'std::io::Result<Vec<u8>>' - compressed chunk or error indicating issue with decoding the chunk
///
/// # examples
///
/// ```
/// chunk::decompress_chunk(&chunk_buf);
/// ```
pub fn decompress_chunk(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = Decoder::new(data)?;
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}
