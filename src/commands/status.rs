use crate::commands::common::{self, CommonError};
use crate::config::RepoConfig;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StatusError {
    #[error("{0}")]
    Common(#[from] CommonError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct StatusOptions {
    pub config_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RepoStatus {
    pub name: String,
    pub local_path: PathBuf,
    pub cloned: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StatusReport {
    pub total_repos: usize,
    pub repos_cloned: usize,
    pub repos_not_cloned: usize,
    pub repos: Vec<RepoStatus>,
}

impl StatusReport {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn status_command(options: &StatusOptions) -> Result<StatusReport, StatusError> {
    let config = common::load_config(&options.config_path)?;
    let base_dir = options
        .config_path
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let mut report = StatusReport::new();

    // Analyze standalone repos
    for repo_config in config.get_standalone_repos() {
        let repo_status = analyze_repo_status(repo_config, base_dir);

        if repo_status.cloned {
            report.repos_cloned += 1;
        } else {
            report.repos_not_cloned += 1;
        }

        report.repos.push(repo_status);
        report.total_repos += 1;
    }

    // Print status report
    write_status_report(&report, &mut std::io::stdout());

    Ok(report)
}

fn analyze_repo_status(repo_config: &RepoConfig, base_dir: &Path) -> RepoStatus {
    let repo_name = common::extract_repo_name(&repo_config.url);
    let local_path = common::build_local_path(repo_config, base_dir, &repo_name);

    // Check if repo is cloned (has .git directory)
    let git_dir = local_path.join(".git");
    let cloned = git_dir.exists();

    RepoStatus {
        name: repo_name,
        local_path,
        cloned,
    }
}

fn write_status_report(report: &StatusReport, w: &mut impl std::io::Write) {
    writeln!(w, "\n=== Repository Status ===").ok();
    writeln!(w, "Total repositories: {}", report.total_repos).ok();
    writeln!(w, "Cloned: {}", report.repos_cloned).ok();
    writeln!(w, "Not cloned: {}", report.repos_not_cloned).ok();
    writeln!(w).ok();

    if report.repos.is_empty() {
        writeln!(w, "No repositories configured.").ok();
        return;
    }

    for repo in &report.repos {
        let status_icon = if repo.cloned { "✓" } else { "✗" };
        let status_text = if repo.cloned { "cloned" } else { "not cloned" };

        writeln!(
            w,
            "{} {} - {} ({})",
            status_icon,
            repo.name,
            status_text,
            repo.local_path.display()
        ).ok();
    }

    writeln!(w).ok();
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_analyze_repo_status_cloned() {
        let temp = assert_fs::TempDir::new().unwrap();
        let repo_dir = temp.path().join("my-repo");
        std::fs::create_dir_all(repo_dir.join(".git")).unwrap();

        let repo_config = RepoConfig {
            url: "https://github.com/example/my-repo.git".to_string(),
            local_dir: None,
        };
        let status = analyze_repo_status(&repo_config, temp.path());
        assert!(status.cloned);
    }

    #[test]
    fn test_analyze_repo_status_not_cloned() {
        let temp = assert_fs::TempDir::new().unwrap();

        let repo_config = RepoConfig {
            url: "https://github.com/example/my-repo.git".to_string(),
            local_dir: None,
        };
        let status = analyze_repo_status(&repo_config, temp.path());
        assert!(!status.cloned);
    }

    #[test]
    fn test_analyze_repo_status_name_extraction() {
        let temp = assert_fs::TempDir::new().unwrap();

        let repo_config = RepoConfig {
            url: "git@github.com:org/cool-tool.git".to_string(),
            local_dir: None,
        };
        let status = analyze_repo_status(&repo_config, temp.path());
        assert_eq!(status.name, "cool-tool");
    }

    #[test]
    fn test_status_report_new_defaults() {
        let report = StatusReport::new();
        assert_eq!(report.total_repos, 0);
        assert_eq!(report.repos_cloned, 0);
        assert_eq!(report.repos_not_cloned, 0);
        assert!(report.repos.is_empty());
    }

    #[test]
    fn test_status_report_new_equals_default() {
        let new_report = StatusReport::new();
        let default_report = StatusReport::default();
        assert_eq!(new_report.total_repos, default_report.total_repos);
        assert_eq!(new_report.repos_cloned, default_report.repos_cloned);
        assert_eq!(new_report.repos_not_cloned, default_report.repos_not_cloned);
        assert_eq!(new_report.repos.len(), default_report.repos.len());
    }

    #[test]
    fn test_status_error_common_display() {
        let inner = CommonError::ConfigNotFound("/path".to_string());
        let err = StatusError::Common(inner);
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {}", msg);
    }

    #[test]
    fn test_status_error_io_display() {
        let err = StatusError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file missing",
        ));
        let msg = err.to_string();
        assert!(msg.contains("IO error"), "got: {}", msg);
    }

    #[test]
    fn test_write_status_report_empty() {
        let report = StatusReport::new();
        let mut buf = Vec::new();
        write_status_report(&report, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Repository Status"), "got: {}", output);
        assert!(output.contains("No repositories configured"), "got: {}", output);
    }

    #[test]
    fn test_write_status_report_with_repos() {
        let report = StatusReport {
            total_repos: 2,
            repos_cloned: 1,
            repos_not_cloned: 1,
            repos: vec![
                RepoStatus {
                    name: "repo-a".to_string(),
                    local_path: PathBuf::from("/tmp/repo-a"),
                    cloned: true,
                },
                RepoStatus {
                    name: "repo-b".to_string(),
                    local_path: PathBuf::from("/tmp/repo-b"),
                    cloned: false,
                },
            ],
        };
        let mut buf = Vec::new();
        write_status_report(&report, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Total repositories: 2"), "got: {}", output);
        assert!(output.contains("Cloned: 1"), "got: {}", output);
        assert!(output.contains("Not cloned: 1"), "got: {}", output);
        assert!(output.contains("repo-a"), "got: {}", output);
        assert!(output.contains("cloned"), "got: {}", output);
        assert!(output.contains("repo-b"), "got: {}", output);
        assert!(output.contains("not cloned"), "got: {}", output);
    }
}
