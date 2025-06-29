use std::error::Error;
use std::fmt;

pub type AppError = Box<dyn Error>;

#[derive(Debug)]
pub enum FileIOError {
    ReadDirError(std::io::Error),
    ReadEntryError(std::io::Error),
}

impl fmt::Display for FileIOError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileIOError::ReadDirError(_e) => write!(f, "Directory not found"),
            FileIOError::ReadEntryError(_e) => write!(f, "File Entity not found"),
        }
    }
}

impl Error for FileIOError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FileIOError::ReadDirError(e) => Some(e),
            FileIOError::ReadEntryError(e) => Some(e),
        }
    }
}
