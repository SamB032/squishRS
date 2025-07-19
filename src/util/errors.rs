use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to read directory {0}: {1}")]
    ReadDirError(String, #[source] io::Error),

    #[error("Failed to read entry in `{0}`: {1}")]
    ReadEntryError(PathBuf, #[source] io::Error),

    #[error("Error writing to squish: {0}")]
    WriterError(#[source] io::Error),

    #[error("Error reading from squish: {0}")]
    ReaderError(#[source] io::Error),

    #[error("Failed to flush archive writer: {0}")]
    FlushError(#[source] io::Error),

    #[error("Compression error")]
    Compression,

    #[error("Archive format error: {0}")]
    Archive(String),

    #[error("Zstd encoder error: {0}")]
    EncoderError(#[source] io::Error),

    #[error("Mutex poisoned")]
    LockPoisoned,

    #[error("Error sending to writer thread: {0}")]
    SenderError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Error creating directory `{0}`: {1}")]
    CreateDirError(PathBuf, #[source] io::Error),

    #[error("Error creating file `{0}`: {1}")]
    CreateFileError(PathBuf, #[source] io::Error),

    #[error("Specified file does not exist: `{0}`")]
    FileNotExist(PathBuf),

    #[error("Illegal UTF8 detected")]
    IllegalUTF8,

    #[error("Missing Chunk for File: `{0}`")]
    MissingChunk(PathBuf),

    #[error("Unable to Cap Maximum Threads: {0}")]
    CapThreadsError(#[source] rayon::ThreadPoolBuildError),

    #[error("Invalid timestamp in squish: {0}")]
    InvalidTimeStamp(#[source] io::Error),

    #[error("Unknown error: {0}")]
    Other(String),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AppError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        AppError::Other(e.to_string())
    }
}
