mod cli;
mod fsutil;
mod pack;
mod progress;

use crate::cli::{Cli, Commands};
use crate::fsutil::walk_dir;
use crate::progress::create_progress_bar;
use clap::Parser;
use colored::*;
use std::path::Path;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack { input, output } => {
            // Default filename.out if output is not given
            let output = output.unwrap_or_else(|| format!("{}.squish", input));

            // Count total files for progress bar
            let files = match walk_dir(Path::new(&input)) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("{}: {}", "Failed to list files".red(), e);
                    std::process::exit(1);
                }
            };

            // Setup progress bar
            let pb = create_progress_bar(files.len() as u64, "Packing");

            // Package file to archive
            let reduction = match pack::pack_directory(Path::new(&input), Path::new(&output), &files, &pb) {
                Ok(reduction) => {
                    pb.finish_and_clear();
                    reduction
                }
                Err(e) => {
                    eprintln!("{}: {e}", "Failed to pack".red());
                    std::process::exit(1);
                }
            };

            let display_output = output.strip_prefix("./").unwrap_or(&output);

            println!(
                "{} Saved as {}. Compression Ratio was {:.2}%",
                "Packing complete!".green(),
                display_output,
                reduction
            );
        }
        Commands::List { archive } => {
            println!("Listing contents of '{}'", archive);
        }
        Commands::Extract {
            archive,
            file,
            output,
        } => match &file {
            Some(f) => println!("Extracting '{}' from '{}' to '{}'", f, archive, output),
            None => println!("Extracting all files from '{}' to '{}'", archive, output),
        },
    }
}
