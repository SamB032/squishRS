use std::fs::{self, File};
use std::path::Path;
use tempfile::tempdir;

use squish::fsutil::walk_dir;

#[test]
fn test_nonexistent_path() {
    let path = Path::new("nonexistent_path");
    let result = walk_dir(path);
    assert!(result.is_err());
}

#[test]
fn test_path_is_file() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("file.txt");
    File::create(&file_path).unwrap();

    let files = walk_dir(&file_path).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], file_path);
}

#[test]
fn test_empty_directory() {
    let dir = tempdir().unwrap();

    let files = walk_dir(dir.path()).unwrap();
    assert!(files.is_empty());
}

#[test]
fn test_directory_with_files() {
    let dir = tempdir().unwrap();
    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");
    File::create(&file1).unwrap();
    File::create(&file2).unwrap();

    let mut files = walk_dir(dir.path()).unwrap();
    files.sort();
    let mut expected = vec![file1, file2];
    expected.sort();

    assert_eq!(files, expected);
}

#[test]
fn test_directory_with_nested_subdirs() {
    let dir = tempdir().unwrap();

    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    let file1 = dir.path().join("file1.txt");
    let file2 = subdir.join("file2.txt");

    File::create(&file1).unwrap();
    File::create(&file2).unwrap();

    let mut files = walk_dir(dir.path()).unwrap();
    files.sort();

    let mut expected = vec![file1, file2];
    expected.sort();

    assert_eq!(files, expected);
}
