use std::error::Error;
use std::fmt;

pub type AppError = Box<dyn Error>;

#[derive(Debug)]
/// Represents errors related to file system input/output operations.
///
/// This enum encapsulates errors that occur while reading directories or directory entries,
/// wrapping the underlying `std::io::Error`.
pub enum Err {
    ReadDirError(std::io::Error),
    ReadEntryError(std::io::Error),
    WriterError(std::io::Error),
    ReaderError(std::io::Error),
    FlushError(std::io::Error),
    LockPoisoned,
    SenderError(Box<dyn std::error::Error + Send + Sync>),
    EncoderError(std::io::Error),
    CreateDirError(std::io::Error),
    CreateFileError(std::io::Error),
    FileNotExist(std::io::Error),
}

impl fmt::Display for Err {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Err::ReadDirError(_e) => write!(f, "Directory not found"),
            Err::ReadEntryError(_e) => write!(f, "File Entity not found"),
            Err::WriterError(_e) => write!(f, "Error writing to squish"),
            Err::ReaderError(_e) => write!(f, "Error reading from squish"),
            Err::FlushError(_) => write!(f, "Failed to flush archive writer"),
            Err::LockPoisoned => write!(f, "Writer mutex was poisoned"),
            Err::SenderError(_e) => write!(f, "Error sending to writer channel"),
            Err::EncoderError(_e) => write!(f, "Error with zstd encoder"),
            Err::CreateDirError(_e) => write!(f, "Error with creating directory"),
            Err::CreateFileError(_e) => write!(f, "Error with creating file"),
            Err::FileNotExist(_e) => write!(f, "Specified file does not exist"),
        }
    }
}

impl Error for Err {
    /// Formats the error for user-friendly display.
    ///
    /// Rather than printing the full underlying I/O error, it prints a simplified message:
    /// - `ReadDirError` results in "Directory not found"
    /// - `ReadEntryError` results in "File Entity not found"
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Err::ReadDirError(e) => Some(e),
            Err::ReadEntryError(e) => Some(e),
            Err::WriterError(e) => Some(e),
            Err::ReaderError(e) => Some(e),
            Err::FlushError(e) => Some(e),
            Err::LockPoisoned => None,
            Err::SenderError(e) => Some(&**e),
            Err::EncoderError(e) => Some(e),
            Err::CreateDirError(e) => Some(e),
            Err::CreateFileError(e) => Some(e),
            Err::FileNotExist(e) => Some(e),
        }
    }
}
