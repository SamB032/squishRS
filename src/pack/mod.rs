use crate::chunk;
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

const CHUNK_SIZE: usize = 1024 * 1024; // 1MB

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

    chunk::write_header(&mut writer)?;

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
            let hash_arr = chunk::hash_chunk(&chunk_buf);

            if !chunk_store.contains_key(&hash_arr) {
                // Compress chunk
                let compressed_chunk = chunk::compress_chunk(&chunk_buf)?;
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
