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
use indicatif::ProgressBar;
use rayon::ThreadPoolBuilder;
use std::path::Path;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack {
            input,
            output,
            max_threads,
        } => {
            //Remove ending front and back slashes from input
            let trimmed_input = input.trim_end_matches(&['/', '\\'][..]).to_string();

            // Default filename.out if output is not given
            let output = output.unwrap_or_else(|| format!("{input}.squish"));

            let files_spinner = create_spinner("Finding Files");

            // Cap the number of threads that can spawn
            cap_max_threads(max_threads).unwrap_or_else(|e| {
                exit_with_error("Failed to list files", Some(&files_spinner), &*e)
            });

            // Count total files for progress bar
            let files = walk_dir(Path::new(&trimmed_input)).unwrap_or_else(|e| {
                exit_with_error("Failed to list files", Some(&files_spinner), &*e)
            });
            files_spinner.finish_and_clear();

            // Setup progress bar
            let mut pb = create_progress_bar(files.len() as u64, "Packing");

            // Package file to archive
            let mut archive_writer =
                ArchiveWriter::new(Path::new(&input), Path::new(&output), Some(&mut pb))
                    .unwrap_or_else(|e| {
                        exit_with_error("Failed to Initalise squish", Some(&pb), &*e)
                    });

            let compressed_size = archive_writer.pack(&files).unwrap_or_else(|e| {
                exit_with_error("Failed to compress into squish", Some(&pb), &*e)
            });

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

            let mut archive_reader = ArchiveReader::new(Path::new(&squish))
                .unwrap_or_else(|e| exit_with_error("Failed to setup file reader", None, &*e));

            let summary = match archive_reader.get_summary() {
                Ok(summary) => summary,
                Err(e) => exit_with_error("Failed to list files", None, &*e),
            };
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

            let mut archive_reader = ArchiveReader::new(Path::new(&squish))
                .unwrap_or_else(|e| exit_with_error("Failed to setup file reader", Some(&pb), &*e));

            match archive_reader.unpack(Path::new(&output), Some(&mut pb)) {
                Ok(_) => {
                    pb.finish_and_clear();
                    println!(
                        "{}\n{} was unsquished into /{}",
                        "Unpacking complete!".green(),
                        squish,
                        output
                    );
                }
                Err(e) => exit_with_error("Failed to unpack", Some(&pb), &*e),
            };
        }
    }
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
fn cap_max_threads(max_number_of_threads: usize) -> Result<(), AppError> {
    ThreadPoolBuilder::new()
        .num_threads(max_number_of_threads)
        .build_global()
        .map_err(|e| e.into())
}

/// Handles a fatal error by optionally finishing a progress bar, printing an error message, and exiting the program.
///
/// # Parameters
/// - `msg`: A short context message describing what operation failed.
/// - `progress_bar`: An optional reference to a `ProgressBar` that will be finished and cleared if present.
/// - `err`: The error that caused the failure; printed alongside the context message.
///
/// # Behavior
/// If a progress bar is provided, it is finished and cleared before printing the error message.
/// The error message is printed to standard error with the context message in red.
/// Finally, the program terminates immediately with exit code 1.
///
/// # Panics
/// This function does not panic; it always terminates the program.
///
/// # Examples
/// ```no_run
/// use indicatif::ProgressBar;
///
/// fn example(progress_bar: Option<&ProgressBar>, err: &(dyn std::error::Error)) -> ! {
///     exit_with_error("Failed to complete operation", progress_bar, err);
/// }
/// ```
fn exit_with_error(
    msg: &str,
    progress_bar: Option<&ProgressBar>,
    err: &(dyn std::error::Error),
) -> ! {
    if let Some(progress_bar) = progress_bar {
        progress_bar.finish_and_clear();
    }
    eprintln!("{}: {}", msg.red(), err);
    std::process::exit(1);
}
