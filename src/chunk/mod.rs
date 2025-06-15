use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use zstd::stream::{Decoder, Encoder};

pub const MAGIC_VERSION: &[u8] = b"SQUISHRS01";

/// Write the header to a archive file
///
/// # arguments
///
/// * 'writer' - writer instance of the archive file
///
/// # returns
///
/// * 'std::io::Result<()>' - Error indicating issue writing to the file
///
/// # examples
///
/// ```
/// chunk::write_header(&mut writer);
/// ```
pub fn write_header<W: Write>(writer: &mut W) -> std::io::Result<()> {
    writer.write_all(MAGIC_VERSION)
}

/// Verify the header of an archive
///
/// # arguments
///
/// * 'reader' - reader instance of the archive file
///
/// # returns
///
/// * 'std::io::Result<()>' - Error indicating the archive header is invalid
///
/// # examples
///
/// ```
/// chunk::verify_header(&mut writer);
/// ```
pub fn verify_header<R: Read>(reader: &mut R) -> std::io::Result<()> {
    let mut header = [0u8; 9];
    reader.read_exact(&mut header)?;

    if &header != MAGIC_VERSION {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid archive header",
        ))
    } else {
        Ok(())
    }
}

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
        let mut encoder = Encoder::new(&mut compressed, 0)?;
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
