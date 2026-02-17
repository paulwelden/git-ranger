use indicatif::{ProgressBar, ProgressStyle, MultiProgress, ProgressDrawTarget};
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
}
