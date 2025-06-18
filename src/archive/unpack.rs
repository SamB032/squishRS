use super::header::verify_header;
use indicatif::ProgressBar;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};
use zstd::stream::decode_all;

/// Reads and decompresses all chunks from the archive into memory.
///
/// # Arguments
/// * `reader` - A mutable reference to any type implementing `Read`, from which chunk data will be read.
/// * `chunk_count` - Number of chunks to read.
///
/// # Returns
/// A HashMap mapping each chunk's 32-byte hash to its decompressed byte data.
///
/// # Errors
/// Returns an error if reading from `reader` fails or decompression fails.
fn read_chunks<R: Read>(
    reader: &mut R,
    chunk_count: u64,
    pb: Option<&ProgressBar>,
) -> Result<HashMap<[u8; 32], Vec<u8>>, Box<dyn std::error::Error>> {
    let mut buf8 = [0u8; 8];
    let mut chunk_map: HashMap<[u8; 32], Vec<u8>> = HashMap::new();

    for _ in 0..chunk_count {
        let mut hash = [0u8; 32];
        reader.read_exact(&mut hash)?;

        reader.read_exact(&mut buf8)?; // original size
        let _orig_size = u64::from_le_bytes(buf8);

        reader.read_exact(&mut buf8)?; // compressed size
        let compressed_size = u64::from_le_bytes(buf8);

        let mut compressed_data = vec![0u8; compressed_size as usize];
        reader.read_exact(&mut compressed_data)?;

        let decompressed = decode_all(&compressed_data[..])?;
        chunk_map.insert(hash, decompressed);

        // Increment progress bar if it exists
        if let Some(pb) = pb {
            pb.inc(1);
        }
    }

    Ok(chunk_map)
}

/// Reconstructs files from chunk data and writes them to the output directory.
///
/// # Arguments
/// * `reader` - A mutable reference to any type implementing `Read`, from which file metadata and chunk hashes are read.
/// * `file_count` - Number of files to reconstruct.
/// * `chunk_map` - A map from chunk hash to decompressed chunk data.
/// * `output_dir` - Directory path where reconstructed files will be written.
///
/// # Errors
/// Returns an error if file system operations or IO fail, or if required chunks are missing.
fn rebuild_file<R: Read>(
    reader: &mut R,
    file_count: u32,
    chunk_map: &HashMap<[u8; 32], Vec<u8>>,
    output_dir: &Path,
    pb: Option<&ProgressBar>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf4 = [0u8; 4];
    let mut buf8 = [0u8; 8];

    for _ in 0..file_count {
        // Read Path Length
        reader.read_exact(&mut buf4)?;
        let path_length = u32::from_le_bytes(buf4) as usize;

        // Get Full Path of File
        let mut path_bytes = vec![0u8; path_length];
        reader.read_exact(&mut path_bytes)?;
        let relative_path = String::from_utf8(path_bytes)?;
        let full_path = output_dir.join(PathBuf::from(&relative_path));

        // Read Original Size and Disgard
        reader.read_exact(&mut buf8)?;

        // Read Chunk Count
        reader.read_exact(&mut buf4)?;
        let chunk_count = u32::from_le_bytes(buf4);

        // Read chunk hashes
        let mut chunks = Vec::with_capacity(chunk_count as usize);
        for _ in 0..chunk_count {
            let mut hash = [0u8; 32];
            reader.read_exact(&mut hash)?;
            chunks.push(hash);
        }

        // Rebuilt the file
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut writer = BufWriter::new(File::create(&full_path)?);

        for hash in chunks {
            if let Some(data) = chunk_map.get(&hash) {
                writer.write_all(data)?;
            } else {
                return Err(format!("Missing chunk for file: {}", relative_path).into());
            }
        }

        // Increment progress bar if it exists
        if let Some(pb) = pb {
            pb.inc(1);
        }
    }

    Ok(())
}

pub fn unpack_squish(
    squish_path: &Path,
    output_dir: &Path,
    pb: Option<&mut ProgressBar>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(squish_path)?;
    let mut reader = BufReader::new(file);

    // Check magic header
    verify_header(&mut reader)?;

    // Read Timestamp and Disregard
    let mut buf8 = [0u8; 8];
    reader.read_exact(&mut buf8)?;

    // Read number of chunks
    reader.read_exact(&mut buf8)?;
    let chunk_count = u64::from_le_bytes(buf8);

    if let Some(pb) = pb.as_ref() {
        pb.set_length(chunk_count);
    }

    // Read all chunks into memory
    let chunk_map = read_chunks(&mut reader, chunk_count, pb.as_deref())?;

    // Read File Count
    let mut buf4 = [0u8; 4];
    reader.read_exact(&mut buf4)?;
    let file_count = u32::from_le_bytes(buf4);

    // Reset progress bar length to total chunks + files, preserving progress done
    if let Some(pb) = pb.as_ref() {
        pb.set_length(file_count as u64);
        pb.set_message("Rebuilding files");
        pb.set_position(0);
    }

    // Rebuild file
    rebuild_file(
        &mut reader,
        file_count,
        &chunk_map,
        output_dir,
        pb.as_deref(),
    )?;

    Ok(())
}
