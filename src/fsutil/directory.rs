use std::fs;
use std::path::{Path, PathBuf};

use rayon::iter::Either;
use rayon::prelude::*;

use crate::util::errors::{AppError, FileIOError};

/// Recursively walks a directory in parallel and returns a vector of all file paths found.
///
/// This function uses an iterative breadth-first approach combined with Rayon for parallel
/// traversal of subdirectories, improving performance on large directory trees.
///
/// # Arguments
///
/// * `path` - A reference to a `Path` representing the root directory to walk.
///
/// # Returns
///
/// * `io::Result<Vec<PathBuf>>` - A vector of all discovered file paths on success, or an I/O error.
///
/// # Examples
///
/// ```
/// use squish::fsutil::walk_dir;
/// use std::path::Path;
/// let files = walk_dir(Path::new(".")).unwrap();
/// println!("Found {} files", files.len());
/// ```
pub fn walk_dir(path: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut stack = vec![path.to_path_buf()];
    let mut files = Vec::new();

    while let Some(dir) = stack.pop() {
        // Collect all Dir entries into a vector
        let entries = fs::read_dir(&dir)
            .map_err(FileIOError::ReadDirError)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(FileIOError::ReadEntryError)?;

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
