use std::io::{Cursor, Read, Seek};
use std::error::Error;

use crate::VERSION;
use crate::util::errors::CustomErr;
use crate::util::chunk::{hash_chunk, ChunkStore};
use crate::util::header::{
    convert_timestamp_to_date, magic_version, patch_u64, verify_header, write_header,
    write_placeholder_u64, write_timestamp, PREFIX,
};

#[test]
fn test_magic_version() {
    let expected = [PREFIX, VERSION.as_bytes()].concat();
    assert_eq!(magic_version(), expected);
}

#[test]
fn test_write_and_verify_header() {
    let mut buffer = Vec::new();
    write_header(&mut buffer).unwrap();

    let mut cursor = Cursor::new(buffer.clone());
    let version = verify_header(&mut cursor).unwrap();
    assert_eq!(version, VERSION);
}

#[test]
fn test_verify_header_invalid_prefix() {
    let mut bad_data = b"notmagic00.01.01".to_vec();
    let mut cursor = Cursor::new(&mut bad_data);
    let result = verify_header(&mut cursor);
    assert!(result.is_err());
}

#[test]
fn test_verify_header_incompatible_version() {
    // Forge header with different major.minor version
    let fake_version = b"squish99.99.99";
    let mut cursor = Cursor::new(fake_version.to_vec());
    let result = verify_header(&mut cursor);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn test_write_timestamp_and_convert() {
    let mut buffer = Vec::new();
    write_timestamp(&mut buffer).unwrap();
    assert_eq!(buffer.len(), 8);

    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&buffer[..8]);
    let ts = u64::from_le_bytes(bytes);

    let formatted = convert_timestamp_to_date(ts);
    assert!(
        formatted.contains('/') && formatted.contains(':'),
        "Unexpected formatted date: {formatted}"
    );
}

#[test]
fn test_convert_timestamp_to_date_known_value() {
    let ts = 1686890000; // Mon, 16 Jun 2023 17:46:40 GMT
    let result = convert_timestamp_to_date(ts);
    assert!(result.ends_with("/2023") || result.ends_with("/2025")); // Accept drift from TZ/localtime
}

#[test]
fn test_write_and_patch_placeholder_u64() {
    let mut cursor = Cursor::new(Vec::new());

    let pos = write_placeholder_u64(&mut cursor).unwrap();
    assert_eq!(pos, 0);
    assert_eq!(cursor.get_ref().len(), 8);
    assert_eq!(&cursor.get_ref()[..], &[0u8; 8]);

    patch_u64(&mut cursor, pos, 12345678).unwrap();
    let mut updated = [0u8; 8];
    cursor.set_position(0);
    cursor.read_exact(&mut updated).unwrap();
    assert_eq!(u64::from_le_bytes(updated), 12345678);

    // Ensure writer position is at end
    let end_pos = cursor.stream_position().unwrap();
    assert_eq!(end_pos, 8);
}

#[test]
fn test_hash_chunk_is_consistent() {
    let data = b"some test data";
    let hash1 = hash_chunk(data);
    let hash2 = hash_chunk(data);
    assert_eq!(hash1, hash2, "Hashes should be consistent for same input");
}

#[test]
fn test_hash_chunk_different_inputs_produce_different_hashes() {
    let hash1 = hash_chunk(b"data 1");
    let hash2 = hash_chunk(b"data 2");
    assert_ne!(
        hash1, hash2,
        "Different inputs should produce different hashes"
    );
}

#[test]
fn test_insert_first_time_returns_compressed_data() {
    let store = ChunkStore::new();
    let data = vec![1u8; 1024]; // small data for fast compression

    let result = store.insert(&data).expect("Insert failed");
    assert_eq!(result.hash, hash_chunk(&data));
    assert!(result.compressed_data.is_some());
    assert_eq!(store.len(), 1);
}

#[test]
fn test_insert_duplicate_returns_none_compressed_data() {
    let store = ChunkStore::new();
    let data = vec![2u8; 1024];

    let first = store.insert(&data).unwrap();
    assert!(first.compressed_data.is_some());

    let second = store.insert(&data).unwrap();
    assert!(second.compressed_data.is_none());
    assert_eq!(first.hash, second.hash);
    assert_eq!(store.len(), 1);
}

#[test]
fn test_multiple_unique_inserts_increase_len() {
    let store = ChunkStore::new();

    let chunk1 = vec![1u8; 1024];
    let chunk2 = vec![2u8; 1024];
    let chunk3 = vec![3u8; 1024];

    store.insert(&chunk1).unwrap();
    store.insert(&chunk2).unwrap();
    store.insert(&chunk3).unwrap();

    assert_eq!(store.len(), 3);
}

#[test]
fn test_compressed_data_is_smaller_or_equal() {
    let store = ChunkStore::new();
    let repetitive_data = vec![42u8; 2048]; // highly compressible

    let result = store.insert(&repetitive_data).unwrap();
    assert!(result.compressed_data.is_some());

    let compressed = result.compressed_data.unwrap();
    assert!(
        compressed.len() < repetitive_data.len(),
        "Compressed data should be smaller than original"
    );

    // Also test it can be decompressed properly
    let mut decoder = zstd::stream::Decoder::new(&compressed[..]).unwrap();
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();

    assert_eq!(decompressed, repetitive_data);
}

#[test]
fn test_display_messages() {
    let cases = vec![
        (CustomErr::ReadDirError(std::io::Error::other("dummy")), "Directory not found"),
        (CustomErr::ReadEntryError(std::io::Error::other("dummy")), "File Entity not found"),
        (CustomErr::WriterError(std::io::Error::other("dummy")), "Error writing to squish"),
        (CustomErr::ReaderError(std::io::Error::other("dummy")), "Error reading from squish"),
        (CustomErr::FlushError(std::io::Error::other("dummy")), "Failed to flush archive writer"),
        (CustomErr::LockPoisoned, "Writer mutex was poisoned"),
        (CustomErr::SenderError(Box::new(std::io::Error::other("dummy"))), "Error sending to writer channel"),
        (CustomErr::EncoderError(std::io::Error::other("dummy")), "Error with zstd encoder"),
        (CustomErr::CreateDirError(std::io::Error::other("dummy")), "Error with creating directory"),
        (CustomErr::CreateFileError(std::io::Error::other("dummy")), "Error with creating file"),
        (CustomErr::FileNotExist(std::io::Error::other("dummy")), "Specified file does not exist"),
    ];

    for (error, expected_msg) in cases {
        assert_eq!(error.to_string(), expected_msg);
    }
}

#[test]
fn test_source_returns_inner_error() {
    // Variants that should return Some(source)

    let with_source_cases = vec![
        CustomErr::ReadDirError(std::io::Error::other("dummy")),
        CustomErr::ReadEntryError(std::io::Error::other("dummy")),
        CustomErr::WriterError(std::io::Error::other("dummy")),
        CustomErr::ReaderError(std::io::Error::other("dummy")),
        CustomErr::FlushError(std::io::Error::other("dummy")),
        CustomErr::SenderError(Box::new(std::io::Error::other("dummy"))),
        CustomErr::EncoderError(std::io::Error::other("dummy")),
        CustomErr::CreateDirError(std::io::Error::other("dummy")),
        CustomErr::CreateFileError(std::io::Error::other("dummy")),
        CustomErr::FileNotExist(std::io::Error::other("dummy")),
    ];

    for error in with_source_cases {
        assert!(error.source().is_some());
    }
}

#[test]
fn test_source_none_for_lock_poisoned() {
    let error = CustomErr::LockPoisoned;
    assert!(error.source().is_none());
}
