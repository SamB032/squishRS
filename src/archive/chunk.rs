use libflate::deflate::Encoder;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::{collections::HashMap, usize};

pub const CHUNK_SIZE: usize = 2048 * 1024; // 2MB

pub struct ChunkStore {
    pub primary_store: HashMap<[u8; 32], (Vec<u8>, u64)>,
    pub secondary_store: HashMap<Vec<u8>, Vec<u8>>,
}

/// Calculates the hash of a binary array
///
/// # arguments
///
/// * 'data' - binary array
///
/// # returns
///
/// * '[u8;32]' - 32 bit hash
///
/// # examples
///
/// ```
/// chunk::hash_chunk(&chunk_buf);
/// ```
pub fn hash_chunk(chunk: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(chunk);
    let result = hasher.finalize();
    let mut hash_arr = [0u8; 32];
    hash_arr.copy_from_slice(&result);
    hash_arr
}

impl ChunkStore {
    pub fn new() -> Self {
        ChunkStore {
            primary_store: HashMap::new(),
            secondary_store: HashMap::new(),
        }
    }

    /// Inserts a chunk of data into the `ChunkStore`, performing deduplication and compression.
    ///
    /// This method first checks if the chunk's hash already exists in the primary store:
    /// - If found, it returns the existing compressed data clone (avoiding recompression).
    /// - Otherwise, it compresses the chunk using the configured compression encoder.
    ///
    /// After compression, it performs secondary deduplication by storing the chunk and its compressed
    /// version in a secondary store only if the compression was effective (compressed data is smaller).
    ///
    /// Finally, it inserts the compressed chunk and its original size into the primary store.
    ///
    /// # Arguments
    ///
    /// * `chunk` - A byte slice representing the chunk to insert.
    ///
    /// # Returns
    ///
    /// Returns the compressed data as a `Vec<u8>`, either retrieved from the store or newly compressed.
    ///
    /// # Errors
    ///
    /// Returns an error if compression or writing to the encoder fails.
    pub fn insert(&mut self, chunk: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Primary duplication
        let hash = hash_chunk(chunk);
        if let Some((compressed, _)) = self.primary_store.get(&hash) {
            return Ok(compressed.clone());
        }

        // Compress if HashMap miss
        let mut compressed = Vec::new();
        {
            let mut encoder = Encoder::new(&mut compressed);
            encoder.write_all(chunk)?;
            encoder.flush()?;
        }

        // Secondary deduplication if compression is effective
        if compressed.len() < chunk.len() {
            self.secondary_store
                .insert(chunk.to_vec(), compressed.clone());
        }

        self.primary_store
            .insert(hash, (compressed.clone(), chunk.len() as u64));
        Ok(compressed)
    }
}
