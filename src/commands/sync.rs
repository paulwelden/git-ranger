use std::path::{Path, PathBuf};
use thiserror::Error;
use crate::config::{RangerConfig, ConfigLoadError, RepoConfig};
use crate::providers::gitlab::{GitLabClient, GitLabError};
use crate::progress::ProgressTracker;

// UI symbols for consistent output
const SUCCESS_MARK: &str = "✓";
const FAILURE_MARK: &str = "✗";

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Configuration file not found at {0}")]
    ConfigNotFound(String),
    
    #[error("Failed to parse configuration: {0}")]
    ConfigParseError(String),
    
    #[error("Failed to load configuration: {0}")]
    ConfigLoadError(#[from] ConfigLoadError),
    
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
    #[allow(dead_code)]
    pub repos_skipped: usize,
    pub errors: Vec<String>,
}

impl SyncReport {
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
    let config = load_config(&options.config_path)?;
    let base_dir = options.config_path.parent().unwrap_or_else(|| Path::new("."));
    
    // Initialize progress tracker
    let mut progress = if options.dry_run {
        ProgressTracker::hidden()
    } else {
        ProgressTracker::new()
    };
    
    // Show discovery spinner
    let discovery_spinner = progress.create_spinner("Discovering repositories...");
    let repos_to_sync = discover_repos(&config, base_dir, &options.target)?;
    progress.finish_spinner(discovery_spinner, &format!("{} Discovered {} repositories", SUCCESS_MARK, repos_to_sync.len()));
    
    let mut report = build_initial_report(&repos_to_sync);
    
    if options.dry_run {
        print_dry_run_report(&report, &repos_to_sync);
        return Ok(report);
    }
    
    execute_sync(repos_to_sync, &mut report, &mut progress);
    print_sync_summary(&report);
    
    Ok(report)
}

fn load_config(config_path: &Path) -> Result<RangerConfig, SyncError> {
    if !config_path.exists() {
        return Err(SyncError::ConfigNotFound(config_path.display().to_string()));
    }
    
    RangerConfig::load_from_file(config_path)
        .map_err(|e| match e {
            ConfigLoadError::ParseError(msg) => SyncError::ConfigParseError(msg),
            other => SyncError::ConfigLoadError(other),
        })
}

fn discover_repos(
    config: &RangerConfig,
    base_dir: &Path,
    target: &Option<String>,
) -> Result<Vec<RepoSyncInfo>, SyncError> {
    let mut repos = Vec::new();
    
    // Add standalone repos
    for repo_config in config.get_standalone_repos() {
        if should_sync_repo(&repo_config, target) {
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
    target: &Option<String>,
) -> Result<Option<Vec<RepoSyncInfo>>, SyncError> {
    let gitlab_provider = match &config.providers.gitlab {
        Some(provider) => provider,
        None => return Ok(None),
    };
    
    let token = match gitlab_provider.token.resolve() {
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
        if let Some(ref target_filter) = target {
            if !group_config.name.contains(target_filter) {
                continue;
            }
        }
        
        println!("Discovering active repositories in GitLab group: {}", group_config.name);
        
        match client.get_group_projects(&group_config.name, group_config.recursive) {
            Ok(projects) => {
                println!("  Found {} repositories", projects.len());
                
                for project in projects {
                    let repo_config = convert_gitlab_project_to_repo_config(
                        &project,
                        &group_config.name,
                        &group_config.local_dir,
                    );
                    repos.push(analyze_repo(&repo_config, base_dir)?);
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to get projects for group '{}': {}", 
                    group_config.name, e);
            }
        }
    }
    
    Ok(Some(repos))
}

fn convert_gitlab_project_to_repo_config(
    project: &crate::providers::gitlab::GitLabProject,
    group_name: &str,
    base_local_dir: &Option<String>,
) -> RepoConfig {
    let relative_path = if let Some(suffix) = project.path_with_namespace.strip_prefix(&format!("{}/", group_name)) {
        suffix.rsplit_once('/').map(|(parent, _)| parent.to_string())
    } else {
        None
    };
    
    let local_dir = if let Some(subpath) = relative_path {
        base_local_dir.as_ref().map(|base| format!("{}/{}", base, subpath))
    } else {
        base_local_dir.clone()
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
                    report.errors.push(format!("Failed to fetch {}: {}", repo.name, e));
                }
            }
        } else {
            match clone_repo(&repo, progress) {
                Ok(_) => {
                    report.repos_cloned += 1;
                }
                Err(e) => {
                    report.errors.push(format!("Failed to clone {}: {}", repo.name, e));
                }
            }
        }
        progress.inc();
    }
    
    progress.finish_with_message(&format!("{} Sync complete", SUCCESS_MARK));
}

fn should_sync_repo(repo_config: &RepoConfig, target: &Option<String>) -> bool {
    // If no target specified, sync all repos
    if target.is_none() {
        return true;
    }
    
    // Check if target matches the repo URL
    let target_str = target.as_ref().unwrap();
    repo_config.url.contains(target_str)
}

fn analyze_repo(repo_config: &RepoConfig, base_dir: &Path) -> Result<RepoSyncInfo, SyncError> {
    // Extract repo name from URL
    let name = extract_repo_name(&repo_config.url);
    
    // Determine local path
    let local_path = if let Some(ref local_dir) = repo_config.local_dir {
        base_dir.join(local_dir).join(&name)
    } else {
        base_dir.join(&name)
    };
    
    // Check if repo already exists
    let exists = local_path.join(".git").exists();
    
    Ok(RepoSyncInfo {
        url: repo_config.url.clone(),
        name,
        local_path,
        exists,
    })
}

fn extract_repo_name(url: &str) -> String {
    // Extract repo name from URL (last component without .git extension)
    let parts: Vec<&str> = url.split('/').collect();
    let last = parts.last().unwrap_or(&"unknown");
    
    last.trim_end_matches(".git").to_string()
}

fn print_dry_run_report(report: &SyncReport, repos: &[RepoSyncInfo]) {
    println!("\n=== Dry Run Mode ===");
    println!("Total repositories: {}", report.total_repos);
    println!("Repos to clone: {}", report.repos_to_clone);
    println!("Repos to fetch: {}", report.repos_to_fetch);
    
    if report.repos_to_clone > 0 {
        println!("\nWould clone:");
        for repo in repos.iter().filter(|r| !r.exists) {
            println!("  - {} -> {}", repo.name, repo.local_path.display());
        }
    }
    
    if report.repos_to_fetch > 0 {
        println!("\nWould fetch updates:");
        for repo in repos.iter().filter(|r| r.exists) {
            println!("  - {} ({})", repo.name, repo.local_path.display());
        }
    }
    
    println!("\nNo changes made. Run without --dry-run to execute.");
}

fn print_sync_summary(report: &SyncReport) {
    println!("\n=== Sync Summary ===");
    println!("Total repositories: {}", report.total_repos);
    println!("Cloned: {}", report.repos_cloned);
    println!("Fetched: {}", report.repos_fetched);
    
    if !report.errors.is_empty() {
        println!("Errors: {}", report.errors.len());
        for error in &report.errors {
            eprintln!("  - {}", error);
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
    success_verb: &str,
) -> Result<(), SyncError> {
    let sub_progress = progress.create_sub_progress(&format!("{} {}...", operation, repo_name));
    
    let result = command.output();
    
    match result {
        Ok(output) => {
            if output.status.success() {
                progress.finish_sub_progress(sub_progress, &format!("{} {} {}", SUCCESS_MARK, success_verb, repo_name));
                Ok(())
            } else {
                progress.finish_sub_progress(sub_progress, &format!("{} Failed to {} {}", FAILURE_MARK, operation.to_lowercase(), repo_name));
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(SyncError::GitError(format!("git {} failed: {}", operation.to_lowercase(), stderr)))
            }
        }
        Err(e) => {
            progress.finish_sub_progress(sub_progress, &format!("{} Failed to {} {}", FAILURE_MARK, operation.to_lowercase(), repo_name));
            Err(SyncError::GitError(format!("Failed to execute git {}: {}", operation.to_lowercase(), e)))
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
    
    execute_git_with_progress(command, progress, &repo.name, "Cloning", "Cloned")
}

fn fetch_repo(repo: &RepoSyncInfo, progress: &ProgressTracker) -> Result<(), SyncError> {
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(&repo.local_path)
        .arg("fetch")
        .arg("--all")
        .arg("--progress");
    
    execute_git_with_progress(command, progress, &repo.name, "Fetching", "Fetched")
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    
    #[test]
    fn test_extract_repo_name_from_https_url() {
        let url = "https://github.com/example/test-repo.git";
        assert_eq!(extract_repo_name(url), "test-repo");
    }
    
    #[test]
    fn test_extract_repo_name_from_ssh_url() {
        let url = "git@github.com:example/test-repo.git";
        assert_eq!(extract_repo_name(url), "test-repo");
    }
    
    #[test]
    fn test_extract_repo_name_without_git_extension() {
        let url = "https://github.com/example/test-repo";
        assert_eq!(extract_repo_name(url), "test-repo");
    }
    
    #[test]
    fn test_should_sync_repo_all() {
        let repo = RepoConfig {
            url: "https://github.com/example/test.git".to_string(),
            local_dir: None,
        };
        
        assert!(should_sync_repo(&repo, &None));
    }
    
    #[test]
    fn test_should_sync_repo_with_matching_target() {
        let repo = RepoConfig {
            url: "https://github.com/example/test.git".to_string(),
            local_dir: None,
        };
        
        assert!(should_sync_repo(&repo, &Some("example".to_string())));
    }
    
    #[test]
    fn test_should_sync_repo_with_non_matching_target() {
        let repo = RepoConfig {
            url: "https://github.com/example/test.git".to_string(),
            local_dir: None,
        };
        
        assert!(!should_sync_repo(&repo, &Some("other".to_string())));
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
        assert_eq!(info.local_path, PathBuf::from("/home/user/projects/my-repo"));
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
}
