mod archive;
mod cmd;
mod fsutil;
mod util;

use crate::archive::ArchiveWriter;
use crate::cmd::progress_bar::{create_listing_files_spinner, create_progress_bar};
use crate::cmd::{build_list_summary_table, format_bytes, Cli, Commands};
use crate::fsutil::walk_dir;
use crate::util::{list_squish, unpack_squish};

use clap::Parser;
use colored::*;
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
            // Default filename.out if output is not given
            let output = output.unwrap_or_else(|| format!("{}.squish", input));

            let files_spinner = create_listing_files_spinner("Finding Files");

            // Cap the number of threads that can spawn
            cap_max_threads(max_threads).expect("Failed to Build Rayon Thread Pool");

            // Count total files for progress bar
            let files = match walk_dir(Path::new(&input)) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("{}: {}", "Failed to list files".red(), e);
                    std::process::exit(1);
                }
            };
            files_spinner.finish_and_clear();

            // Setup progress bar
            let mut pb = create_progress_bar(files.len() as u64, "Packing");

            // Package file to archive
            let archive_writer =
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
            let summary = match list_squish(Path::new(&squish)) {
                Ok(summary) => summary,
                Err(e) => {
                    eprint!("{}: {}", "Failed to list files".red(), e);
                    std::process::exit(1)
                }
            };

            if simple {
                // Make it machine readable, could be piped to fzf
                println!(
                    "squish_size(bytes): {}, original_size(bytes): {}, reduction: {:.2}%, number_of_files: {}, chunks_count: {}",
                    summary.archive_size, summary.total_original_size, summary.reduction_percentage, summary.files.len(), summary.unique_chunks
                );

                println!("{:>10}  File Path", "Size (Bytes)");
                println!("----------  --------------------");
                for file in summary.files {
                    println!("{:>10}  {}", file.original_size, file.path);
                }
            } else {
                let output = build_list_summary_table(&summary);
                println!("{}", output);
            }
        }
        Commands::Unpack { squish, output } => {
            // Default filename.out if output is not given
            let output = output.unwrap_or_else(|| {
                squish
                    .strip_suffix(".squish")
                    .unwrap_or(&squish)
                    .to_string()
            });

            let mut pb = create_progress_bar(0, "Reading Chunks");

            match unpack_squish(Path::new(&squish), Path::new(&output), Some(&mut pb)) {
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
