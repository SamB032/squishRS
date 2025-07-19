use std::fs;
use std::io::{BufWriter, Write};
use std::sync::Arc;
use std::sync::Mutex;

use crate::util::chunk::ChunkHash;
use crate::util::errors::AppError;

use crossbeam::channel::Receiver;

pub struct ChunkMessage {
    pub hash: ChunkHash,
    pub compressed_data: Arc<Vec<u8>>,
    pub original_size: u64,
}

pub fn writer_thread<W: Write + Send + 'static>(
    mut writer: W,
    rx: Receiver<ChunkMessage>,
) -> Result<(), AppError> {
    for chunk_msg in rx.iter() {
        let compressed_size = chunk_msg.compressed_data.len() as u64;

        writer
            .write_all(&chunk_msg.hash)
            .map_err(AppError::WriterError)?;
        writer
            .write_all(&chunk_msg.original_size.to_le_bytes())
            .map_err(AppError::WriterError)?;
        writer
            .write_all(&compressed_size.to_le_bytes())
            .map_err(AppError::WriterError)?;
        writer
            .write_all(&chunk_msg.compressed_data)
            .map_err(AppError::WriterError)?;
    }
    writer.flush().map_err(AppError::FlushError)?;
    Ok(())
}

// Wrapper that implements Write for Arc<Mutex<BufWriter<fs::File>>>
pub struct ThreadSafeWriter {
    pub writer: Arc<Mutex<BufWriter<fs::File>>>,
}

impl ThreadSafeWriter {
    pub fn new(writer: Arc<Mutex<BufWriter<fs::File>>>) -> Self {
        Self { writer }
    }
}

impl Write for ThreadSafeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.writer.lock().unwrap();
        guard.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut guard = self.writer.lock().unwrap();
        guard.flush()
    }
}
