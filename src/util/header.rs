use std::io::{Read, Write, Seek, SeekFrom};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local, TimeZone};

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

/// Writes the current system time as a little-endian
/// 64-bit unsigned integer representing seconds since the UNIX epoch
/// into the provided writer.
///
/// # Arguments
///
/// * `writer` - A mutable reference to a writer implementing the `Write` trait.
///
/// # Errors
///
/// Returns an `std::io::Error` if writing to the writer fails.
///
/// # Panics
///
/// Panics if the system time is before the UNIX epoch (should not happen on normal systems).
pub fn write_timestamp<W: Write>(writer: &mut W) -> std::io::Result<()> {
    // Get current system time as seconds since UNIX epoch
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before UNIX");
    let timestamp = now.as_secs();

    writer.write_all(&timestamp.to_le_bytes())
}

/// Converts a UNIX timestamp (seconds since epoch) into a formatted
/// local date and time string.
///
/// The returned string is formatted as `"HH:MM DD/MM/YYYY"`.
///
/// # Arguments
///
/// * `timestamp_sec` - The timestamp in seconds since the UNIX epoch.
///
/// # Panics
///
/// Panics if the timestamp is invalid or cannot be converted to a single
/// valid local datetime.
///
/// # Examples
///
/// ```
/// let formatted_date = convert_timestamp_to_date(1686890000);
/// println!("{}", formatted_date); // e.g. "17:49 16/06/2025"
/// ```
pub fn convert_timestamp_to_date(timestamp_sec: u64) -> String {
    let datetime: DateTime<Local> = Local
        .timestamp_opt(timestamp_sec as i64, 0)
        .single()
        .expect("Invalid timestamp");
    datetime.format("%H:%M %d/%m/%Y").to_string()
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

    if header != MAGIC_VERSION {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid archive header",
        ))
    } else {
        Ok(())
    }
}

/// Writes a placeholder `u64` (8 zero bytes) to the writer and returns its stream position.
///
/// This function is useful when the actual value (e.g., number of items written) is not yet known.
/// You can later overwrite this placeholder using [`patch_u64`].
///
/// # Arguments
///
/// * `writer` - A mutable reference to any writer that implements `Write + Seek`.
///
/// # Returns
///
/// * `Ok(u64)` - The byte offset in the stream where the placeholder was written.
/// * `Err` - If writing or getting the stream position fails.
///
/// # Example
///
/// ```rust
/// let pos = write_placeholder_u64(&mut writer)?;
/// // ... later ...
/// patch_u64(&mut writer, pos, actual_value)?;
/// ```
pub fn write_placeholder_u64<W: Write + Seek>(writer: &mut W) -> Result<u64, std::io::Error> {
    let pos = writer.stream_position()?;
    writer.write_all(&0u64.to_le_bytes())?;
    Ok(pos)
}

/// Overwrites a `u64` value at a previously recorded position in the writer stream.
///
/// This is typically used to update a placeholder written earlier with [`write_placeholder_u64`].
/// After writing the value, the stream is moved to the end to resume normal writing.
///
/// # Arguments
///
/// * `writer` - A mutable reference to a writer that implements `Write + Seek`.
/// * `pos` - The byte offset at which to write the new `u64` value.
/// * `value` - The actual `u64` value to write.
///
/// # Returns
///
/// * `Ok(())` - If the patch was successful.
/// * `Err` - If seeking or writing fails.
///
/// # Example
///
/// ```rust
/// patch_u64(&mut writer, pos, 1234)?;
/// ```
pub fn patch_u64<W: Write + Seek>(writer: &mut W, pos: u64, value: u64) -> Result<(), std::io::Error> {
    writer.seek(SeekFrom::Start(pos))?;
    writer.write_all(&value.to_le_bytes())?;
    writer.seek(SeekFrom::End(0))?;
    Ok(())
}
