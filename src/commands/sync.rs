use crate::commands::common::{self, CommonError};
use crate::config::{RangerConfig, RepoConfig};
use crate::progress::ProgressTracker;
use crate::providers::gitlab::{GitLabClient, GitLabError};
use std::path::{Path, PathBuf};
use thiserror::Error;

// UI symbols for consistent output
const SUCCESS_MARK: &str = "✓";

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("{0}")]
    Common(#[from] CommonError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Git operation failed: {0}")]
    GitError(String),

    #[error("GitLab API error: {0}")]
    GitLabError(#[from] GitLabError),
}

#[derive(Debug, Clone)]
pub struct SyncOptions {
    pub config_path: PathBuf,
    pub target: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Default, Clone)]
pub struct SyncReport {
    pub total_repos: usize,
    pub repos_to_clone: usize,
    pub repos_to_fetch: usize,
    pub repos_cloned: usize,
    pub repos_fetched: usize,
    pub errors: Vec<String>,
}

impl SyncReport {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Information about a repository that needs to be synced
#[derive(Debug, Clone)]
struct RepoSyncInfo {
    url: String,
    name: String,
    local_path: PathBuf,
    exists: bool,
}

pub fn sync_command(options: &SyncOptions) -> Result<SyncReport, SyncError> {
    let config = common::load_config(&options.config_path)?;
    let base_dir = options
        .config_path
        .parent()
        .unwrap_or_else(|| Path::new("."));

    // Initialize progress tracker
    let mut progress = if options.dry_run {
        ProgressTracker::hidden()
    } else {
        ProgressTracker::new()
    };

    // Show discovery spinner
    let discovery_spinner = progress.create_spinner("Discovering repositories...");
    let repos_to_sync = discover_repos(&config, base_dir, options.target.as_deref())?;
    progress.finish_spinner(
        discovery_spinner,
        &format!(
            "{} Discovered {} repositories",
            SUCCESS_MARK,
            repos_to_sync.len()
        ),
    );

    let mut report = build_initial_report(&repos_to_sync);

    if options.dry_run {
        write_dry_run_report(&report, &repos_to_sync, &mut std::io::stdout());
        return Ok(report);
    }

    execute_sync(repos_to_sync, &mut report, &mut progress);
    write_sync_summary(&report, &mut std::io::stdout(), &mut std::io::stderr());

    Ok(report)
}

fn discover_repos(
    config: &RangerConfig,
    base_dir: &Path,
    target: Option<&str>,
) -> Result<Vec<RepoSyncInfo>, SyncError> {
    let mut repos = Vec::new();

    // Add standalone repos
    for repo_config in config.get_standalone_repos() {
        if should_sync_repo(repo_config, target) {
            repos.push(analyze_repo(repo_config, base_dir)?);
        }
    }

    // Add GitLab group repos
    if let Some(gitlab_repos) = discover_gitlab_repos(config, base_dir, target)? {
        repos.extend(gitlab_repos);
    }

    Ok(repos)
}

fn discover_gitlab_repos(
    config: &RangerConfig,
    base_dir: &Path,
    target: Option<&str>,
) -> Result<Option<Vec<RepoSyncInfo>>, SyncError> {
    let gitlab_provider = match &config.providers.gitlab {
        Some(provider) => provider,
        None => return Ok(None),
    };

    let token: String = match gitlab_provider.token.resolve() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Warning: Failed to resolve GitLab token: {}", e);
            eprintln!("         Skipping GitLab groups");
            return Ok(None);
        }
    };

    if token.is_empty() {
        return Ok(None);
    }

    let client = match GitLabClient::new(gitlab_provider.host.clone(), token) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Warning: Failed to create GitLab client: {}", e);
            eprintln!("         Skipping GitLab groups");
            return Ok(None);
        }
    };

    let mut repos = Vec::new();

    for group_config in &config.groups.gitlab {
        if let Some(target_filter) = target {
            if !group_config.name.contains(target_filter) {
                continue;
            }
        }

        println!(
            "Discovering active repositories in GitLab group: {}",
            group_config.name
        );

        match client.get_group_projects(&group_config.name, group_config.recursive) {
            Ok(projects) => {
                println!("  Found {} repositories", projects.len());

                for project in projects {
                    let repo_config = convert_gitlab_project_to_repo_config(
                        &project,
                        &group_config.name,
                        group_config.local_dir.as_deref(),
                    );
                    repos.push(analyze_repo(&repo_config, base_dir)?);
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to get projects for group '{}': {}",
                    group_config.name, e
                );
            }
        }
    }

    Ok(Some(repos))
}

fn convert_gitlab_project_to_repo_config(
    project: &crate::providers::gitlab::GitLabProject,
    group_name: &str,
    base_local_dir: Option<&str>,
) -> RepoConfig {
    let relative_path = if let Some(suffix) = project
        .path_with_namespace
        .strip_prefix(&format!("{}/", group_name))
    {
        suffix
            .rsplit_once('/')
            .map(|(parent, _)| parent.to_string())
    } else {
        None
    };

    let local_dir = if let Some(subpath) = relative_path {
        base_local_dir.map(|base| format!("{}/{}", base, subpath))
    } else {
        base_local_dir.map(String::from)
    };

    RepoConfig {
        url: project.ssh_url_to_repo.clone(),
        local_dir,
    }
}

fn build_initial_report(repos: &[RepoSyncInfo]) -> SyncReport {
    let mut report = SyncReport::new();
    report.total_repos = repos.len();

    for repo in repos {
        if repo.exists {
            report.repos_to_fetch += 1;
        } else {
            report.repos_to_clone += 1;
        }
    }

    report
}

fn execute_sync(repos: Vec<RepoSyncInfo>, report: &mut SyncReport, progress: &mut ProgressTracker) {
    // Initialize progress bar for sync operations
    let total_repos = repos.len() as u64;
    progress.init_main_bar(total_repos, "Syncing repositories");

    for repo in repos {
        if repo.exists {
            match fetch_repo(&repo, progress) {
                Ok(_) => {
                    report.repos_fetched += 1;
                }
                Err(e) => {
                    report
                        .errors
                        .push(format!("Failed to fetch {}: {}", repo.name, e));
                }
            }
        } else {
            match clone_repo(&repo, progress) {
                Ok(_) => {
                    report.repos_cloned += 1;
                }
                Err(e) => {
                    report
                        .errors
                        .push(format!("Failed to clone {}: {}", repo.name, e));
                }
            }
        }
        progress.inc();
    }

    progress.finish_with_message(&format!("{} Sync complete", SUCCESS_MARK));
}

fn should_sync_repo(repo_config: &RepoConfig, target: Option<&str>) -> bool {
    match target {
        Some(target_str) => repo_config.url.contains(target_str),
        None => true,
    }
}

fn analyze_repo(repo_config: &RepoConfig, base_dir: &Path) -> Result<RepoSyncInfo, SyncError> {
    let name = common::extract_repo_name(&repo_config.url);
    let local_path = common::build_local_path(repo_config, base_dir, &name);

    // Check if repo already exists
    let exists = local_path.join(".git").exists();

    Ok(RepoSyncInfo {
        url: repo_config.url.clone(),
        name,
        local_path,
        exists,
    })
}

fn write_dry_run_report(
    report: &SyncReport,
    repos: &[RepoSyncInfo],
    w: &mut impl std::io::Write,
) {
    writeln!(w, "\n=== Dry Run Mode ===").ok();
    writeln!(w, "Total repositories: {}", report.total_repos).ok();
    writeln!(w, "Repos to clone: {}", report.repos_to_clone).ok();
    writeln!(w, "Repos to fetch: {}", report.repos_to_fetch).ok();

    if report.repos_to_clone > 0 {
        writeln!(w, "\nWould clone:").ok();
        for repo in repos.iter().filter(|r| !r.exists) {
            writeln!(w, "  - {} -> {}", repo.name, repo.local_path.display()).ok();
        }
    }

    if report.repos_to_fetch > 0 {
        writeln!(w, "\nWould fetch updates:").ok();
        for repo in repos.iter().filter(|r| r.exists) {
            writeln!(w, "  - {} ({})", repo.name, repo.local_path.display()).ok();
        }
    }

    writeln!(w, "\nNo changes made. Run without --dry-run to execute.").ok();
}

fn write_sync_summary(
    report: &SyncReport,
    w: &mut impl std::io::Write,
    ew: &mut impl std::io::Write,
) {
    writeln!(w, "\n=== Sync Summary ===").ok();
    writeln!(w, "Total repositories: {}", report.total_repos).ok();
    writeln!(w, "Cloned: {}", report.repos_cloned).ok();
    writeln!(w, "Fetched: {}", report.repos_fetched).ok();

    if !report.errors.is_empty() {
        writeln!(w, "Errors: {}", report.errors.len()).ok();
        for error in &report.errors {
            writeln!(ew, "  - {}", error).ok();
        }
    }
}

/// Execute a git command with progress tracking
/// Returns Ok(()) on success, Err on failure
fn execute_git_with_progress(
    mut command: std::process::Command,
    progress: &ProgressTracker,
    repo_name: &str,
    operation: &str,
) -> Result<(), SyncError> {
    let sub_progress = progress.create_sub_progress(&format!("{} {}...", operation, repo_name));

    let result = command.output();

    match result {
        Ok(output) => {
            if output.status.success() {
                progress.finish_sub_progress(sub_progress);
                Ok(())
            } else {
                progress.finish_sub_progress(sub_progress);
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(SyncError::GitError(format!(
                    "git {} failed: {}",
                    operation.to_lowercase(),
                    stderr
                )))
            }
        }
        Err(e) => {
            progress.finish_sub_progress(sub_progress);
            Err(SyncError::GitError(format!(
                "Failed to execute git {}: {}",
                operation.to_lowercase(),
                e
            )))
        }
    }
}

fn clone_repo(repo: &RepoSyncInfo, progress: &ProgressTracker) -> Result<(), SyncError> {
    // Create parent directory if needed
    if let Some(parent) = repo.local_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut command = std::process::Command::new("git");
    command
        .arg("clone")
        .arg("--progress")
        .arg(&repo.url)
        .arg(&repo.local_path);

    execute_git_with_progress(command, progress, &repo.name, "Cloning")
}

fn fetch_repo(repo: &RepoSyncInfo, progress: &ProgressTracker) -> Result<(), SyncError> {
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(&repo.local_path)
        .arg("fetch")
        .arg("--all")
        .arg("--progress");

    execute_git_with_progress(command, progress, &repo.name, "Fetching")
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_should_sync_repo_all() {
        let repo = RepoConfig {
            url: "https://github.com/example/test.git".to_string(),
            local_dir: None,
        };

        assert!(should_sync_repo(&repo, None));
    }

    #[test]
    fn test_should_sync_repo_with_matching_target() {
        let repo = RepoConfig {
            url: "https://github.com/example/test.git".to_string(),
            local_dir: None,
        };

        assert!(should_sync_repo(&repo, Some("example")));
    }

    #[test]
    fn test_should_sync_repo_with_non_matching_target() {
        let repo = RepoConfig {
            url: "https://github.com/example/test.git".to_string(),
            local_dir: None,
        };

        assert!(!should_sync_repo(&repo, Some("other")));
    }

    #[test]
    fn test_build_initial_report() {
        let repos = vec![
            RepoSyncInfo {
                url: "https://github.com/example/test1.git".to_string(),
                name: "test1".to_string(),
                local_path: PathBuf::from("/tmp/test1"),
                exists: false,
            },
            RepoSyncInfo {
                url: "https://github.com/example/test2.git".to_string(),
                name: "test2".to_string(),
                local_path: PathBuf::from("/tmp/test2"),
                exists: true,
            },
        ];

        let report = build_initial_report(&repos);

        assert_eq!(report.total_repos, 2);
        assert_eq!(report.repos_to_clone, 1);
        assert_eq!(report.repos_to_fetch, 1);
        assert_eq!(report.repos_cloned, 0);
        assert_eq!(report.repos_fetched, 0);
    }

    #[test]
    fn test_analyze_repo_with_local_dir() {
        let repo_config = RepoConfig {
            url: "https://github.com/example/my-repo.git".to_string(),
            local_dir: Some("projects".to_string()),
        };
        let base_dir = Path::new("/home/user");

        let result = analyze_repo(&repo_config, base_dir);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.name, "my-repo");
        assert_eq!(info.url, "https://github.com/example/my-repo.git");
        assert_eq!(
            info.local_path,
            PathBuf::from("/home/user/projects/my-repo")
        );
    }

    #[test]
    fn test_analyze_repo_without_local_dir() {
        let repo_config = RepoConfig {
            url: "https://github.com/example/another-repo.git".to_string(),
            local_dir: None,
        };
        let base_dir = Path::new("/home/user");

        let result = analyze_repo(&repo_config, base_dir);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.name, "another-repo");
        assert_eq!(info.local_path, PathBuf::from("/home/user/another-repo"));
    }

    #[test]
    fn test_sync_report_default() {
        let report = SyncReport::default();

        assert_eq!(report.total_repos, 0);
        assert_eq!(report.repos_to_clone, 0);
        assert_eq!(report.repos_to_fetch, 0);
        assert_eq!(report.repos_cloned, 0);
        assert_eq!(report.repos_fetched, 0);
        assert_eq!(report.errors.len(), 0);
    }

    #[test]
    fn test_sync_report_new() {
        let report = SyncReport::new();

        assert_eq!(report.total_repos, 0);
        assert_eq!(report.repos_to_clone, 0);
        assert_eq!(report.repos_to_fetch, 0);
    }

    #[test]
    fn test_should_sync_repo_partial_url_match_middle() {
        let repo = RepoConfig {
            url: "https://github.com/myorg/awesome-tool.git".to_string(),
            local_dir: None,
        };
        assert!(should_sync_repo(&repo, Some("awesome")));
    }

    #[test]
    fn test_should_sync_repo_case_sensitive() {
        let repo = RepoConfig {
            url: "https://github.com/example/Test.git".to_string(),
            local_dir: None,
        };
        assert!(!should_sync_repo(&repo, Some("test")));
        assert!(should_sync_repo(&repo, Some("Test")));
    }

    #[test]
    fn test_should_sync_repo_empty_string_target() {
        let repo = RepoConfig {
            url: "https://github.com/example/repo.git".to_string(),
            local_dir: None,
        };
        // Empty string matches everything (every string contains "")
        assert!(should_sync_repo(&repo, Some("")));
    }

    #[test]
    fn test_build_initial_report_all_existing() {
        let repos = vec![
            RepoSyncInfo {
                url: "u1".to_string(),
                name: "r1".to_string(),
                local_path: PathBuf::from("/tmp/r1"),
                exists: true,
            },
            RepoSyncInfo {
                url: "u2".to_string(),
                name: "r2".to_string(),
                local_path: PathBuf::from("/tmp/r2"),
                exists: true,
            },
        ];
        let report = build_initial_report(&repos);
        assert_eq!(report.total_repos, 2);
        assert_eq!(report.repos_to_fetch, 2);
        assert_eq!(report.repos_to_clone, 0);
    }

    #[test]
    fn test_build_initial_report_all_new() {
        let repos = vec![
            RepoSyncInfo {
                url: "u1".to_string(),
                name: "r1".to_string(),
                local_path: PathBuf::from("/tmp/r1"),
                exists: false,
            },
            RepoSyncInfo {
                url: "u2".to_string(),
                name: "r2".to_string(),
                local_path: PathBuf::from("/tmp/r2"),
                exists: false,
            },
        ];
        let report = build_initial_report(&repos);
        assert_eq!(report.total_repos, 2);
        assert_eq!(report.repos_to_clone, 2);
        assert_eq!(report.repos_to_fetch, 0);
    }

    #[test]
    fn test_build_initial_report_empty() {
        let repos: Vec<RepoSyncInfo> = vec![];
        let report = build_initial_report(&repos);
        assert_eq!(report.total_repos, 0);
        assert_eq!(report.repos_to_clone, 0);
        assert_eq!(report.repos_to_fetch, 0);
    }

    #[test]
    fn test_build_initial_report_single_new() {
        let repos = vec![RepoSyncInfo {
            url: "u".to_string(),
            name: "r".to_string(),
            local_path: PathBuf::from("/tmp/r"),
            exists: false,
        }];
        let report = build_initial_report(&repos);
        assert_eq!(report.total_repos, 1);
        assert_eq!(report.repos_to_clone, 1);
        assert_eq!(report.repos_to_fetch, 0);
    }

    #[test]
    fn test_analyze_repo_nonexistent_path() {
        let repo_config = RepoConfig {
            url: "https://github.com/example/no-such-repo.git".to_string(),
            local_dir: None,
        };
        let info = analyze_repo(&repo_config, Path::new("/nonexistent/base")).unwrap();
        assert!(!info.exists);
        assert_eq!(info.name, "no-such-repo");
    }

    #[test]
    fn test_analyze_repo_with_fake_git_dir() {
        let temp = assert_fs::TempDir::new().unwrap();
        let repo_dir = temp.path().join("my-repo");
        std::fs::create_dir_all(repo_dir.join(".git")).unwrap();

        let repo_config = RepoConfig {
            url: "https://github.com/example/my-repo.git".to_string(),
            local_dir: None,
        };
        let info = analyze_repo(&repo_config, temp.path()).unwrap();
        assert!(info.exists);
        assert_eq!(info.name, "my-repo");
    }

    #[test]
    fn test_convert_gitlab_project_in_group_root() {
        let project = crate::providers::gitlab::GitLabProject {
            id: 1,
            name: "my-tool".to_string(),
            path: "my-tool".to_string(),
            path_with_namespace: "mygroup/my-tool".to_string(),
            ssh_url_to_repo: "git@gitlab.com:mygroup/my-tool.git".to_string(),
            http_url_to_repo: "https://gitlab.com/mygroup/my-tool.git".to_string(),
            archived: false,
        };
        let config = convert_gitlab_project_to_repo_config(&project, "mygroup", Some("projects"));
        assert_eq!(config.url, "git@gitlab.com:mygroup/my-tool.git");
        // Project is directly in group root, no subpath
        assert_eq!(config.local_dir, Some("projects".to_string()));
    }

    #[test]
    fn test_convert_gitlab_project_in_subgroup() {
        let project = crate::providers::gitlab::GitLabProject {
            id: 2,
            name: "my-tool".to_string(),
            path: "my-tool".to_string(),
            path_with_namespace: "mygroup/sub1/my-tool".to_string(),
            ssh_url_to_repo: "git@gitlab.com:mygroup/sub1/my-tool.git".to_string(),
            http_url_to_repo: "https://gitlab.com/mygroup/sub1/my-tool.git".to_string(),
            archived: false,
        };
        let config = convert_gitlab_project_to_repo_config(&project, "mygroup", Some("projects"));
        assert_eq!(config.local_dir, Some("projects/sub1".to_string()));
    }

    #[test]
    fn test_convert_gitlab_project_deep_nesting() {
        let project = crate::providers::gitlab::GitLabProject {
            id: 3,
            name: "my-tool".to_string(),
            path: "my-tool".to_string(),
            path_with_namespace: "mygroup/sub1/sub2/my-tool".to_string(),
            ssh_url_to_repo: "git@gitlab.com:mygroup/sub1/sub2/my-tool.git".to_string(),
            http_url_to_repo: "https://gitlab.com/mygroup/sub1/sub2/my-tool.git".to_string(),
            archived: false,
        };
        let config = convert_gitlab_project_to_repo_config(&project, "mygroup", Some("projects"));
        assert_eq!(config.local_dir, Some("projects/sub1/sub2".to_string()));
    }

    #[test]
    fn test_convert_gitlab_project_without_base_local_dir() {
        let project = crate::providers::gitlab::GitLabProject {
            id: 4,
            name: "my-tool".to_string(),
            path: "my-tool".to_string(),
            path_with_namespace: "mygroup/sub1/my-tool".to_string(),
            ssh_url_to_repo: "git@gitlab.com:mygroup/sub1/my-tool.git".to_string(),
            http_url_to_repo: "https://gitlab.com/mygroup/sub1/my-tool.git".to_string(),
            archived: false,
        };
        let config = convert_gitlab_project_to_repo_config(&project, "mygroup", None);
        assert!(config.local_dir.is_none());
    }

    #[test]
    fn test_sync_report_new_equals_default() {
        let new_report = SyncReport::new();
        let default_report = SyncReport::default();
        assert_eq!(new_report.total_repos, default_report.total_repos);
        assert_eq!(new_report.repos_to_clone, default_report.repos_to_clone);
        assert_eq!(new_report.repos_to_fetch, default_report.repos_to_fetch);
        assert_eq!(new_report.repos_cloned, default_report.repos_cloned);
        assert_eq!(new_report.repos_fetched, default_report.repos_fetched);
        assert_eq!(new_report.errors.len(), default_report.errors.len());
    }

    #[test]
    fn test_sync_error_common_display() {
        let inner = CommonError::ConfigNotFound("/path".to_string());
        let err = SyncError::Common(inner);
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {}", msg);
    }

    #[test]
    fn test_sync_error_io_display() {
        let err = SyncError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file missing",
        ));
        let msg = err.to_string();
        assert!(msg.contains("IO error"), "got: {}", msg);
    }

    #[test]
    fn test_sync_error_git_display() {
        let err = SyncError::GitError("clone failed".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Git operation failed"), "got: {}", msg);
        assert!(msg.contains("clone failed"), "got: {}", msg);
    }

    #[test]
    fn test_sync_error_gitlab_display() {
        let inner = crate::providers::gitlab::GitLabError::GroupNotFound("mygroup".to_string());
        let err = SyncError::GitLabError(inner);
        let msg = err.to_string();
        assert!(msg.contains("GitLab API error"), "got: {}", msg);
    }

    /// Helper: create a bare git repo in `dir` and return its path.
    fn create_bare_repo(dir: &std::path::Path, name: &str) -> PathBuf {
        let bare_path = dir.join(format!("{}.git", name));
        std::process::Command::new("git")
            .args(["init", "--bare"])
            .arg(&bare_path)
            .output()
            .expect("git init --bare failed");
        bare_path
    }

    #[test]
    fn test_clone_repo_creates_local_directory() {
        let temp = assert_fs::TempDir::new().unwrap();
        let bare = create_bare_repo(temp.path(), "test-clone");

        let repo = RepoSyncInfo {
            url: bare.display().to_string(),
            name: "test-clone".to_string(),
            local_path: temp.path().join("cloned-repo"),
            exists: false,
        };
        let progress = ProgressTracker::hidden();

        let result = clone_repo(&repo, &progress);
        assert!(result.is_ok(), "clone_repo should succeed: {:?}", result);
        assert!(
            repo.local_path.join(".git").exists(),
            "Cloned repo should have .git directory"
        );
    }

    #[test]
    fn test_fetch_repo_picks_up_new_commits() {
        let temp = assert_fs::TempDir::new().unwrap();
        let bare = create_bare_repo(temp.path(), "test-fetch");

        // Clone the bare repo first
        let clone_path = temp.path().join("fetched-repo");
        std::process::Command::new("git")
            .args(["clone"])
            .arg(&bare)
            .arg(&clone_path)
            .output()
            .expect("git clone failed");

        // Create a commit in a working copy and push to bare
        let pusher = temp.path().join("pusher");
        std::process::Command::new("git")
            .args(["clone"])
            .arg(&bare)
            .arg(&pusher)
            .output()
            .expect("git clone for pusher failed");

        // Configure git user for commit
        std::process::Command::new("git")
            .args(["-C"])
            .arg(&pusher)
            .args(["config", "user.email", "test@test.com"])
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C"])
            .arg(&pusher)
            .args(["config", "user.name", "Test"])
            .output()
            .unwrap();

        // Create a file, commit, and push
        std::fs::write(pusher.join("file.txt"), "content").unwrap();
        std::process::Command::new("git")
            .args(["-C"])
            .arg(&pusher)
            .args(["add", "file.txt"])
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C"])
            .arg(&pusher)
            .args(["commit", "-m", "add file"])
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C"])
            .arg(&pusher)
            .args(["push"])
            .output()
            .unwrap();

        // Record the remote HEAD before fetch
        let before = std::process::Command::new("git")
            .args(["-C"])
            .arg(&clone_path)
            .args(["rev-parse", "origin/master"])
            .output();

        let repo = RepoSyncInfo {
            url: bare.display().to_string(),
            name: "test-fetch".to_string(),
            local_path: clone_path.clone(),
            exists: true,
        };
        let progress = ProgressTracker::hidden();

        let result = fetch_repo(&repo, &progress);
        assert!(result.is_ok(), "fetch_repo should succeed: {:?}", result);

        // After fetch, origin/master should point to the new commit
        let after = std::process::Command::new("git")
            .args(["-C"])
            .arg(&clone_path)
            .args(["rev-parse", "origin/master"])
            .output()
            .unwrap();

        let after_sha = String::from_utf8_lossy(&after.stdout).trim().to_string();
        assert!(!after_sha.is_empty(), "Should have a valid SHA after fetch");

        // The fetched SHA should differ from the initial empty state
        if let Ok(b) = before {
            let before_sha = String::from_utf8_lossy(&b.stdout).trim().to_string();
            assert_ne!(before_sha, after_sha, "Fetch should update remote tracking ref");
        }
    }

    #[test]
    fn test_execute_sync_clone_updates_report() {
        let temp = assert_fs::TempDir::new().unwrap();
        let bare = create_bare_repo(temp.path(), "sync-clone");

        let repos = vec![RepoSyncInfo {
            url: bare.display().to_string(),
            name: "sync-clone".to_string(),
            local_path: temp.path().join("sync-cloned"),
            exists: false,
        }];

        let mut report = SyncReport::new();
        let mut progress = ProgressTracker::hidden();

        execute_sync(repos, &mut report, &mut progress);

        assert_eq!(report.repos_cloned, 1, "Should report 1 clone");
        assert_eq!(report.repos_fetched, 0, "Should report 0 fetches");
        assert!(report.errors.is_empty(), "Should have no errors");
    }

    #[test]
    fn test_execute_sync_fetch_updates_report() {
        let temp = assert_fs::TempDir::new().unwrap();
        let bare = create_bare_repo(temp.path(), "sync-fetch");

        // Clone the bare repo first so it "exists"
        let clone_path = temp.path().join("sync-fetched");
        std::process::Command::new("git")
            .args(["clone"])
            .arg(&bare)
            .arg(&clone_path)
            .output()
            .expect("git clone failed");

        let repos = vec![RepoSyncInfo {
            url: bare.display().to_string(),
            name: "sync-fetch".to_string(),
            local_path: clone_path,
            exists: true,
        }];

        let mut report = SyncReport::new();
        let mut progress = ProgressTracker::hidden();

        execute_sync(repos, &mut report, &mut progress);

        assert_eq!(report.repos_fetched, 1, "Should report 1 fetch");
        assert_eq!(report.repos_cloned, 0, "Should report 0 clones");
        assert!(report.errors.is_empty(), "Should have no errors");
    }

    #[test]
    fn test_execute_sync_mixed_clone_and_fetch() {
        let temp = assert_fs::TempDir::new().unwrap();
        let bare1 = create_bare_repo(temp.path(), "mixed-clone");
        let bare2 = create_bare_repo(temp.path(), "mixed-fetch");

        // Clone bare2 so it "exists"
        let clone_path = temp.path().join("mixed-fetched");
        std::process::Command::new("git")
            .args(["clone"])
            .arg(&bare2)
            .arg(&clone_path)
            .output()
            .expect("git clone failed");

        let repos = vec![
            RepoSyncInfo {
                url: bare1.display().to_string(),
                name: "mixed-clone".to_string(),
                local_path: temp.path().join("mixed-cloned"),
                exists: false,
            },
            RepoSyncInfo {
                url: bare2.display().to_string(),
                name: "mixed-fetch".to_string(),
                local_path: clone_path,
                exists: true,
            },
        ];

        let mut report = SyncReport::new();
        let mut progress = ProgressTracker::hidden();

        execute_sync(repos, &mut report, &mut progress);

        assert_eq!(report.repos_cloned, 1);
        assert_eq!(report.repos_fetched, 1);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_execute_git_with_progress_success() {
        let mut cmd = std::process::Command::new("git");
        cmd.arg("--version");
        let progress = ProgressTracker::hidden();

        let result = execute_git_with_progress(cmd, &progress, "test", "Version");
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_git_with_progress_failure() {
        let mut cmd = std::process::Command::new("git");
        cmd.args(["clone", "nonexistent-url-xyzzy", "/tmp/never"]);
        let progress = ProgressTracker::hidden();

        let result = execute_git_with_progress(cmd, &progress, "test", "Cloning");
        assert!(result.is_err());
    }

    #[test]
    fn test_write_dry_run_report_with_clones() {
        let report = SyncReport {
            total_repos: 2,
            repos_to_clone: 1,
            repos_to_fetch: 1,
            ..Default::default()
        };
        let repos = vec![
            RepoSyncInfo {
                url: "u1".to_string(),
                name: "new-repo".to_string(),
                local_path: PathBuf::from("/tmp/new-repo"),
                exists: false,
            },
            RepoSyncInfo {
                url: "u2".to_string(),
                name: "existing-repo".to_string(),
                local_path: PathBuf::from("/tmp/existing-repo"),
                exists: true,
            },
        ];
        let mut buf = Vec::new();
        write_dry_run_report(&report, &repos, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Dry Run Mode"), "got: {}", output);
        assert!(output.contains("Total repositories: 2"), "got: {}", output);
        assert!(output.contains("Would clone:"), "should list repos to clone, got: {}", output);
        assert!(output.contains("new-repo"), "got: {}", output);
        assert!(output.contains("Would fetch updates:"), "should list repos to fetch, got: {}", output);
        assert!(output.contains("existing-repo"), "got: {}", output);
    }

    #[test]
    fn test_write_dry_run_report_nothing_to_clone() {
        let report = SyncReport {
            total_repos: 1,
            repos_to_clone: 0,
            repos_to_fetch: 1,
            ..Default::default()
        };
        let repos = vec![RepoSyncInfo {
            url: "u".to_string(),
            name: "existing".to_string(),
            local_path: PathBuf::from("/tmp/existing"),
            exists: true,
        }];
        let mut buf = Vec::new();
        write_dry_run_report(&report, &repos, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(!output.contains("Would clone:"), "nothing to clone, got: {}", output);
        assert!(output.contains("Would fetch updates:"), "got: {}", output);
    }

    #[test]
    fn test_write_dry_run_report_nothing_to_fetch() {
        let report = SyncReport {
            total_repos: 1,
            repos_to_clone: 1,
            repos_to_fetch: 0,
            ..Default::default()
        };
        let repos = vec![RepoSyncInfo {
            url: "u".to_string(),
            name: "new".to_string(),
            local_path: PathBuf::from("/tmp/new"),
            exists: false,
        }];
        let mut buf = Vec::new();
        write_dry_run_report(&report, &repos, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Would clone:"), "got: {}", output);
        assert!(!output.contains("Would fetch updates:"), "nothing to fetch, got: {}", output);
    }

    #[test]
    fn test_write_sync_summary_no_errors() {
        let report = SyncReport {
            total_repos: 3,
            repos_cloned: 1,
            repos_fetched: 2,
            ..Default::default()
        };
        let mut buf = Vec::new();
        let mut err_buf = Vec::new();
        write_sync_summary(&report, &mut buf, &mut err_buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Sync Summary"), "got: {}", output);
        assert!(output.contains("Total repositories: 3"), "got: {}", output);
        assert!(output.contains("Cloned: 1"), "got: {}", output);
        assert!(output.contains("Fetched: 2"), "got: {}", output);
        assert!(!output.contains("Errors:"), "no errors expected, got: {}", output);
    }

    #[test]
    fn test_write_sync_summary_with_errors() {
        let report = SyncReport {
            total_repos: 2,
            repos_cloned: 0,
            repos_fetched: 0,
            errors: vec!["clone failed".to_string(), "fetch timeout".to_string()],
            ..Default::default()
        };
        let mut buf = Vec::new();
        let mut err_buf = Vec::new();
        write_sync_summary(&report, &mut buf, &mut err_buf);
        let output = String::from_utf8(buf).unwrap();
        let err_output = String::from_utf8(err_buf).unwrap();
        assert!(output.contains("Errors: 2"), "got: {}", output);
        assert!(err_output.contains("clone failed"), "got: {}", err_output);
        assert!(err_output.contains("fetch timeout"), "got: {}", err_output);
    }
}
