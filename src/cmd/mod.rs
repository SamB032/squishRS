pub mod progress_bar;

use std::collections::HashMap;

use crate::archive::reader::ArchiveSummary;
use byte_unit::{Byte, UnitType};
use clap::{Parser, Subcommand};
use num_format::{Locale, ToFormattedString};
use prettytable::{format::consts::FORMAT_NO_LINESEP_WITH_TITLE, row, Cell, Row, Table};

#[derive(Parser)]
#[clap(name = "squishrs")]
#[clap(about = "Compact, compress, and deduplicate files into a single archive")]
pub struct Cli {
    /// Maximum number of threads to use
    #[arg(long = "max-threads", short = 'j', default_value_t = 30, global = true)]
    pub max_threads: usize,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
#[command(name = "squish", version, about = "A CLI tool to pack and unpack .squish archives", long_about = None)]
pub enum Commands {
    /// Pack a directory into a .squish archive
    #[command(
        about = "Pack a directory",
        long_about = "Compress and deduplicate a directory into a .squish archive file"
    )]
    Pack {
        input: String,
        #[clap(short, long)]
        output: Option<String>,
    },

    /// List contents of a .squish archive
    #[command(
        about = "List files in an archive",
        long_about = "Display the contents of a .squish archive"
    )]
    List {
        squish: String,
        #[arg(long, default_value_t = false)]
        simple: bool,
    },

    /// Unpack files from a .squish archive
    #[command(
        about = "Extract archive contents",
        long_about = "Unpacks all files from a .squish archive into a target directory"
    )]
    Unpack {
        squish: String,
        #[clap(short, long)]
        output: Option<String>,
    },
}

/// Prints a summary table of the archive contents including overall statistics
/// and a detailed breakdown of files grouped by their top-level directory.
///
/// The summary table includes:
/// - Archive size
/// - Original total size
/// - Compression reduction percentage
/// - Number of files
/// - Number of unique chunks
///
/// After the summary, the function prints a "Top-level directory breakdown"
/// table that shows the count of files grouped by the first path component,
/// providing insight into the archive's directory structure.
///
/// # Arguments
///
/// * `summary` - A reference to a `ArchiveSummary` struct containing the archive metadata,
///   including file paths, sizes, chunk counts, and compression stats.
///
/// # Example
///
/// ```rust
/// use squishrs::cmd::build_list_summary_table;
/// use squishrs::archive::reader::ArchiveSummary;
///
/// let summary = ArchiveSummary {
///     unique_chunks: 10,
///     total_original_size: 5000,
///     archive_size: 3500,
///     reduction_percentage: 30.0,
///     squish_creation_date: "2025-07-19".to_string(),
///     squish_version: "1.0".to_string(),
///     files: vec![], // empty for example
/// };
///
/// build_list_summary_table(&summary);
/// ```
pub fn build_list_summary_table(summary: &ArchiveSummary) -> String {
    let mut output = Vec::new();

    // -- Summary Table --
    output.push("\nSquash breakdown:".to_string());
    let mut summary_table = Table::new();
    summary_table.set_format(*FORMAT_NO_LINESEP_WITH_TITLE);

    // Set title
    summary_table.set_titles(Row::new(vec![Cell::new("Squash Summary").with_hspan(2)]));

    summary_table.add_row(row!["Creation Date (UTC)", summary.squish_creation_date]);
    summary_table.add_row(row!["Squish Version", summary.squish_version]);
    summary_table.add_row(row!["Compressed size", format_bytes(summary.archive_size)]);
    summary_table.add_row(row![
        "Original size",
        format_bytes(summary.total_original_size)
    ]);
    summary_table.add_row(row![
        "Reduction Percentage",
        format!("{:.1}%", summary.reduction_percentage)
    ]);
    summary_table.add_row(row![
        "Number of files",
        summary.files.len().to_formatted_string(&Locale::en)
    ]);
    summary_table.add_row(row![
        "Number of chunks",
        summary.unique_chunks.to_formatted_string(&Locale::en)
    ]);

    output.push(summary_table.to_string());

    // Breakdown by top-level directory
    let mut dir_counts: HashMap<String, usize> = HashMap::new();

    for file_path in &summary.files {
        // Extract first path component
        let top_level = file_path.path.split('/').next().unwrap_or("");
        *dir_counts.entry(top_level.to_string()).or_insert(0) += 1;
    }

    output.push("\nTop-level directory breakdown:".to_string());

    let mut breakdown_table = Table::new();
    breakdown_table.set_format(*FORMAT_NO_LINESEP_WITH_TITLE);
    breakdown_table.set_titles(Row::new(vec![
        Cell::new("Directory").style_spec("bFc"),
        Cell::new("File Count").style_spec("bFc"),
    ]));

    // Sort directories by file count descending
    let mut dir_counts_vec: Vec<_> = dir_counts.into_iter().collect();
    dir_counts_vec.sort_by(|a, b| b.1.cmp(&a.1));

    for (dir, count) in dir_counts_vec {
        breakdown_table.add_row(row![dir, count.to_formatted_string(&Locale::en)]);
    }
    output.push(breakdown_table.to_string());

    output.join("\n")
}

/// Convert bytes into a more human readable form
pub fn format_bytes(bytes: u64) -> String {
    let byte = Byte::from_u128(bytes as u128);
    let unit = byte.unwrap().get_appropriate_unit(UnitType::Decimal);
    format!("{:.2} {}", unit.get_value(), unit.get_unit())
}

#[cfg(test)]
mod tests;
