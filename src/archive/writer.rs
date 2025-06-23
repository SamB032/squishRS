use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crossbeam::channel::{unbounded, Sender};
use indicatif::ProgressBar;
use rayon::prelude::*;

use crate::fsutil::writer::{writer_thread, ChunkMessage, ThreadSafeWriter};
use crate::util::chunk::{ChunkHash, ChunkStore, CHUNK_SIZE};
use crate::util::header::{patch_u64, write_header, write_placeholder_u64, write_timestamp};

type PackedResult = Result<(String, u64, Vec<ChunkHash>), Box<dyn std::error::Error + Send + Sync>>;

pub struct ArchiveWriter {
    writer: Arc<Mutex<BufWriter<File>>>,
    chunk_store: ChunkStore,
    sender: Sender<ChunkMessage>,
    progress_bar: Option<ProgressBar>,
    input_path: PathBuf,
    placeholder_number_of_chunks_pos: u64,
}

impl ArchiveWriter {
    pub fn new(
        input_dir: &Path,
        output_path: &Path,
        progress_bar: Option<&mut ProgressBar>,
    ) -> std::io::Result<Self> {
        // Open output writer
        let output = File::create(output_path)?;
        let writer = Arc::new(Mutex::new(BufWriter::new(output)));

        // Write header and timestamp
        let placeholder_number_of_chunks_pos;
        {
            let mut guard = writer.lock().unwrap();
            write_header(&mut *guard)?;
            write_timestamp(&mut *guard)?;
            placeholder_number_of_chunks_pos = write_placeholder_u64(&mut *guard)?;
            guard.flush()?;
        }

        let chunk_store = ChunkStore::new();
        let (sender, receiver) = unbounded::<ChunkMessage>();

        // Spawn writer thread
        let thread_safe_writer = ThreadSafeWriter::new(Arc::clone(&writer));
        std::thread::spawn(move || -> std::io::Result<()> {
            writer_thread(thread_safe_writer, receiver)
        });

        Ok(Self {
            writer,
            chunk_store,
            sender,
            progress_bar: progress_bar.cloned(),
            input_path: input_dir.to_path_buf(),
            placeholder_number_of_chunks_pos,
        })
    }

    pub fn pack(&self, files: &[PathBuf]) -> Result<u64, Box<dyn std::error::Error>> {
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
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;

        // Close sender so writer thread can finish
        drop(self.sender.clone());

        // Write number of chunks in the placeholder
        {
            let mut guard = self.writer.lock().unwrap();
            patch_u64(
                &mut *guard,
                self.placeholder_number_of_chunks_pos,
                self.chunk_store.len(),
            )?;
            guard.flush()?;
        }

        // Write metadata at the end
        self.write_files_metadata(&files_metadata)?;

        // Return archive size
        let binding = self.writer.lock().unwrap();
        let file = binding.get_ref();
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
    /// - A `Vec` of 32-byte chunk hashes (`[u8; 32]`) representing the chunks of the file.
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
            let bytes_read = reader.read(&mut chunk_buf)?;
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
                self.sender.send(msg)?;
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
    ///    - Each 32-byte chunk hash
    ///
    /// # Arguments
    /// * `files_metadata` – Slice of `(String, u64, Vec<[u8; 32]>)` tuples containing:
    ///     1. File’s relative path
    ///     2. Original file size
    ///     3. Vector of chunk hashes
    ///
    /// # Errors
    /// Returns an error if any I/O write operation fails.
    fn write_files_metadata(
        &self,
        files_metadata: &[(String, u64, Vec<ChunkHash>)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Lock the shared writer once
        let mut guard = self.writer.lock().unwrap();

        // Number of files
        let file_count = files_metadata.len() as u32;
        guard.write_all(&file_count.to_le_bytes())?;

        // For each file: path length, path, original size, chunk count, chunk hashes
        for (path, orig_size, chunk_hashes) in files_metadata {
            let path_bytes = path.as_bytes();
            let path_len = path_bytes.len() as u32;

            guard.write_all(&path_len.to_le_bytes())?;
            guard.write_all(path_bytes)?;
            guard.write_all(&orig_size.to_le_bytes())?;

            let chunk_count = chunk_hashes.len() as u32;
            guard.write_all(&chunk_count.to_le_bytes())?;

            for hash in chunk_hashes {
                guard.write_all(hash)?;
            }
        }
        guard.flush()?;
        Ok(())
    }
}
