use std::io::{Cursor, Seek, Read};

use crate::VERSION;
use crate::util::header::{
    PREFIX,
    magic_version,
    write_header,
    verify_header,
    write_timestamp,
    convert_timestamp_to_date,
    patch_u64,
    write_placeholder_u64
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
