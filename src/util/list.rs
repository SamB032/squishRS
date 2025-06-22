use super::header::{convert_timestamp_to_date, verify_header};
use std::fs::{self, File};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

pub struct ListSummary {
    pub unique_chunks: u64,
    pub total_original_size: u64,
    pub archive_size: u64,
    pub reduction_percentage: f64,
    pub squish_creation_date: String,
    pub files: Vec<FileEntry>,
}

pub struct FileEntry {
    pub path: String,
    pub original_size: u64,
}

/// Lists the files contained in a squishRS archive.
///
/// # Arguments
///
/// * `archive_path` - A reference to the path of the archive file to read.
///
/// # Returns
///
/// * `Result<ListSummary, Box<dyn std::error::Error>>` - On success, returns a struct of
///   `ListSummary` which represents the summary of the archive and its files. On failure, returns an error.
///
/// # Errors
///
/// This function will return an error if:
/// - The archive file cannot be opened or read.
/// - The archive header is invalid or corrupted.
/// - Any IO operation fails during reading.
/// - UTF-8 decoding of file paths fails.
///
/// # Example
///
/// ```no_run
/// let files = list_archive(Path::new("target.squish"))?;
/// for file in files {
///     println!("File: {}, size: {}", file.path, file.original_size);
/// }
/// ```
pub fn list_squish(archive_path: &Path) -> Result<ListSummary, Box<dyn std::error::Error>> {
    let file = File::open(archive_path)?;

    // Get size of archive
    let metadata = fs::metadata(archive_path)?;
    let archive_size = metadata.len();

    let mut reader = BufReader::new(file);

    // Check magic header
    verify_header(&mut reader)?;

    // Read the ISO EPOCH
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    let squish_creation_date = convert_timestamp_to_date(u64::from_le_bytes(buf));

    // Read the number of chunks
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    let unique_chunk_count = u64::from_le_bytes(buf);

    // Skip all chunks
    for _ in 0..unique_chunk_count {
        let mut hash = [0u8; 32];
        reader.read_exact(&mut hash)?;

        reader.read_exact(&mut buf)?; // original size
        let _orig_size = u64::from_le_bytes(buf);

        reader.read_exact(&mut buf)?; // compressed size
        let compressed_size = u64::from_le_bytes(buf);

        // Skip over compressed data
        reader.seek(SeekFrom::Current(compressed_size as i64))?;
    }

    // Read file count
    let mut buf4 = [0u8; 4];
    reader.read_exact(&mut buf4)?;
    let file_count = u32::from_le_bytes(buf4);

    let mut files = Vec::with_capacity(file_count as usize);
    let mut total_orig_size: u64 = 0;

    for _ in 0..file_count {
        // Read Path length
        reader.read_exact(&mut buf4)?;
        let path_length = u32::from_le_bytes(buf4) as usize;

        let mut path_bytes = vec![0u8; path_length];
        reader.read_exact(&mut path_bytes)?;
        let path = String::from_utf8(path_bytes)?;

        reader.read_exact(&mut buf)?;
        let orig_size = u64::from_le_bytes(buf);
        total_orig_size += orig_size;

        reader.read_exact(&mut buf4)?;
        let chunk_count = u32::from_le_bytes(buf4);

        reader.seek(SeekFrom::Current(chunk_count as i64 * 32))?;

        files.push(FileEntry {
            path,
            original_size: orig_size,
        });
    }

    // Calculate reduction percentage
    let reduction_percentage = if total_orig_size > 0 {
        (1.0 - (archive_size as f64 / total_orig_size as f64)) * 100.0
    } else {
        0.0
    };

    let summary = ListSummary {
        unique_chunks: unique_chunk_count,
        total_original_size: total_orig_size,
        archive_size,
        reduction_percentage,
        squish_creation_date,
        files,
    };

    Ok(summary)
}
