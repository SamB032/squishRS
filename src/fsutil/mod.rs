use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Recursively walks a directory and returns a vector of all file paths found.
///
/// # Arguments
///
/// * `path` - A reference to a Path to walk.
///
/// # Returns
///
/// * `io::Result<Vec<PathBuf>>` - Vector of file paths on success, or an I/O error.
///
/// # Examples
///
/// ```
/// use squish::fsutil::walk_dir;
/// use std::path::Path;
/// let files = walk_dir(Path::new(".")).unwrap();
/// println!("Found {} files", files.len());
/// ```
pub fn walk_dir(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !path.exists() {
        // Path does not exist — return an error
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Path does not exist",
        ));
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                files.extend(walk_dir(&path)?);
            } else {
                files.push(path);
            }
        }
    } else {
        files.push(path.to_path_buf());
    }
    Ok(files)
}

#[cfg(test)]
mod tests;
