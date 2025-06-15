use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(name = "squishrs")]
#[clap(about = "Compact, compress, and deduplicate files into a single archive")]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Pack a directory into a .squish archive
    Pack {
        input: String,
        #[clap(short, long)]
        output: String,
    },

    /// List contents of a .squish archive
    List {
        archive: String,
    },

    /// Extract files from a .squish archive
    Extract {
        archive: String,
        #[clap(short, long)]
        file: Option<String>,
        #[clap(short, long, default_value = ".")]
        output: String,
    },
}
