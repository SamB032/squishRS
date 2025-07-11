use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use indicatif::ProgressBar;
use rayon::prelude::*;
use zstd::decode_all;

use crate::util::chunk::ChunkHash;
use crate::util::errors::{AppError, CustomErr};
use crate::util::header::{convert_timestamp_to_date, verify_header};

pub struct ArchiveReader {
    reader: BufReader<File>,
    archive_size: u64,
    squish_creation_time: String,
    number_of_chunks: u64,
    squish_version: String,
    file_count: u32,
    chunk_table_offset: u64,
    file_table_offset: u64,
}

pub struct ArchiveSummary {
    pub unique_chunks: u64,
    pub total_original_size: u64,
    pub archive_size: u64,
    pub reduction_percentage: f64,
    pub squish_creation_date: String,
    pub squish_version: String,
    pub files: Vec<FileEntry>,
}

pub struct FileEntry {
    pub path: String,
    pub original_size: u64,
}

struct FileRebuildEntry {
    relative_path: String,
    chunk_hashes: Vec<ChunkHash>,
}

impl ArchiveReader {
    pub fn new(archive_path: &Path) -> Result<Self, AppError> {
        let file = File::open(archive_path).map_err(CustomErr::FileNotExist)?;
        let mut reader = BufReader::new(file);

        // Get size of archive
        let metadata = fs::metadata(archive_path)?;
        let archive_size = metadata.len();

        // Check magic header
        let squish_version = verify_header(&mut reader)?;

        // Setup buffers for reading
        let mut buf8 = [0u8; 8];
        let mut buf16 = [0u8; 16];

        // Get creation time
        reader.read_exact(&mut buf8)?;
        let squish_creation_time = convert_timestamp_to_date(u64::from_le_bytes(buf8));

        // Read the number of chunks
        reader
            .read_exact(&mut buf8)
            .map_err(CustomErr::ReaderError)?;
        let unique_chunk_count = u64::from_le_bytes(buf8);

        let chunk_table_offset = reader.stream_position().map_err(CustomErr::ReaderError)?;

        // Skip all chunks
        for _ in 0..unique_chunk_count {
            // Read chunk hash
            reader
                .read_exact(&mut buf16)
                .map_err(CustomErr::ReaderError)?;

            // original size
            reader
                .read_exact(&mut buf8)
                .map_err(CustomErr::ReaderError)?;

            // compressed size
            reader
                .read_exact(&mut buf8)
                .map_err(CustomErr::ReaderError)?;
            let compressed_size = u64::from_le_bytes(buf8);

            // Skip over compressed data
            reader
                .seek(SeekFrom::Current(compressed_size as i64))
                .map_err(CustomErr::ReaderError)?;
        }

        // Read number of files (u32)
        let mut buf4 = [0u8; 4];
        reader
            .read_exact(&mut buf4)
            .map_err(CustomErr::ReaderError)?;
        let file_count = u32::from_le_bytes(buf4);

        // Get file table offset
        let file_table_offset = reader.stream_position().map_err(CustomErr::ReaderError)?;

        Ok(Self {
            reader,
            archive_size,
            squish_creation_time,
            number_of_chunks: unique_chunk_count,
            file_count,
            chunk_table_offset,
            file_table_offset,
            squish_version,
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
    pub fn get_summary(&mut self) -> Result<ArchiveSummary, AppError> {
        self.reader
            .seek(SeekFrom::Start(self.file_table_offset))
            .map_err(CustomErr::ReaderError)?;

        let mut buf4 = [0u8; 4];
        let mut buf8 = [0u8; 8];

        let mut files = Vec::with_capacity(self.file_count as usize);
        let mut total_orig_size = 0;

        for _ in 0..self.file_count {
            // Read Path length
            self.reader
                .read_exact(&mut buf4)
                .map_err(CustomErr::ReaderError)?;
            let path_length = u32::from_le_bytes(buf4) as usize;

            // Read Path
            let mut path_bytes = vec![0u8; path_length];
            self.reader
                .read_exact(&mut path_bytes)
                .map_err(CustomErr::ReaderError)?;
            let path = String::from_utf8(path_bytes)?;

            // Read original size
            self.reader
                .read_exact(&mut buf8)
                .map_err(CustomErr::ReaderError)?;
            let orig_size = u64::from_le_bytes(buf8);
            total_orig_size += orig_size;

            // Read number of chunks belonging to file
            self.reader
                .read_exact(&mut buf4)
                .map_err(CustomErr::ReaderError)?;
            let chunk_count = u32::from_le_bytes(buf4);

            self.reader
                .seek(SeekFrom::Current(chunk_count as i64 * 16))
                .map_err(CustomErr::ReaderError)?;

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
            reduction_percentage,
            squish_creation_date: self.squish_creation_time.clone(),
            squish_version: self.squish_version.clone(),
            files,
        })
    }

    /// Unpacks the archive contents into the specified output directory.
    ///
    /// Reads all chunks, decompresses them, and reconstructs all files,
    /// writing them into `output_dir`.
    ///
    /// # Arguments
    /// * `output_dir` - Directory path where files should be restored.
    /// * `progress_bar` - Optional progress bar for progress reporting.
    ///
    /// # Errors
    /// Returns an error if reading, decompression, or writing fails.
    pub fn unpack(
        &mut self,
        output_dir: &Path,
        progress_bar: Option<&mut ProgressBar>,
    ) -> Result<(), AppError> {
        // Read chunks here
        let chunk_map = self.read_chunks(progress_bar.as_deref())?;

        // Rebuild files from chunk_map
        self.rebuild_files(&chunk_map, output_dir, progress_bar.as_deref())?;

        Ok(())
    }

    /// Reads and decompresses all chunks from the archive's chunk table into memory.
    ///
    /// Seeks to the chunk table offset stored in the archive, then reads and decompresses
    /// each chunk. Decompressed chunks are stored in a HashMap keyed by their 16-byte hash.
    ///
    /// # Arguments
    /// * `pb` - Optional progress bar for tracking chunk reading progress.
    ///
    /// # Returns
    /// A `HashMap` where keys are chunk hashes (`[u8; 16]`) and values are decompressed chunk data (`Vec<u8>`).
    ///
    /// # Errors
    /// Returns an error if any IO operation or decompression fails.
    fn read_chunks(
        &mut self,
        progress_bar: Option<&ProgressBar>,
    ) -> Result<HashMap<ChunkHash, Vec<u8>>, Box<dyn std::error::Error>> {
        // Seek to chunk table offset
        self.reader
            .seek(std::io::SeekFrom::Start(self.chunk_table_offset))?;

        let mut buf8 = [0u8; 8];
        let mut chunk_map: HashMap<ChunkHash, Vec<u8>> = HashMap::new();

        // Setup progress bar if one is given
        if let Some(progress_bar) = progress_bar {
            progress_bar.set_length(self.number_of_chunks);
        }

        // For each chunk, decompress and insert it corresponding hash into the hashmap
        for _ in 0..self.number_of_chunks {
            let mut hash = [0u8; 16];
            self.reader
                .read_exact(&mut hash)
                .map_err(CustomErr::ReaderError)?;

            // original size
            self.reader
                .read_exact(&mut buf8)
                .map_err(CustomErr::ReaderError)?;
            let _orig_size = u64::from_le_bytes(buf8);

            // compressed size
            self.reader
                .read_exact(&mut buf8)
                .map_err(CustomErr::ReaderError)?;
            let compressed_size = u64::from_le_bytes(buf8);

            let mut compressed_data = vec![0u8; compressed_size as usize];
            self.reader
                .read_exact(&mut compressed_data)
                .map_err(CustomErr::ReaderError)?;

            let decompressed = decode_all(&compressed_data[..]).map_err(CustomErr::ReaderError)?;
            chunk_map.insert(hash, decompressed);

            // Increment progress bar if it exists
            if let Some(progress_bar) = progress_bar {
                progress_bar.inc(1);
            }
        }

        Ok(chunk_map)
    }

    fn rebuild_files(
        &mut self,
        chunk_map: &HashMap<ChunkHash, Vec<u8>>,
        output_dir: &Path,
        progress_bar: Option<&ProgressBar>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Move to the file table
        self.reader
            .seek(SeekFrom::Start(self.file_table_offset))
            .map_err(CustomErr::ReaderError)?;

        let mut buf4 = [0u8; 4];
        let mut buf8 = [0u8; 8];
        let mut entries = Vec::with_capacity(self.file_count as usize);

        // Setup progress bar if one is given
        if let Some(progress_bar) = progress_bar {
            progress_bar.set_length(self.file_count as u64);
            progress_bar.set_message("Rebuilding files");
            progress_bar.set_position(0);
        }

        for _ in 0..self.file_count {
            // Read Path Length
            self.reader
                .read_exact(&mut buf4)
                .map_err(CustomErr::ReaderError)?;
            let path_length = u32::from_le_bytes(buf4) as usize;

            // Get Full Path of File
            let mut path_bytes = vec![0u8; path_length];
            self.reader
                .read_exact(&mut path_bytes)
                .map_err(CustomErr::ReaderError)?;
            let relative_path = String::from_utf8(path_bytes)?;

            // Read Original Size and Disgard
            self.reader
                .read_exact(&mut buf8)
                .map_err(CustomErr::ReaderError)?;

            // Read Chunk Count
            self.reader
                .read_exact(&mut buf4)
                .map_err(CustomErr::ReaderError)?;
            let chunk_count = u32::from_le_bytes(buf4);

            // Read chunk hashes
            let mut chunks = Vec::with_capacity(chunk_count as usize);
            for _ in 0..chunk_count {
                let mut hash = [0u8; 16];
                self.reader
                    .read_exact(&mut hash)
                    .map_err(CustomErr::ReaderError)?;
                chunks.push(hash);
            }

            entries.push(FileRebuildEntry {
                relative_path,
                chunk_hashes: chunks,
            });
        }

        // Rebuild files in parallel
        entries
            .par_iter()
            .try_for_each(
                |entry| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let full_path = output_dir.join(PathBuf::from(&entry.relative_path));
                    if let Some(parent) = full_path.parent() {
                        fs::create_dir_all(parent).map_err(CustomErr::CreateDirError)?;
                    }

                    let mut writer = BufWriter::new(
                        File::create(&full_path).map_err(CustomErr::CreateFileError)?,
                    );
                    for hash in &entry.chunk_hashes {
                        if let Some(data) = chunk_map.get(hash) {
                            writer.write_all(data).map_err(CustomErr::CreateDirError)?;
                        } else {
                            return Err(
                                format!("Missing chunk for file: {}", entry.relative_path).into()
                            );
                        }
                    }

                    if let Some(pb) = progress_bar {
                        pb.inc(1);
                    }

                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
                },
            )
            .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;

        Ok(())
    }
}
