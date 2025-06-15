mod cli;

use clap::Parser;
use crate::cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pack { input, output } => {
            println!("Packing '{}' into '{}'", input, output);
        }
        Commands::List { archive } => {
            println!("Listing contents of '{}'", archive);
        }
        Commands::Extract { archive, file, output } => {
            match &file {
                Some(f) => println!("Extracting '{}' from '{}' to '{}'", f, archive, output),
                None => println!("Extracting all files from '{}' to '{}'", archive, output),
            }
        }
    }
}
