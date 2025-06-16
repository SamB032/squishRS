use std::io::{Read, Write};

pub const MAGIC_VERSION: &[u8] = b"squish000101";

/// Write the header to a archive file
///
/// # arguments
///
/// * 'writer' - writer instance of the archive file
///
/// # returns
///
/// * 'std::io::Result<()>' - Error indicating issue writing to the file
///
/// # examples
///
/// ```
/// chunk::write_header(&mut writer);
/// ```
pub fn write_header<W: Write>(writer: &mut W) -> std::io::Result<()> {
    writer.write_all(MAGIC_VERSION)
}

/// Verify the header of an archive
///
/// # arguments
///
/// * 'reader' - reader instance of the archive file
///
/// # returns
///
/// * 'std::io::Result<()>' - Error indicating the archive header is invalid
///
/// # examples
///
/// ```
/// chunk::verify_header(&mut writer);
/// ```
pub fn verify_header<R: Read>(reader: &mut R) -> std::io::Result<()> {
    let mut header = vec![0u8; 12];
    reader.read_exact(&mut header)?;

    if &header != MAGIC_VERSION {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid archive header",
        ))
    } else {
        Ok(())
    }
}
