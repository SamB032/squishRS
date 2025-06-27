use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::{io::Write, sync::Arc};
use zstd::stream::Encoder;

pub type ChunkHash = [u8; 32];

pub const CHUNK_SIZE: usize = 2048 * 1024; // 2MB
const COMPRESSION_LEVEL: i32 = 15;

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
/// Return an array of type '[u8;32]' representing the 32 bit hash
///
/// # examples
///
/// ```
/// chunk::hash_chunk(&chunk_buf);
/// ```
pub fn hash_chunk(chunk: &[u8]) -> ChunkHash {
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
                let mut compressed = Vec::new();
                {
                    let mut encoder = Encoder::new(&mut compressed, COMPRESSION_LEVEL)?;
                    encoder.write_all(chunk)?;
                    encoder.finish()?;
                }

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
    /// let store = ChunkStore::new();
    /// assert_eq!(store.len(), 0);
    /// ```
    pub fn len(&self) -> u64 {
        self.primary_store.len() as u64
    }
}
