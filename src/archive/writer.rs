use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crossbeam::channel::{unbounded, Sender};
use indicatif::ProgressBar;
use rayon::prelude::*;

use crate::fsutil::writer::{writer_thread, ChunkMessage, ThreadSafeWriter};
use crate::util::chunk::{ChunkHash, ChunkStore, CHUNK_SIZE};
use crate::util::errors::AppError;
use crate::util::header::{patch_u64, write_header, write_placeholder_u64, write_timestamp};

type PackedResult = Result<(String, u64, Vec<ChunkHash>), Box<dyn std::error::Error + Send + Sync>>;

pub struct ArchiveWriter {
    writer: Arc<Mutex<BufWriter<File>>>,
    chunk_store: ChunkStore,
    sender: Option<Sender<ChunkMessage>>,
    progress_bar: Option<ProgressBar>,
    input_path: PathBuf,
    chunks_count_position: u64,
    writer_handle: Option<std::thread::JoinHandle<std::io::Result<()>>>,
}

impl ArchiveWriter {
    /// Creates a new `ArchiveWriter` for packing files into an archive.
    ///
    /// This function initializes the archive by:
    /// - Creating and buffering the output file,
    /// - Writing the archive header and a timestamp,
    /// - Reserving space for the number of chunks (to be patched later),
    /// - Setting up a `ChunkStore` for deduplication,
    /// - Spawning a background writer thread to handle chunk writing,
    /// - Optionally associating a progress bar for visual feedback.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - A reference to the input directory from which files will be collected.
    /// * `output_path` - The path where the archive file will be created.
    /// * `progress_bar` - An optional mutable reference to a `ProgressBar` (from `indicatif`) for tracking progress.
    ///
    /// # Returns
    ///
    /// * `Ok(ArchiveWriter)` - On successful initialization of the archive writer.
    /// * `Err(AppError)` - If any I/O error occurs while creating the output file or writing initial metadata.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - The output file cannot be created or written to,
    /// - The header, timestamp, or placeholder values cannot be written or flushed,
    /// - The writer thread cannot be started (though this is rare).
    ///
    /// # Example
    ///
    /// ```rust
    /// use squishrs::archive::ArchiveWriter;
    /// use std::path::Path;
    ///
    /// let output = Path::new("output.squish");
    /// let input = Path::new("./files");
    /// let writer = ArchiveWriter::new(input, output, None)?;
    /// ```;
    pub fn new(
        input_dir: &Path,
        output_path: &Path,
        progress_bar: Option<&mut ProgressBar>,
    ) -> Result<Self, AppError> {
        // Open output writer
        let output = File::create(output_path)?;
        let writer = Arc::new(Mutex::new(BufWriter::new(output)));

        // Write header and timestamp
        let chunks_count_position;
        {
            let mut guard = writer.lock().map_err(|_| AppError::LockPoisoned)?;
            write_header(&mut *guard).map_err(AppError::WriterError)?;
            write_timestamp(&mut *guard).map_err(AppError::WriterError)?;

            // Write placeholder for chunk count
            chunks_count_position =
                write_placeholder_u64(&mut *guard).map_err(AppError::WriterError)?;
            guard.flush()?;
        }

        let chunk_store = ChunkStore::new();
        let (sender, receiver) = unbounded::<ChunkMessage>();

        // Spawn writer thread
        let thread_safe_writer = ThreadSafeWriter::new(Arc::clone(&writer));
        let handle = std::thread::spawn(move || -> std::io::Result<()> {
            writer_thread(thread_safe_writer, receiver)
                .map_err(|_e| std::io::Error::other("Writer Thread Failed"))
        });

        Ok(Self {
            writer,
            chunk_store,
            sender: Some(sender),
            progress_bar: progress_bar.cloned(),
            input_path: input_dir.to_path_buf(),
            chunks_count_position,
            writer_handle: Some(handle),
        })
    }

    /// Packs a list of files into the archive.
    ///
    /// This method takes a slice of file paths and processes each file concurrently using Rayon.
    /// For each file, it reads and compresses its contents, sends the resulting chunks to a background writer thread,
    /// and optionally updates a progress bar if one is enabled.
    ///
    /// After all files are processed, the function:
    /// - Waits for the writer thread to finish,
    /// - Patches the placeholder value for the total number of chunks written,
    /// - Appends metadata for all files at the end of the archive,
    /// - Returns the final size of the archive in bytes.
    ///
    /// # Arguments
    ///
    /// * `files` - A slice of `PathBuf` objects representing the files to be packed into the archive.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The total size of the resulting archive in bytes, if the operation is successful.
    /// * `Err(Box<dyn std::error::Error>)` - If any I/O, thread join, or metadata-related error occurs.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any file fails to be read or processed,
    /// - The writer thread fails or panics,
    /// - File metadata cannot be written or retrieved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use squishrs::archive::ArchiveWriter;
    /// use std::path::PathBuf;
    ///
    /// let mut writer = ArchiveWriter::new("output.squish")?;
    /// let files = vec![PathBuf::from("file1.txt"), PathBuf::from("file2.txt")];
    /// let archive_size = writer.pack(&files)?;
    /// println!("Archive written ({} bytes)", archive_size);
    /// ```
    pub fn pack(&mut self, files: &[PathBuf]) -> Result<u64, AppError> {
        // Run process_file function concurrently
        let files_metadata: Vec<_> = files
            .par_iter()
            .map(|file_path| -> PackedResult {
                let result = self.process_file(file_path)?;

                // Increment progres bar if present
                if let Some(pb) = self.progress_bar.as_ref() {
                    pb.inc(1);
                }

                Ok(result)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Close sender so writer thread can finish
        if let Some(sender) = self.sender.take() {
            drop(sender);
        }

        if let Some(handle) = self.writer_handle.take() {
            handle.join().expect("Writer thread panicked")?;
        }

        // Write number of chunks in the placeholder
        {
            let mut guard = self.writer.lock().map_err(|_| AppError::LockPoisoned)?;
            patch_u64(
                &mut *guard,
                self.chunks_count_position,
                self.chunk_store.len(),
            )?;
        }

        // Write metadata at the end
        self.write_files_metadata(&files_metadata)?;

        // Return archive size
        let guard = self.writer.lock().map_err(|_| AppError::LockPoisoned)?;
        let file = guard.get_ref();
        let size = file.metadata()?.len();

        Ok(size)
    }

    /// Processes a single file by reading it in fixed-size chunks, inserting those chunks into
    /// a chunk store, and optionally sending compressed chunk data through a channel.
    ///
    /// # Arguments
    ///
    /// * `file_path` - A reference to the path of the file to process.
    ///
    /// # Returns
    ///
    /// On success, returns a tuple containing:
    /// - The file path relative to the configured input directory as a `String`.
    /// - The original uncompressed size of the file as a `u64`.
    /// - A `Vec` of 16-byte chunk hashes (`[u8; 16]`) representing the chunks of the file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The relative path cannot be derived from the input path.
    /// - The file cannot be opened or read.
    /// - Metadata cannot be accessed.
    /// - Chunk insertion into the chunk store fails.
    /// - Sending compressed chunk data through the channel fails.
    ///
    /// # Behavior
    ///
    /// The method:
    /// - Opens the file and obtains its size.
    /// - Reads the file in chunks of size `CHUNK_SIZE`.
    /// - Inserts each chunk into the chunk store, which may return compressed data.
    /// - If compressed data is returned, it sends a `ChunkMessage` containing the chunk hash,
    ///   compressed data, and original chunk size through a channel.
    /// - Collects all chunk hashes to associate with the processed file.
    fn process_file(&self, file_path: &Path) -> PackedResult {
        let rel_path = file_path.strip_prefix(&self.input_path)?;
        let rel_path_str = rel_path.to_string_lossy();

        let file = File::open(file_path)?;
        let metadata = file.metadata()?;
        let orig_file_size = metadata.len();

        let mut reader = BufReader::new(file);
        let mut file_chunk_hashes = Vec::new();

        let mut chunk_buf = vec![0u8; CHUNK_SIZE];
        loop {
            let bytes_read = reader.read(&mut chunk_buf).map_err(AppError::ReaderError)?;
            if bytes_read == 0 {
                break;
            }
            let slice = &chunk_buf[..bytes_read];

            // Insert chunk via ChunkStore
            let result = self.chunk_store.insert(slice)?;

            if let Some(compressed) = result.compressed_data {
                let msg = ChunkMessage {
                    hash: result.hash,
                    compressed_data: compressed,
                    original_size: chunk_buf.len() as u64,
                };
                if let Some(sender) = &self.sender {
                    sender
                        .send(msg)
                        .map_err(|e| AppError::SenderError(Box::new(e)))?;
                } else {
                    // sender is None, maybe return an error or handle accordingly
                    return Err("Sender channel is closed".into());
                }
            }
            // Calculate chunk hash and store it for the file metadata
            file_chunk_hashes.push(result.hash);
        }

        Ok((rel_path_str.to_string(), orig_file_size, file_chunk_hashes))
    }

    /// Writes file metadata at the end of the archive using the shared writer.
    ///
    /// This method locks the internal writer once and then writes:
    /// 1. Number of files in the archive (`u32`, little-endian)
    /// 2. For each file:
    ///    - Path length (`u32`, little-endian)
    ///    - Path bytes (UTF-8)
    ///    - Original file size (`u64`, little-endian)
    ///    - Number of chunks for this file (`u32`, little-endian)
    ///    - Each 16-byte chunk hash
    ///
    /// # Arguments
    /// * `files_metadata` – Slice of `(String, u64, Vec<[u8; 16]>)` tuples containing:
    ///     1. File’s relative path
    ///     2. Original file size
    ///     3. Vector of chunk hashes
    ///
    /// # Errors
    /// Returns an error if any I/O write operation fails.
    fn write_files_metadata(
        &self,
        files_metadata: &[(String, u64, Vec<ChunkHash>)],
    ) -> Result<(), AppError> {
        // Lock the shared writer once
        let mut guard = self.writer.lock().unwrap();

        // Number of files
        let file_count = files_metadata.len() as u32;
        guard
            .write_all(&file_count.to_le_bytes())
            .map_err(AppError::WriterError)?;

        // For each file: path length, path, original size, chunk count, chunk hashes
        for (path, orig_size, chunk_hashes) in files_metadata {
            let path_bytes = path.as_bytes();
            let path_len = path_bytes.len() as u32;

            guard
                .write_all(&path_len.to_le_bytes())
                .map_err(AppError::WriterError)?;
            guard.write_all(path_bytes).map_err(AppError::WriterError)?;
            guard
                .write_all(&orig_size.to_le_bytes())
                .map_err(AppError::WriterError)?;

            let chunk_count = chunk_hashes.len() as u32;
            guard
                .write_all(&chunk_count.to_le_bytes())
                .map_err(AppError::WriterError)?;

            for hash in chunk_hashes {
                guard.write_all(hash).map_err(AppError::WriterError)?;
            }
        }
        guard.flush().map_err(AppError::WriterError)?;
        Ok(())
    }
}
