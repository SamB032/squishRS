mod archive;
mod cmd;
mod fsutil;
mod util;

use crate::archive::{ArchiveReader, ArchiveWriter};
use crate::cmd::progress_bar::{create_progress_bar, create_spinner};
use crate::cmd::{build_list_summary_table, format_bytes, Cli, Commands};
use crate::fsutil::directory::walk_dir;

use clap::Parser;
use colored::*;
use indicatif::ProgressBar;
use rayon::{ThreadPoolBuildError, ThreadPoolBuilder};
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
            cap_max_threads(max_threads).expect("Failed to Build Rayon Thread Pool");

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
                        pb.finish_and_clear();
                        eprintln!("{}: {e}", "Failed to create ArchiveWriter".red());
                        std::process::exit(1);
                    });

            let compressed_size = archive_writer.pack(&files).unwrap_or_else(|e| {
                pb.finish_and_clear();
                eprintln!("{}: {e}", "Failed to pack".red());
                std::process::exit(1);
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

            let mut archive_reader = ArchiveReader::new(Path::new(&squish)).unwrap_or_else(|e| {
                eprint!("{}: {}", "Failed to setup file reader".red(), e);
                std::process::exit(1)
            });

            let summary = match archive_reader.get_summary() {
                Ok(summary) => summary,
                Err(e) => {
                    eprint!("{}: {}", "Failed to list files".red(), e);
                    std::process::exit(1)
                }
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

            let mut archive_reader = ArchiveReader::new(Path::new(&squish)).unwrap_or_else(|e| {
                eprint!("{}: {}", "Failed to setup file reader".red(), e);
                std::process::exit(1)
            });

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
                Err(e) => {
                    eprintln!("{}: {e}", "Failed to unpack".red());
                    std::process::exit(1);
                }
            };
        }
    }
}

/// Configures the global Rayon thread pool to use at most `max_number_of_threads` threads.
///
/// This function attempts to build and initialize the global Rayon thread pool with the
/// specified maximum number of threads. It should be called once early in the program
/// before any parallel computations occur.
///
/// # Arguments
///
/// * `max_number_of_threads` - The maximum number of worker threads to use in the Rayon thread pool.
///
/// # Returns
///
/// * `Ok(())` if the global thread pool was successfully initialized.
/// * `Err(rayon::ThreadPoolBuildError)` if the thread pool has already been initialized or the build fails.
///
/// # Examples
///
/// ```
/// cap_max_threads(8)?;
/// ```
/// # Note
///
/// This function can only be called once per process; subsequent calls will return an error.
fn cap_max_threads(max_number_of_threads: usize) -> Result<(), ThreadPoolBuildError> {
    ThreadPoolBuilder::new()
        .num_threads(max_number_of_threads)
        .build_global()
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
