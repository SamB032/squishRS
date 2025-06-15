use indicatif::{ProgressBar, ProgressStyle};

/// Creates and returns a configured progress bar with a custom message.
///
/// # Arguments
///
/// * `length` - The total length (count) of the progress bar (e.g., total number of items to process).
/// * `message` - A static string slice that will be displayed as the message prefix for the progress bar.
///
/// # Returns
///
/// A `ProgressBar` instance from the `indicatif` crate, styled with a cyan/blue bar, showing progress,
/// position, total length, and estimated time remaining.
///
/// # Example
///
/// ```
/// let pb = create_progress_bar(100, "Processing");
/// for i in 0..100 {
///     pb.inc(1);
/// }
/// pb.finish_with_message("Done");
/// ```
pub fn create_progress_bar(length: u64, message: &'static str) -> ProgressBar {
    let pb = ProgressBar::new(length);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("=> "),
    );
    pb.set_message(message);
    pb
}
