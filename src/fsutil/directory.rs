use std::fs;
use std::path::{Path, PathBuf};

use rayon::iter::Either;
use rayon::prelude::*;

use crate::util::errors::AppError;

/// Recursively walks a directory and returns a vector of all file paths found.
///
/// This function performs an iterative breadth-first traversal of the directory tree starting
/// at the given `path`. It collects directory entries and processes them in parallel using
/// Rayon to improve performance when traversing large directory hierarchies.
///
/// # Arguments
///
/// * `path` - A reference to a `Path` representing the root directory to walk.
///
/// # Returns
///
/// * `Result<Vec<PathBuf>, AppError>` - On success, returns a vector containing the paths of all
///   files found recursively under `path`. On failure, returns a custom application error
///   wrapping underlying I/O errors.
///
/// # Errors
///
/// Returns a `FileIOError::ReadDirError` if the root directory cannot be read, or
/// `FileIOError::ReadEntryError` if individual directory entries cannot be accessed.
///
/// # Examples
///
/// ```rust
/// use squishrs::fsutil::directory::walk_dir;
/// use std::path::Path;
///
/// let files = walk_dir(Path::new(".")).expect("Failed to walk directory");
/// println!("Found {} files", files.len());
/// ```
pub fn walk_dir(path: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut stack = vec![path.to_path_buf()];
    let mut files = Vec::new();

    while let Some(dir) = stack.pop() {
        // Collect all Dir entries into a vector
        let entries = fs::read_dir(&dir)
            .map_err(|e| AppError::ReadDirError(dir.display().to_string(), e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::ReadEntryError(dir.clone(), e))?;

        // Process each entry concurrently
        let (dirs, regular_files): (Vec<_>, Vec<_>) = entries
            .into_par_iter()
            .map(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    (Some(path), None)
                } else {
                    (None, Some(path))
                }
            })
            .partition_map(|(dir, file)| match (dir, file) {
                (Some(d), None) => Either::Left(d),
                (None, Some(f)) => Either::Right(f),
                _ => unreachable!(),
            });

        // Update for next iteration
        stack.extend(dirs);
        files.extend(regular_files);
    }

    Ok(files)
}
