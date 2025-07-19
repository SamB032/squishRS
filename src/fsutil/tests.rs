use crossbeam::channel::unbounded;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::fsutil::directory::walk_dir;
use crate::fsutil::writer::{writer_thread, ChunkMessage, ThreadSafeWriter};

use tempfile::{tempdir, tempfile};

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

    let result = walk_dir(&file_path);
    assert!(result.is_err());
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

#[test]
fn test_writer_thread_happy_path() {
    // Setup in-memory writer
    let buffer = Vec::new();
    let writer = Cursor::new(buffer);

    // Setup channel and send a ChunkMessage
    let (tx, rx) = unbounded();

    let hash = [1u8; 16];
    let data = Arc::new(vec![2u8; 10]);
    let original_size = 10u64;

    tx.send(ChunkMessage {
        hash,
        compressed_data: data.clone(),
        original_size,
    })
    .unwrap();

    drop(tx); // Close channel to end the loop

    // Run writer_thread
    writer_thread(writer, rx).unwrap();
}

#[test]
fn test_thread_safe_writer_new() {
    // Create a temporary file
    let file = tempfile().expect("Failed to create temp file");

    let buf_writer = BufWriter::new(file);
    let arc_writer = Arc::new(Mutex::new(buf_writer));

    let ts_writer = ThreadSafeWriter::new(arc_writer.clone());

    // Ensure the inner Arc is the same
    assert!(Arc::ptr_eq(&ts_writer.writer, &arc_writer));
}

#[test]
fn test_thread_safe_writer_write_and_flush() {
    // Create a temporary file
    let mut temp_file = tempfile().expect("Failed to create temp file");

    // Wrap it in BufWriter and ThreadSafeWriter
    let buf_writer = BufWriter::new(temp_file.try_clone().unwrap());
    let arc_writer = Arc::new(Mutex::new(buf_writer));
    let mut ts_writer = ThreadSafeWriter::new(arc_writer);

    // Write data
    let data = b"hello world";
    ts_writer.write_all(data).unwrap();
    ts_writer.flush().unwrap();

    // Read back the data from the file to verify it was written
    let mut output = Vec::new();
    temp_file.seek(SeekFrom::Start(0)).unwrap(); // Reset file cursor
    temp_file.read_to_end(&mut output).unwrap();

    assert_eq!(&output[..], data);
}
