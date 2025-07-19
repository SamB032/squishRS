use std::fs::{self, File};
use std::io::{Read, Write};

use crate::archive::ArchiveWriter;
use crate::util::errors::AppError;
use crate::util::header::verify_header;
use crate::VERSION;

use tempfile::{tempdir, NamedTempFile};

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
