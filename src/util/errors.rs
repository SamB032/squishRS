use std::error::Error;
use std::fmt;

pub type AppError = Box<dyn Error>;

#[derive(Debug)]
/// Represents errors related to file system input/output operations.
///
/// This enum encapsulates errors that occur while reading directories or directory entries,
/// wrapping the underlying `std::io::Error`.
pub enum CustomErr {
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

impl fmt::Display for CustomErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CustomErr::ReadDirError(_e) => write!(f, "Directory not found"),
            CustomErr::ReadEntryError(_e) => write!(f, "File Entity not found"),
            CustomErr::WriterError(_e) => write!(f, "Error writing to squish"),
            CustomErr::ReaderError(_e) => write!(f, "Error reading from squish"),
            CustomErr::FlushError(_) => write!(f, "Failed to flush archive writer"),
            CustomErr::LockPoisoned => write!(f, "Writer mutex was poisoned"),
            CustomErr::SenderError(_e) => write!(f, "Error sending to writer channel"),
            CustomErr::EncoderError(_e) => write!(f, "Error with zstd encoder"),
            CustomErr::CreateDirError(_e) => write!(f, "Error with creating directory"),
            CustomErr::CreateFileError(_e) => write!(f, "Error with creating file"),
            CustomErr::FileNotExist(_e) => write!(f, "Specified file does not exist"),
        }
    }
}

impl Error for CustomErr {
    /// Formats the error for user-friendly display.
    ///
    /// Rather than printing the full underlying I/O error, it prints a simplified message:
    /// - `ReadDirError` results in "Directory not found"
    /// - `ReadEntryError` results in "File Entity not found"
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CustomErr::ReadDirError(e) => Some(e),
            CustomErr::ReadEntryError(e) => Some(e),
            CustomErr::WriterError(e) => Some(e),
            CustomErr::ReaderError(e) => Some(e),
            CustomErr::FlushError(e) => Some(e),
            CustomErr::LockPoisoned => None,
            CustomErr::SenderError(e) => Some(&**e),
            CustomErr::EncoderError(e) => Some(e),
            CustomErr::CreateDirError(e) => Some(e),
            CustomErr::CreateFileError(e) => Some(e),
            CustomErr::FileNotExist(e) => Some(e),
        }
    }
}
