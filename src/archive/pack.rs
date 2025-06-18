use super::chunk::{hash_chunk, ChunkStore, CHUNK_SIZE};
use super::header::{write_header, write_timestamp};
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

type PackedResult = Result<(String, u64, Vec<[u8; 32]>), Box<dyn std::error::Error + Send + Sync>>;

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

    for entry in chunk_store.primary_store.iter() {
        let chunk_hash = entry.key();
        let (compressed_data, orig_size) = entry.value();

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
fn process_file(file_path: &Path, input_dir: &Path, chunk_store: &ChunkStore) -> PackedResult {
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
        let _ = chunk_store.insert(&chunk_buf);

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
    pb: Option<&mut ProgressBar>,
) -> Result<u64, Box<dyn std::error::Error>> {
    // Open output writer
    let output = fs::File::create(output_file)?;
    let mut writer = BufWriter::new(output);

    write_header(&mut writer)?;
    write_timestamp(&mut writer)?;

    let chunk_store = ChunkStore::new();

    // Run process_file function concurrently
    let files_metadata: Vec<_> = files
        .par_iter()
        .map(|file_path| -> PackedResult {
            let result = process_file(file_path, input_dir, &chunk_store)?;

            // Increment progres bar if present
            if let Some(pb) = pb.as_ref() {
                pb.inc(1);
            }

            Ok(result)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;

    write_chunks(&mut writer, &chunk_store)?;
    write_files_metadata(&mut writer, &files_metadata)?;

    // Track archive size
    let archive_metadata = fs::metadata(output_file)?;
    let archive_size = archive_metadata.len();

    Ok(archive_size)
}
