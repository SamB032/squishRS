use indicatif::ProgressBar;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::path::PathBuf;
use zstd::stream::write::Encoder;

/// Packs a directory's files into a single compressed archive.
///
/// This function traverses a list of files relative to the `input_dir`, reads each file's contents,
/// compresses it using zstd compression, and writes metadata along with the compressed data
/// to the specified `output_file`. The metadata includes the relative file path length and bytes,
/// the original uncompressed file size, and the compressed data size. A progress bar is updated
/// to reflect the packing progress.
///
/// # Arguments
///
/// * `input_dir` - The root directory path containing the files to be packed.
/// * `output_file` - The path where the resulting archive file will be created.
/// * `files` - A vector of `PathBuf` representing all files to pack (should be relative to `input_dir`).
/// * `pb` - A reference to a `ProgressBar` to display packing progress.
///
/// # Returns
///
/// Returns `Ok(())` if packing succeeds, or an error boxed as `Box<dyn std::error::Error>`
/// if any file operation or compression step fails.
///
/// # Errors
///
/// This function will return an error if:
/// - The output file cannot be created or written to.
/// - Any file cannot be read from disk.
/// - Compression or IO operations fail.
///
/// # Example
///
/// ```no_run
/// use indicatif::ProgressBar;
/// use std::path::PathBuf;
///
/// let files: Vec<PathBuf> = vec![ /* ... list of files ... */ ];
/// let pb = ProgressBar::new(files.len() as u64);
/// pack_directory(Path::new("input_dir"), Path::new("archive.squish"), &files, &pb)?;
/// pb.finish_with_message("Packing complete");
/// ```
pub fn pack_directory(
    input_dir: &Path,
    output_file: &Path,
    files: &Vec<PathBuf>,
    pb: &ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    // Set the output file to write to
    let output = fs::File::create(output_file)?;
    let mut writer = BufWriter::new(output);

    writer.write_all(b"SQUISHR01")?; // magic + version

    for file_path in files {
        // Get relative path
        let rel_path = file_path.strip_prefix(input_dir)?;
        let rel_path_str = rel_path.to_string_lossy();

        // Read file data
        let data = fs::read(&file_path)?;

        // Write path length and path bytes
        let path_bytes = rel_path_str.as_bytes();
        let path_len = path_bytes.len() as u32;
        writer.write_all(&path_len.to_le_bytes())?;
        writer.write_all(path_bytes)?;

        // Write original file size
        let orig_size = data.len() as u64;
        writer.write_all(&orig_size.to_le_bytes())?;

        // Compress into a temporary buffer to get compressed size
        let mut compressed_buf = Vec::new();
        {
            let mut encoder = Encoder::new(&mut compressed_buf, 0)?;
            encoder.write_all(&data)?;
            encoder.finish()?;
        }

        // Write compressed size and compressed data
        let compressed_size = compressed_buf.len() as u64;
        writer.write_all(&compressed_size.to_le_bytes())?;
        writer.write_all(&compressed_buf)?;

        pb.inc(1); // Increment progress bar
    }

    Ok(())
}
