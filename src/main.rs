mod archive;
mod cmd;
mod fsutil;
mod util;

use crate::archive::{ArchiveReader, ArchiveWriter};
use crate::cmd::progress_bar::{create_progress_bar, create_spinner};
use crate::cmd::{build_list_summary_table, format_bytes, Cli, Commands};
use crate::fsutil::directory::walk_dir;
use crate::util::errors::AppError;

use clap::Parser;
use colored::*;
use rayon::{ThreadPoolBuildError, ThreadPoolBuilder};
use std::path::Path;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), AppError> {
    let cli = Cli::parse();

    // Cap the number of threads globally that can spawn
    cap_max_threads(cli.max_threads).map_err(|e| AppError::CapThreadsError(e))?;

    match cli.command {
        Commands::Pack { input, output } => {
            //Remove ending front and back slashes from input
            let trimmed_input = input.trim_end_matches(&['/', '\\'][..]).to_string();

            // Default filename.out if output is not given
            let output = output.unwrap_or_else(|| format!("{input}.squish"));

            let files_spinner = create_spinner("Finding Files");

            // Count total files for progress bar
            let files = walk_dir(Path::new(&trimmed_input))?;           
            files_spinner.finish_and_clear();

            // Setup progress bar
            let mut pb = create_progress_bar(files.len() as u64, "Packing");

            // Package file to archive
            let mut archive_writer =
                ArchiveWriter::new(Path::new(&input), Path::new(&output), Some(&mut pb))?;

            let compressed_size = archive_writer.pack(&files)?;
            pb.finish_and_clear();

            println!(
                "{}\nCompressed to {}\n{}: {}",
                "Packing complete!".green(),
                output.strip_prefix("./").unwrap_or(&output),
                "Final archive size".blue(),
                format_bytes(compressed_size)
            );
        }
        Commands::List { squish, simple } => {
            let discovery_spinner = create_spinner("Scanning Squish");

            let mut archive_reader = ArchiveReader::new(Path::new(&squish))?;

            let summary = archive_reader.get_summary()?;
            discovery_spinner.finish_and_clear();

            if simple {
                // Make it machine readable, could be piped to fzf
                println!(
                    "squish_size(bytes): {}, original_size(bytes): {}, reduction: {:.2}%, number_of_files: {}, chunks_count: {}",
                    summary.archive_size,
                    summary.total_original_size,
                    summary.reduction_percentage,
                    summary.files.len(),
                    summary.unique_chunks
                );

                println!("{:>10}  File Path", "Size (Bytes)");
                println!("----------  --------------------");
                for file in summary.files {
                    println!("{:>10}  {}", file.original_size, file.path);
                }
            } else {
                let output = build_list_summary_table(&summary);
                println!("{output}");
            }
        }
        Commands::Unpack { squish, output } => {
            // Default filename.squish if output is not given
            let output = output.unwrap_or_else(|| {
                squish
                    .strip_suffix(".squish")
                    .unwrap_or(&squish)
                    .to_string()
            });

            let mut pb = create_progress_bar(0, "Reading Chunks");

            let mut archive_reader = ArchiveReader::new(Path::new(&squish))?;

            archive_reader.unpack(Path::new(&output), Some(&mut pb))?;
            pb.finish_and_clear();
            println!(
                "{}\n{} was unsquished into /{}",
                "Unpacking complete!".green(),
                squish,
                output
            );
        }
    }

    Ok(())
}

/// Configures the global Rayon thread pool to use at most `max_number_of_threads` threads.
///
/// This function attempts to initialize the global Rayon thread pool with a specified maximum
/// number of threads. It must be called **before** any parallel computations are triggered
/// in the application. If the thread pool is already initialized, this function will return
/// an error.
///
/// # Arguments
///
/// * `max_number_of_threads` - The maximum number of worker threads to use in the Rayon thread pool.
///
/// # Returns
///
/// * `Ok(())` if the thread pool was successfully initialized.
/// * `Err(AppError)` if the thread pool was already initialized or failed to build.
///
/// # Errors
///
/// Returns an `AppError` wrapping a `rayon::ThreadPoolBuildError` if the thread pool has
/// already been set or the configuration fails.
///
/// # Examples
///
/// ```no_run
/// use squish::parallel::cap_max_threads;
///
/// cap_max_threads(8).expect("Failed to configure thread pool");
/// ```
///
/// # Note
///
/// This function can only be called once per process. All subsequent attempts will return an error.
fn cap_max_threads(max_number_of_threads: usize) -> Result<(), ThreadPoolBuildError> {
    ThreadPoolBuilder::new()
        .num_threads(max_number_of_threads)
        .build_global()?;
    Ok(())
}
