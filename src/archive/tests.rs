use std::fs::{self, File};
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use crate::archive::{ArchiveReader, ArchiveWriter};
use crate::util::errors::AppError;
use crate::util::header::{
    patch_u64, verify_header, write_header, write_placeholder_u64, write_timestamp,
};
use crate::VERSION;

use tempfile::{tempdir, NamedTempFile};

pub fn create_dummy_archive<W: Write + Seek>(
    writer: &mut W,
) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    // Write header
    write_header(writer)?;

    // Write current timestamp
    write_timestamp(writer)?;

    // Write number of chunks (placeholder, will patch later)
    let chunk_count_pos = write_placeholder_u64(writer)?;

    // --- Chunk Section ---
    let chunk_data = b"test";
    let chunk_hash = [1u8; 16];
    let original_size = chunk_data.len() as u64;

    let compressed_chunk = zstd::encode_all(Cursor::new(chunk_data), 0)?;
    let compressed_size = compressed_chunk.len() as u64;

    writer.write_all(&chunk_hash)?;
    writer.write_all(&original_size.to_le_bytes())?;
    writer.write_all(&compressed_size.to_le_bytes())?;
    writer.write_all(&compressed_chunk)?;

    // Patch chunk count (1)
    patch_u64(writer, chunk_count_pos, 1)?;

    // --- File Section ---
    let file_count = 1u32;
    writer.write_all(&file_count.to_le_bytes())?;

    // File metadata
    let path_bytes = b"file1.txt";
    let path_len = path_bytes.len() as u32;
    writer.write_all(&path_len.to_le_bytes())?;
    writer.write_all(path_bytes)?;

    writer.write_all(&original_size.to_le_bytes())?; // File size
    writer.write_all(&1u32.to_le_bytes())?; // Chunk count
    writer.write_all(&chunk_hash)?; // Chunk hash

    // Return dummy file content for testing purposes
    Ok(vec![("file1.txt".to_string(), chunk_data.to_vec())])
}

#[test]
fn test_archive_writer_basic() -> Result<(), AppError> {
    // Create temp input directory
    let input_dir = tempdir()?;
    let input_path = input_dir.path();

    // Create some test files
    let file1_path = input_path.join("file1.txt");
    let mut file1 = File::create(&file1_path)?;
    writeln!(file1, "Hello, world!")?;

    let file2_path = input_path.join("file2.txt");
    let mut file2 = File::create(&file2_path)?;
    writeln!(file2, "This is a test file.")?;

    // Create output file path
    let output_path = input_dir.path().join("archive.squish");

    // Initialize ArchiveWriter
    let mut writer = ArchiveWriter::new(input_path, &output_path, None)?;

    // Collect files to pack
    let files = vec![file1_path.clone(), file2_path.clone()];

    // Pack files into archive
    let archive_size = writer.pack(&files)?;
    assert!(archive_size > 0, "Archive should not be empty");

    // Optional: Verify archive file exists and is non-zero
    let metadata = fs::metadata(&output_path)?;
    assert_eq!(metadata.len(), archive_size);

    Ok(())
}

#[test]
fn test_archive_writer_new() -> Result<(), AppError> {
    // Create temp dir
    let temp_dir = tempdir()?;
    let temp_file = NamedTempFile::new()?;

    let _archive_writer = ArchiveWriter::new(temp_dir.path(), temp_file.path(), None)?;

    // Open the file and verify headers are written as expected
    let mut file = File::open(temp_file.path())?;
    let version_str = verify_header(&mut file)?;

    let mut timestamp_bytes = [0u8; 8];
    file.read_exact(&mut timestamp_bytes)?;
    assert_eq!(version_str, VERSION);

    let timestamp = u64::from_le_bytes(timestamp_bytes);
    assert!(timestamp > 0, "Timestamp should be non-zero");

    Ok(())
}

#[test]
fn test_archive_reader_get_summary() -> Result<(), AppError> {
    let dir = tempdir()?;
    let archive_path = dir.path().join("dummy.squish");

    // Create the dummy archive
    let mut file = File::create(&archive_path)?;
    let _files = create_dummy_archive(&mut file);
    file.flush()?;
    file.rewind()?; // Important: reset cursor to start

    let mut reader = ArchiveReader::new(&archive_path)?;
    let summary = reader.get_summary()?;

    assert_eq!(summary.unique_chunks, 1);
    assert_eq!(summary.total_original_size, 4);
    assert!(summary.archive_size > 0);
    assert!(summary.compression_ratio <= 0.0);
    assert_eq!(summary.files.len(), 1);
    assert_eq!(summary.files[0].path, "file1.txt");

    Ok(())
}

#[test]
fn test_archive_reader_unpack() -> Result<(), AppError> {
    let dir = tempdir()?;
    let archive_path = dir.path().join("dummy.squish");

    // Create the dummy archive
    let mut file = File::create(&archive_path)?;
    let files = create_dummy_archive(&mut file)?;
    file.flush()?;
    file.rewind()?; // Important: reset cursor to start

    let output_dir = dir.path().join("output");

    let mut reader = ArchiveReader::new(&archive_path)?;
    reader.unpack(&output_dir, None)?;

    // Check if file is correctly restored
    for (filename, contents) in files {
        let restored_path = output_dir.join(filename);
        assert!(restored_path.exists());
        let restored_data = fs::read(restored_path)?;
        assert_eq!(restored_data, contents);
    }

    Ok(())
}

#[test]
fn test_invalid_file_path_reader() {
    let res = ArchiveReader::new(Path::new("nonexistent.squish"));
    assert!(matches!(res, Err(AppError::FileNotExist(_))));
}
