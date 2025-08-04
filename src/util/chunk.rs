use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use std::sync::Arc;
use xxhash_rust::xxh3::xxh3_128;
use zstd::bulk::compress;

use crate::util::errors::AppError;

pub type ChunkHash = [u8; 16];

pub const CHUNK_SIZE: usize = 2048 * 1024; // 2MB
const COMPRESSION_LEVEL: i32 = 12;

pub struct InsertReturn {
    pub hash: ChunkHash,
    pub compressed_data: Option<Arc<Vec<u8>>>,
}

#[derive(Clone)]
pub struct ChunkStore {
    pub primary_store: PrimaryStore,
}

type PrimaryStore = Arc<DashMap<ChunkHash, ()>>;
type ReturnInsertChunk = Result<InsertReturn, Box<dyn std::error::Error + Send + Sync>>;

/// Calculates the hash of a binary array
///
/// # arguments
///
/// * 'data' - binary array
///
/// # returns
///
/// Return an array of type '[u8;16]' representing the 16 bit hash
///
/// # examples
///
/// ```rust
/// use squishrs::util::chunk::hash_chunk;
///
/// let chunk_buf = b"example chunk data";
/// let hash = hash_chunk(chunk_buf);
/// println!("Chunk hash: {:?}", hash);
/// ```
pub fn hash_chunk(chunk: &[u8]) -> ChunkHash {
    let hash = xxh3_128(chunk);
    hash.to_le_bytes()
}

impl ChunkStore {
    pub fn new() -> Self {
        ChunkStore {
            primary_store: Arc::new(DashMap::new()),
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
    /// Returns the hash of the chunk if OK
    ///
    /// # Errors
    ///
    /// Returns an error if compression or writing to the encoder fails.
    pub fn insert(&self, chunk: &[u8]) -> ReturnInsertChunk {
        let hash = hash_chunk(chunk);

        match self.primary_store.entry(hash) {
            Entry::Occupied(_) => Ok(InsertReturn {
                hash,
                compressed_data: None,
            }),
            Entry::Vacant(entry) => {
                let compressed =
                    compress(chunk, COMPRESSION_LEVEL).map_err(|_| AppError::Compression)?;

                entry.insert(());

                Ok(InsertReturn {
                    hash,
                    compressed_data: Some(Arc::new(compressed)),
                })
            }
        }
    }

    /// Returns the number of entries currently stored in the `ChunkStore`.
    ///
    /// # Returns
    ///
    /// * `u64` - The count of key-value pairs in the underlying `primary_store`.
    ///
    /// # Example
    ///
    /// ```
    /// use squishrs::util::chunk::ChunkStore;
    ///
    /// let store = ChunkStore::new();
    /// assert_eq!(store.len(), 0);
    /// ```
    pub fn len(&self) -> u64 {
        self.primary_store.len() as u64
    }

    /// Returns true if the chunkstore is empty
    ///
    /// # Returns
    ///
    /// * `bool` - true if chunkstore is empty, false otherwise
    ///
    /// # Example
    ///
    /// ```
    /// use squishrs::util::chunk::ChunkStore;
    ///
    /// let store = ChunkStore::new();
    /// assert_eq!(store.is_empty(), true);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ChunkStore {
    fn default() -> Self {
        Self::new()
    }
}
