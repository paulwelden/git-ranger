use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::sync::Arc;

/// A progress tracker for repository operations
pub struct ProgressTracker {
    multi_progress: Arc<MultiProgress>,
    main_bar: Option<ProgressBar>,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            multi_progress: Arc::new(MultiProgress::new()),
            main_bar: None,
        }
    }

    /// Create a new progress tracker that doesn't show any progress (for dry-run or quiet mode)
    /// This uses a hidden draw target to prevent any terminal output
    pub fn hidden() -> Self {
        let multi = MultiProgress::with_draw_target(ProgressDrawTarget::hidden());
        Self {
            multi_progress: Arc::new(multi),
            main_bar: None,
        }
    }

    /// Initialize the main progress bar for repository operations
    pub fn init_main_bar(&mut self, total: u64, message: &str) {
        let bar = self.multi_progress.add(ProgressBar::new(total));
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .expect("Invalid progress bar template")
                .progress_chars("=>-"),
        );
        bar.set_message(message.to_string());
        self.main_bar = Some(bar);
    }

    /// Increment the main progress bar by 1
    pub fn inc(&self) {
        if let Some(bar) = &self.main_bar {
            bar.inc(1);
        }
    }

    /// Finish the main progress bar with a completion message
    pub fn finish_with_message(&self, message: &str) {
        if let Some(bar) = &self.main_bar {
            bar.finish_with_message(message.to_string());
        }
    }

    /// Create a spinner for operations that don't have a known total
    pub fn create_spinner(&self, message: &str) -> ProgressBar {
        let spinner = self.multi_progress.add(ProgressBar::new_spinner());
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("Invalid spinner template")
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        spinner
    }

    /// Finish a spinner with a completion message
    pub fn finish_spinner(&self, spinner: ProgressBar, message: &str) {
        spinner.finish_with_message(message.to_string());
    }

    /// Create a sub-progress bar for individual operations (like git clone/fetch)
    /// This shows an indeterminate progress bar for operations where we can't track exact progress.
    /// When a main bar exists, the sub-bar is inserted before it so the main bar stays visible
    /// at the bottom of the terminal during multi-repo operations.
    pub fn create_sub_progress(&self, message: &str) -> ProgressBar {
        let pb = if let Some(main) = &self.main_bar {
            self.multi_progress
                .insert_before(main, ProgressBar::new_spinner())
        } else {
            self.multi_progress.add(ProgressBar::new_spinner())
        };
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("  {spinner:.cyan} {msg}")
                .expect("Invalid spinner template")
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    }

    /// Finish a sub-progress bar, clearing it from the display so completed entries
    /// do not accumulate and push the main bar off screen.
    pub fn finish_sub_progress(&self, pb: ProgressBar) {
        pb.finish_and_clear();
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_tracker_creation() {
        let tracker = ProgressTracker::new();
        assert!(tracker.main_bar.is_none());
    }

    #[test]
    fn test_progress_tracker_init() {
        let mut tracker = ProgressTracker::new();
        tracker.init_main_bar(10, "Testing");
        assert!(tracker.main_bar.is_some());
    }

    #[test]
    fn test_hidden_tracker() {
        let tracker = ProgressTracker::hidden();
        assert!(tracker.main_bar.is_none());
    }

    #[test]
    fn test_progress_tracker_default() {
        let tracker = ProgressTracker::default();
        assert!(tracker.main_bar.is_none());
    }

    #[test]
    fn test_create_spinner() {
        let tracker = ProgressTracker::new();
        let spinner = tracker.create_spinner("Test spinner");
        // Spinner should be created successfully
        assert!(!spinner.is_finished());
    }

    #[test]
    fn test_finish_spinner() {
        let tracker = ProgressTracker::new();
        let spinner = tracker.create_spinner("Test spinner");
        tracker.finish_spinner(spinner.clone(), "Complete");
        // After finishing, spinner should be done
        assert!(spinner.is_finished());
    }

    #[test]
    fn test_create_sub_progress() {
        let tracker = ProgressTracker::new();
        let sub_progress = tracker.create_sub_progress("Sub operation");
        // Sub-progress should be created successfully
        assert!(!sub_progress.is_finished());
    }

    #[test]
    fn test_finish_sub_progress() {
        let tracker = ProgressTracker::new();
        let sub_progress = tracker.create_sub_progress("Sub operation");
        tracker.finish_sub_progress(sub_progress.clone());
        // After finishing, sub-progress should be done
        assert!(sub_progress.is_finished());
    }

    #[test]
    fn test_main_bar_increment() {
        let mut tracker = ProgressTracker::new();
        tracker.init_main_bar(5, "Processing");

        // Increment should work without panicking
        tracker.inc();
        tracker.inc();

        // Main bar should still exist
        assert!(tracker.main_bar.is_some());
    }

    #[test]
    fn test_finish_main_bar() {
        let mut tracker = ProgressTracker::new();
        tracker.init_main_bar(5, "Processing");
        tracker.finish_with_message("Complete");

        // Main bar should still exist but be finished
        assert!(tracker.main_bar.is_some());
        if let Some(bar) = &tracker.main_bar {
            assert!(bar.is_finished());
        }
    }

    #[test]
    fn test_hidden_tracker_no_output() {
        let tracker = ProgressTracker::hidden();
        let spinner = tracker.create_spinner("Test");
        tracker.finish_spinner(spinner, "Done");
        // Should complete without error even though it's hidden
    }

    #[test]
    fn test_inc_without_main_bar() {
        let tracker = ProgressTracker::new();
        // Should not panic when incrementing without a main bar
        tracker.inc();
    }

    #[test]
    fn test_finish_without_main_bar() {
        let tracker = ProgressTracker::new();
        // Should not panic when finishing without a main bar
        tracker.finish_with_message("Done");
    }

    #[test]
    fn test_create_sub_progress_with_main_bar() {
        let mut tracker = ProgressTracker::new();
        tracker.init_main_bar(5, "Main task");
        // With main_bar present, create_sub_progress uses insert_before
        let sub = tracker.create_sub_progress("Sub operation");
        assert!(!sub.is_finished());
        tracker.finish_sub_progress(sub.clone());
        assert!(sub.is_finished());
    }

    #[test]
    fn test_hidden_tracker_full_lifecycle() {
        let mut tracker = ProgressTracker::hidden();
        let spinner = tracker.create_spinner("Discovering...");
        tracker.finish_spinner(spinner.clone(), "Done discovering");
        assert!(spinner.is_finished());

        tracker.init_main_bar(3, "Syncing");
        let sub = tracker.create_sub_progress("Cloning repo-1");
        tracker.finish_sub_progress(sub);

        tracker.inc();
        tracker.inc();
        tracker.inc();
        tracker.finish_with_message("All done");

        assert!(tracker.main_bar.as_ref().unwrap().is_finished());
    }

    #[test]
    fn test_multiple_inc_advances_position() {
        let mut tracker = ProgressTracker::new();
        tracker.init_main_bar(5, "Processing");
        tracker.inc();
        tracker.inc();

        let bar = tracker.main_bar.as_ref().unwrap();
        assert_eq!(bar.position(), 2);
    }
}
