mod archive;
mod cmd;
mod fsutil;

use crate::archive::{list_squish, pack_squish, unpack_squish};
use crate::cmd::progress_bar::{create_listing_files_spinner, create_progress_bar};
use crate::cmd::{build_list_summary_table, format_bytes, Cli, Commands};
use crate::fsutil::walk_dir;

use clap::Parser;
use colored::*;
use std::path::Path;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack { input, output } => {
            // Default filename.out if output is not given
            let output = output.unwrap_or_else(|| format!("{}.squish", input));

            let files_spinner = create_listing_files_spinner("Finding Files");

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
            let pb = create_progress_bar(files.len() as u64, "Packing");

            // Package file to archive
            let compressed_size =
                match pack_squish(Path::new(&input), Path::new(&output), &files, &pb) {
                    Ok(compressed_size) => {
                        pb.finish_and_clear();
                        compressed_size
                    }
                    Err(e) => {
                        eprintln!("{}: {e}", "Failed to pack".red());
                        std::process::exit(1);
                    }
                };

            let display_output = output.strip_prefix("./").unwrap_or(&output);

            println!(
                "{} Saved as {} \nCompressed to {}",
                "Packing complete!".green(),
                display_output,
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

            let files_spinner = create_listing_files_spinner("Unpacking Files");
            // TODO, see how we could set up a progress bar

            match unpack_squish(Path::new(&squish), Path::new(&output)) {
                Ok(_) => {
                    files_spinner.finish_and_clear();
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
