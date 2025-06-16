use crate::archive::list::ListSummary;
use byte_unit::{Byte, UnitType};
use clap::{Parser, Subcommand};
use num_format::{Locale, ToFormattedString};
use prettytable::{format::consts::FORMAT_NO_LINESEP_WITH_TITLE, row, Cell, Row, Table};
use std::collections::HashMap;

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
        output: Option<String>,
    },

    /// List contents of a .squish archive
    List {
        archive: String,
        #[arg(long, default_value_t = false)]
        simple: bool,
    },

    /// Unpack files from a .squish archive
    Unpack {
        archive: String,
        #[clap(short, long)]
        file: Option<String>,
        #[clap(short, long, default_value = ".")]
        output: String,
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
/// * `summary` - A reference to a `ListSummary` struct containing the archive metadata,
///               including file paths, sizes, chunk counts, and compression stats.
///
/// # Example
///
/// ```rust
/// print_list_summary_table(&summary);
/// ```
pub fn print_list_summary_table(summary: &ListSummary) {
    // -- Summary Table --
    let mut summary_table = Table::new();
    summary_table.set_format(*FORMAT_NO_LINESEP_WITH_TITLE);

    println!("\nSquish breakdown:");

    // Set title
    summary_table.set_titles(Row::new(vec![Cell::new("Archive Summary").with_hspan(2)]));

    summary_table.add_row(row!["Archive size", format_bytes(summary.archive_size)]);
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
        "Number of Chunks",
        summary.unique_chunks.to_formatted_string(&Locale::en)
    ]);
    summary_table.printstd();

    // Breakdown by top-level directory
    let mut dir_counts: HashMap<String, usize> = HashMap::new();

    for file_path in &summary.files {
        // Extract first path component
        let top_level = file_path.path.split('/').next().unwrap_or("");
        *dir_counts.entry(top_level.to_string()).or_insert(0) += 1;
    }

    println!("\nTop-level directory breakdown:");

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

    breakdown_table.printstd();
}

/// Convert bytes into a more human readable form
fn format_bytes(bytes: u64) -> String {
    let byte = Byte::from_u128(bytes as u128);
    let unit = byte.unwrap().get_appropriate_unit(UnitType::Decimal);
    format!("{:.2} {}", unit.get_value(), unit.get_unit())
}
