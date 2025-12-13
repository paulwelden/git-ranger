use std::path::{Path, PathBuf};
use thiserror::Error;
use crate::config::{RangerConfig, ConfigLoadError, RepoConfig};
use crate::providers::gitlab::{GitLabClient, GitLabError};

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
    // Check if config exists
    if !options.config_path.exists() {
        return Err(SyncError::ConfigNotFound(
            options.config_path.display().to_string()
        ));
    }
    
    // Load and parse configuration
    let config = RangerConfig::load_from_file(&options.config_path)
        .map_err(|e| match e {
            ConfigLoadError::ParseError(msg) => SyncError::ConfigParseError(msg),
            other => SyncError::ConfigLoadError(other),
        })?;
    
    // Get the base directory (where the config file is located)
    let base_dir = options.config_path.parent()
        .unwrap_or_else(|| Path::new("."));
    
    // Collect all repos to sync
    let mut repos_to_sync = Vec::new();
    
    // Process standalone repos from config
    for repo_config in config.get_standalone_repos() {
        if should_sync_repo(&repo_config, &options.target) {
            let sync_info = analyze_repo(repo_config, base_dir)?;
            repos_to_sync.push(sync_info);
        }
    }
    
    // Expand GitLab groups by querying API
    if let Some(ref gitlab_provider) = config.providers.gitlab {
        // Resolve the token from environment variable
        let token = match gitlab_provider.token.resolve() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Warning: Failed to resolve GitLab token: {}", e);
                eprintln!("         Skipping GitLab groups");
                String::new()
            }
        };
        
        if !token.is_empty() {
            // Create GitLab client
            match GitLabClient::new(gitlab_provider.host.clone(), token) {
                Ok(client) => {
                    // Process each GitLab group
                    for group_config in &config.groups.gitlab {
                        // Check if we should sync this group based on target filter
                        if let Some(ref target) = options.target {
                            if !group_config.name.contains(target) {
                                continue;
                            }
                        }
                        
                        println!("Discovering repositories in GitLab group: {}", group_config.name);
                        
                        // Query GitLab API for projects in this group
                        match client.get_group_projects(&group_config.name, group_config.recursive) {
                            Ok(projects) => {
                                println!("  Found {} repositories", projects.len());
                                
                                // Convert each GitLab project to a repo config
                                for project in projects {
                                    let repo_config = RepoConfig {
                                        url: project.ssh_url_to_repo.clone(),
                                        local_dir: group_config.local_dir.clone(),
                                    };
                                    
                                    let sync_info = analyze_repo(&repo_config, base_dir)?;
                                    repos_to_sync.push(sync_info);
                                }
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to get projects for group '{}': {}", 
                                    group_config.name, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to create GitLab client: {}", e);
                    eprintln!("         Skipping GitLab groups");
                }
            }
        }
    }
    
    // Build the sync report
    let mut report = SyncReport::new();
    report.total_repos = repos_to_sync.len();
    
    for repo in &repos_to_sync {
        if repo.exists {
            report.repos_to_fetch += 1;
        } else {
            report.repos_to_clone += 1;
        }
    }
    
    // If dry run, just report what would happen
    if options.dry_run {
        print_dry_run_report(&report, &repos_to_sync);
        return Ok(report);
    }
    
    // Execute sync operations
    for repo in repos_to_sync {
        if repo.exists {
            // Fetch updates for existing repo
            match fetch_repo(&repo) {
                Ok(_) => {
                    report.repos_fetched += 1;
                    println!("✓ Fetched updates: {}", repo.name);
                }
                Err(e) => {
                    report.errors.push(format!("Failed to fetch {}: {}", repo.name, e));
                    eprintln!("✗ Failed to fetch {}: {}", repo.name, e);
                }
            }
        } else {
            // Clone new repo
            match clone_repo(&repo) {
                Ok(_) => {
                    report.repos_cloned += 1;
                    println!("✓ Cloned: {}", repo.name);
                }
                Err(e) => {
                    report.errors.push(format!("Failed to clone {}: {}", repo.name, e));
                    eprintln!("✗ Failed to clone {}: {}", repo.name, e);
                }
            }
        }
    }
    
    // Print summary
    print_sync_summary(&report);
    
    Ok(report)
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

fn clone_repo(repo: &RepoSyncInfo) -> Result<(), SyncError> {
    // Create parent directory if needed
    if let Some(parent) = repo.local_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Use git command to clone (this is a placeholder - in production might use git2 crate)
    let output = std::process::Command::new("git")
        .arg("clone")
        .arg(&repo.url)
        .arg(&repo.local_path)
        .output()
        .map_err(|e| SyncError::GitError(format!("Failed to execute git clone: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SyncError::GitError(format!("git clone failed: {}", stderr)));
    }
    
    Ok(())
}

fn fetch_repo(repo: &RepoSyncInfo) -> Result<(), SyncError> {
    // Use git command to fetch (this is a placeholder - in production might use git2 crate)
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&repo.local_path)
        .arg("fetch")
        .arg("--all")
        .output()
        .map_err(|e| SyncError::GitError(format!("Failed to execute git fetch: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SyncError::GitError(format!("git fetch failed: {}", stderr)));
    }
    
    Ok(())
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
}
