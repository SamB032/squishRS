use indicatif::ProgressBar;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use zstd::stream::write::Encoder;


const CHUNK_SIZE: usize = 1024 * 1024; // 1MB

/// Packs a collection of files from a directory into a single compressed archive using chunk-based
/// deduplication.
///
/// This function reads each file relative to `input_dir`, splits the file contents into fixed-size
/// chunks, and computes a SHA256 hash for each chunk. Unique chunks are compressed using zstd and
/// stored once in the archive. Files are represented as sequences of chunk hashes to enable
/// deduplication of identical chunks across files.
///
/// The archive format includes:
/// - A magic header and version (`SQUISHR02`)
/// - The number of unique compressed chunks, followed by each chunk's hash, original size,
///   compressed size, and compressed data
/// - The number of files, followed by each file's relative path, original size, chunk count,
///   and ordered list of chunk hashes
///
/// This approach improves compression efficiency by avoiding repeated compression of identical
/// data chunks across multiple files.
///
/// The function updates the given progress bar as it processes each file.
///
/// # Arguments
///
/// * `input_dir` - The root directory against which file paths are made relative.
/// * `output_file` - The path where the packed archive will be written.
/// * `files` - A slice of file paths to pack. Paths should be within `input_dir`.
/// * `pb` - A progress bar to reflect packing progress.
///
/// # Returns
///
/// Returns a `Result` containing the overall reduction percentage (how much size was saved by compression)
/// as a `f64` on success, or an error boxed as `Box<dyn std::error::Error>`.
///
/// # Errors
///
/// Returns an error if any file cannot be read, if writing to the output file fails, or if compression fails.
///
/// # Example
///
/// ```no_run
/// use indicatif::ProgressBar;
/// use std::path::{Path, PathBuf};
///
/// let input_dir = Path::new("my_data");
/// let output = Path::new("archive.squish");
/// let files: Vec<PathBuf> = vec![ /* list of files */ ];
/// let pb = ProgressBar::new(files.len() as u64);
///
/// match pack_directory(input_dir, output, &files, &pb) {
///     Ok(reduction) => println!("Compression reduced size by {:.2}%", reduction),
///     Err(e) => eprintln!("Failed to pack files: {}", e),
/// }
/// pb.finish_with_message("Packing complete");
/// ```
pub fn pack_directory(
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

    writer.write_all(b"SQUISHR02")?; // Magic + Version

    // Map from chunk hash to (compressed data, original size)
    let mut chunk_store: HashMap<[u8; 32], (Vec<u8>, u64)> = HashMap::new();

    // Per-file metadata: Vec of chunk hashes (in order)
    let mut files_metadata = Vec::new();

    // Process all files, splitting into chunks, hashing and compressing unique chunks
    for file_path in files {
        let rel_path = file_path.strip_prefix(input_dir)?;
        let rel_path_str = rel_path.to_string_lossy();

        let file = fs::File::open(file_path)?;
        let metadata = file.metadata()?;
        let orig_file_size = metadata.len();
        total_orig_size += orig_file_size;

        let mut reader = BufReader::new(file);

        let mut file_chunk_hashes = Vec::new();

        loop {
            let mut chunk_buf = vec![0u8; CHUNK_SIZE];
            let bytes_read = reader.read(&mut chunk_buf)?;
            if bytes_read == 0 {
                break;
            }
            chunk_buf.truncate(bytes_read);

            // Compute SHA256 hash of chunk
            let mut hasher = Sha256::new();
            hasher.update(&chunk_buf);
            let chunk_hash = hasher.finalize();

            // Convert to array for HashMap key
            let mut hash_arr = [0u8; 32];
            hash_arr.copy_from_slice(&chunk_hash);

            if !chunk_store.contains_key(&hash_arr) {
                // Compress chunk
                let mut compressed_chunk = Vec::new();
                {
                    let mut encoder = Encoder::new(&mut compressed_chunk, 0)?;
                    encoder.write_all(&chunk_buf)?;
                    encoder.finish()?;
                }
                total_compressed_size += compressed_chunk.len() as u64;
                chunk_store.insert(hash_arr, (compressed_chunk, bytes_read as u64));
            }

            file_chunk_hashes.push(hash_arr);
        }

        files_metadata.push((rel_path_str.into_owned(), orig_file_size, file_chunk_hashes));
        pb.inc(1);
    }

    // Write unique chunks count
    let unique_chunk_count = chunk_store.len() as u64;
    writer.write_all(&unique_chunk_count.to_le_bytes())?;

    // Write each unique chunk: hash (32 bytes), original size, compressed size, compressed data
    for (chunk_hash, (compressed_data, orig_size)) in &chunk_store {
        writer.write_all(chunk_hash)?;
        writer.write_all(&orig_size.to_le_bytes())?;
        let compressed_size = compressed_data.len() as u64;
        writer.write_all(&compressed_size.to_le_bytes())?;
        writer.write_all(compressed_data)?;
    }

    // Write file count
    let file_count = files_metadata.len() as u64;
    writer.write_all(&file_count.to_le_bytes())?;

    // Write file metadata
    for (path, orig_size, chunk_hashes) in &files_metadata {
        let path_bytes = path.as_bytes();
        let path_len = path_bytes.len() as u32;

        writer.write_all(&path_len.to_le_bytes())?;
        writer.write_all(path_bytes)?;
        writer.write_all(&orig_size.to_le_bytes())?;

        let chunk_count = chunk_hashes.len() as u32;
        writer.write_all(&chunk_count.to_le_bytes())?;

        // Write chunk hashes in order (each 32 bytes)
        for hash in chunk_hashes {
            writer.write_all(hash)?;
        }
    }

    let ratio = (total_compressed_size as f64) / (total_orig_size as f64);
    let reduction_percentage = 100.0 * (1.0 - ratio);

    Ok(reduction_percentage)
}
