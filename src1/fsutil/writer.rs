use crossbeam::channel::Receiver;
use std::fs;
use std::io::{BufWriter, Write};
use std::sync::Arc;
use std::sync::Mutex;

pub struct ChunkMessage {
    pub hash: [u8; 32],
    pub compressed_data: Arc<Vec<u8>>,
    pub original_size: u64,
}

pub fn writer_thread<W: Write + Send + 'static>(
    mut writer: W,
    rx: Receiver<ChunkMessage>,
) -> std::io::Result<()> {
    while let Ok(chunk_msg) = rx.recv() {
        let compressed_size = chunk_msg.compressed_data.len() as u64;

        writer.write_all(&chunk_msg.hash)?;
        writer.write_all(&chunk_msg.original_size.to_le_bytes())?;
        writer.write_all(&compressed_size.to_le_bytes())?;
        writer.write_all(&chunk_msg.compressed_data)?;
    }
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
