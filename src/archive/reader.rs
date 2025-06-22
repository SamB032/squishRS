use std::fs::{self, File};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::util::header::{convert_timestamp_to_date, verify_header};

pub struct ArchiveReader {
    reader: BufReader<File>,
    archive_size: u64,
    squish_creation_time: String,
    number_of_chunks: u64,
    chunk_table_offset: u64,
    file_table_offset: u64,
}

pub struct ArchiveSummary {
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

impl ArchiveReader {
    pub fn new(archive_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(archive_path)?;
        let mut reader = BufReader::new(file);

        // Get size of archive
        let metadata = fs::metadata(archive_path)?;
        let archive_size = metadata.len();

        // Check magic header
        verify_header(&mut reader)?;

        // Setup buffers for reading
        let mut buf8 = [0u8; 8];
        let mut buf32 = [0u8; 32];

        // Get creation time
        reader.read_exact(&mut buf8)?;
        let squish_creation_time = convert_timestamp_to_date(u64::from_le_bytes(buf8));

        // Read the number of chunks
        reader.read_exact(&mut buf8)?;
        let unique_chunk_count = u64::from_le_bytes(buf8);

        // Skip all chunks
        for _ in 0..unique_chunk_count {
            reader.read_exact(&mut buf32)?;

            reader.read_exact(&mut buf8)?; // original size

            reader.read_exact(&mut buf8)?; // compressed size
            let compressed_size = u64::from_le_bytes(buf8);

            // Skip over compressed data
            reader.seek(SeekFrom::Current(compressed_size as i64))?;
        }

        // Get file table offset
        let file_table_offset = reader.seek(SeekFrom::Current(0))?;

        Ok(Self {
            reader,
            archive_size,
            squish_creation_time,
            chunk_table_offset: unique_chunk_count,
            file_table_offset,
            number_of_chunks: unique_chunk_count,
        })
    }

    /// Returns a summary of the archive's contents, including total size, compression ratio,
    /// number of files, and file metadata.
    ///
    /// This method seeks to the file table offset within the archive and reads metadata
    /// for all stored files. It also calculates statistics such as the total uncompressed
    /// size, compression reduction percentage, and includes general archive information
    /// like the number of unique chunks and creation timestamp.
    ///
    /// # Returns
    ///
    /// * `Ok(ArchiveSummary)` — Contains a high-level overview of the archive's contents,
    ///   including all file paths, their original sizes, and archive statistics.
    /// * `Err(Box<dyn std::error::Error>)` — Returned if the archive is malformed or an I/O
    ///   operation fails (e.g., seeking or reading from the file).
    ///
    /// # Errors
    ///
    /// This function may fail if:
    /// - The file table offset is invalid or corrupted.
    /// - File metadata entries are incomplete or malformed.
    /// - Any I/O operation (e.g., `read_exact`, `seek`) fails.
    /// - File paths cannot be parsed as UTF-8 strings.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut reader = ArchiveReader::open("backup.squish")?;
    /// let summary = reader.get_summary()?;
    /// println!("Files: {}", summary.files.len());
    /// println!("Reduction: {:.2}%", summary.reduction_percentage);
    /// ```
    pub fn get_summary(&mut self) -> Result<ArchiveSummary, Box<dyn std::error::Error>> {
        self.reader.seek(SeekFrom::Start(self.file_table_offset))?;

        let mut buf4 = [0u8; 4];
        let mut buf8 = [0u8; 8];

        // Fails to fill buffer here
        self.reader.read_exact(&mut buf4)?;
        let file_count = u32::from_le_bytes(buf4);

        let mut files = Vec::with_capacity(file_count as usize);
        let mut total_orig_size = 0;

        for _ in 0..file_count {
            // Read Path length
            self.reader.read_exact(&mut buf4)?;
            let path_length = u32::from_le_bytes(buf4) as usize;

            // Read Path
            let mut path_bytes = vec![0u8; path_length];
            self.reader.read_exact(&mut path_bytes)?;
            let path = String::from_utf8(path_bytes)?;

            // Read original size
            self.reader.read_exact(&mut buf8)?;
            let orig_size = u64::from_le_bytes(buf8);
            total_orig_size += orig_size;

            // Read number of chunks belonging to file
            self.reader.read_exact(&mut buf4)?;
            let chunk_count = u32::from_le_bytes(buf4);

            self.reader
                .seek(SeekFrom::Current(chunk_count as i64 * 32))?;

            files.push(FileEntry {
                path,
                original_size: orig_size,
            });
        }

        // Calculate reduction percentage
        let reduction_percentage = if total_orig_size > 0 {
            (1.0 - (self.archive_size as f64 / total_orig_size as f64)) * 100.0
        } else {
            0.0
        };

        Ok(ArchiveSummary {
            unique_chunks: self.number_of_chunks,
            total_original_size: total_orig_size,
            archive_size: self.archive_size,
            reduction_percentage: reduction_percentage,
            squish_creation_date: self.squish_creation_time.clone(),
            files,
        })
    }
}
