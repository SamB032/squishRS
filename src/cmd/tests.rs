#[cfg(test)]
mod tests {
    use crate::{create_listing_files_spinner, create_progress_bar, build_list_summary_table};
    use crate::archive::list::ListSummary;
    use super::super::format_bytes;

    #[test]
    fn test_create_progress_bar_basic() {
        let length = 10;
        let message = "Test progress";
        let pb = create_progress_bar(length, message);
        assert_eq!(pb.length(), Some(length));
        assert_eq!(pb.message(), message);

        // Increment and finish to ensure no panic
        pb.inc(1);
        pb.finish_with_message("Done");
    }

    #[test]
    fn test_create_listing_files_spinner_basic() {
        let message = "Scanning";
        let pb = create_listing_files_spinner(message);
        assert_eq!(pb.message(), message);

        // The spinner should tick without panicking
        pb.tick();
        pb.finish_with_message("Finished");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0.00 B");
        assert_eq!(format_bytes(500), "500.00 B");
        assert_eq!(format_bytes(1500), "1.50 KB");
        assert_eq!(format_bytes(1_500_000), "1.50 MB");
    }

    #[test]
    fn test_build_list_summary_table() {
        let summary = ListSummary { 
            unique_chunks: 32, 
            total_original_size: 100, 
            archive_size: 20, 
            reduction_percentage: 80.0, 
            squish_creation_date: "DATE".to_string(), 
            files: Vec::new(),
        };
        let output = build_list_summary_table(&summary);

        assert!(output.contains("Squash Summary"));
        assert!(output.contains("Compressed size"));
        assert!(output.contains("Original size"));
        assert!(output.contains("Number of files"));
        assert!(output.contains("Number of chunks"));
        assert!(output.contains("Top-level directory breakdown"));
    }
}


