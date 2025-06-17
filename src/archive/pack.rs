use super::chunk::{hash_chunk, ChunkStore, CHUNK_SIZE};
use super::header::{write_header, write_timestamp};
use indicatif::ProgressBar;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// Writes all unique chunks from the `ChunkStore` to the writer.
///
/// The format written is:
/// - Number of unique chunks (u64, little-endian)
/// - For each chunk:
///   - 32-byte chunk hash
///   - Original size (u64, little-endian)
///   - Compressed size (u64, little-endian)
///   - Compressed chunk data
///
/// # Arguments
///
/// * `writer` - A mutable writer to output the chunks.
/// * `chunk_store` - The store containing unique chunks.
///
/// # Errors
///
/// Returns an error if writing to the writer fails.
fn write_chunks<W: Write>(
    writer: &mut W,
    chunk_store: &ChunkStore,
) -> Result<(), Box<dyn std::error::Error>> {
    let unique_chunk_count = chunk_store.primary_store.len() as u64;
    writer.write_all(&unique_chunk_count.to_le_bytes())?;

    for (chunk_hash, (compressed_data, orig_size)) in &chunk_store.primary_store {
        writer.write_all(chunk_hash)?;
        writer.write_all(&orig_size.to_le_bytes())?;
        let compressed_size = compressed_data.len() as u64;
        writer.write_all(&compressed_size.to_le_bytes())?;
        writer.write_all(compressed_data)?;
    }
    Ok(())
}

/// Writes metadata about each file in the archive to the writer.
///
/// For each file, the following is written:
/// - Path length (u32, little-endian)
/// - Path bytes (UTF-8)
/// - Original file size (u64, little-endian)
/// - Number of chunks for this file (u32, little-endian)
/// - List of 32-byte chunk hashes.
///
/// # Arguments
///
/// * `writer` - A mutable writer to output the file metadata.
/// * `files_metadata` - A slice of tuples containing:
///   - File path as String
///   - Original file size (u64)
///   - Vector of chunk hashes (`[u8; 32]`)
///
/// # Errors
///
/// Returns an error if writing to the writer fails.
fn write_files_metadata<W: Write>(
    writer: &mut W,
    files_metadata: &[(String, u64, Vec<[u8; 32]>)],
) -> Result<(), Box<dyn std::error::Error>> {
    let file_count = files_metadata.len() as u32;
    writer.write_all(&file_count.to_le_bytes())?;

    for (path, orig_size, chunk_hashes) in files_metadata {
        let path_bytes = path.as_bytes();
        let path_len = path_bytes.len() as u32;

        writer.write_all(&path_len.to_le_bytes())?;
        writer.write_all(path_bytes)?;
        writer.write_all(&orig_size.to_le_bytes())?;

        let chunk_count = chunk_hashes.len() as u32;
        writer.write_all(&chunk_count.to_le_bytes())?;

        for hash in chunk_hashes {
            writer.write_all(hash)?;
        }
    }

    Ok(())
}

/// Processes a single file by reading its contents in chunks, hashing
/// and inserting each chunk into the `ChunkStore`, and collecting
/// the chunk hashes to build metadata.
///
/// # Arguments
///
/// * `file_path` - The full path to the file to process.
/// * `input_dir` - The root input directory, used to compute relative paths.
/// * `chunk_store` - The mutable chunk store where chunks are stored.
///
/// # Returns
///
/// Returns a tuple containing:
/// - The file path relative to `input_dir` as a String.
/// - The original file size in bytes (u64).
/// - A vector of chunk hashes (`[u8; 32]`) for this file.
///
/// # Errors
///
/// Returns an error if reading the file or inserting chunks fails.
fn process_file(
    file_path: &Path,
    input_dir: &Path,
    chunk_store: &mut ChunkStore,
) -> Result<(String, u64, Vec<[u8; 32]>), Box<dyn std::error::Error>> {
    let rel_path = file_path.strip_prefix(input_dir)?;
    let rel_path_str = rel_path.to_string_lossy();

    let file = fs::File::open(file_path)?;
    let metadata = file.metadata()?;
    let orig_file_size = metadata.len();

    let mut reader = BufReader::new(file);
    let mut file_chunk_hashes = Vec::new();

    loop {
        let mut chunk_buf = vec![0u8; CHUNK_SIZE];
        let bytes_read = reader.read(&mut chunk_buf)?;
        if bytes_read == 0 {
            break;
        }
        chunk_buf.truncate(bytes_read);

        // Insert chunk via ChunkStore
        chunk_store.insert(&chunk_buf)?;

        // Calculate chunk hash and store it for the file metadata
        let chunk_hash = hash_chunk(&chunk_buf);
        file_chunk_hashes.push(chunk_hash);
    }

    Ok((rel_path_str.to_string(), orig_file_size, file_chunk_hashes))
}

/// Packs the given list of files into a single archive file,
/// writing the header, timestamp, chunks, and file metadata.
///
/// Also tracks and returns the overall compression ratio percentage.
///
/// # Arguments
///
/// * `input_dir` - The root directory of input files (used for relative paths).
/// * `output_file` - The output archive file path.
/// * `files` - A list of file paths to include in the archive.
/// * `pb` - A progress bar instance to show progress.
///
/// # Returns
///
/// Returns the reduction percentage as a `f64` representing
/// how much the data was compressed.
///
/// # Errors
///
/// Returns an error if any file operation or writing fails.
pub fn pack_squish(
    input_dir: &Path,
    output_file: &Path,
    files: &[PathBuf],
    pb: &ProgressBar,
) -> Result<f64, Box<dyn std::error::Error>> {
    // Track overall compression ratio
    let mut total_orig_size: u64 = 0;
    let mut total_compressed_size: u64 = 0;

    // Open output writer
    let output = fs::File::create(output_file)?;
    let mut writer = BufWriter::new(output);

    write_header(&mut writer)?;
    write_timestamp(&mut writer)?;

    let mut chunk_store = ChunkStore::new();
    let mut files_metadata = Vec::new();

    for file_path in files {
        let (rel_path_str, orig_file_size, file_chunk_hashes) =
            process_file(file_path, input_dir, &mut chunk_store)?;

        total_orig_size += orig_file_size;
        for chunk_hash in &file_chunk_hashes {
            if let Some((compressed_data, _)) = chunk_store.primary_store.get(chunk_hash) {
                total_compressed_size += compressed_data.len() as u64;
            }
        }

        files_metadata.push((rel_path_str, orig_file_size, file_chunk_hashes));
        pb.inc(1) // for progress bar
    }

    write_chunks(&mut writer, &chunk_store)?;
    write_files_metadata(&mut writer, &files_metadata)?;

    // Calculate reduction percentage
    let reduction_percentage = if total_orig_size > 0 {
        (1.0 - (total_compressed_size as f64 / total_orig_size as f64)) * 100.0
    } else {
        0.0
    };

    Ok(reduction_percentage)
}
