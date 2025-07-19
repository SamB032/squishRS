use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

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
/// use squishrs::cmd::progress_bar::create_progress_bar;
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

/// Creates and configures a spinner-style progress bar for displaying file listing progress.
///
/// The spinner updates every 500 milliseconds and cycles through a sequence of dots to indicate activity.
///
/// # Arguments
///
/// * `message` - A static string slice used as the message prefix displayed alongside the spinner.
///
/// # Returns
///
/// * `ProgressBar` - A configured `ProgressBar` spinner instance ready for use.
///
/// # Example
///
/// ```
/// use squishrs::cmd::progress_bar::create_spinner;
/// let pb = create_spinner("Scanning files");
/// pb.finish_with_message("Done scanning files");
/// ```
pub fn create_spinner(message: &'static str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_message(message);
    pb.enable_steady_tick(Duration::from_millis(500)); // update spinner every 500ms
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[".", "..", "...", "...."])
            .template("{msg} {spinner}")
            .unwrap(),
    );
    pb
}
