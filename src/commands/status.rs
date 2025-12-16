use std::path::{Path, PathBuf};
use thiserror::Error;
use crate::config::{RangerConfig, ConfigLoadError, RepoConfig};

#[derive(Error, Debug)]
pub enum StatusError {
    #[error("Configuration file not found at {0}")]
    ConfigNotFound(String),
    
    #[error("Failed to parse configuration: {0}")]
    ConfigParseError(String),
    
    #[error("Failed to load configuration: {0}")]
    ConfigLoadError(#[from] ConfigLoadError),
    
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
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn status_command(options: &StatusOptions) -> Result<StatusReport, StatusError> {
    let config = load_config(&options.config_path)?;
    let base_dir = options.config_path.parent().unwrap_or_else(|| Path::new("."));
    
    let mut report = StatusReport::new();
    
    // Analyze standalone repos
    for repo_config in config.get_standalone_repos() {
        let repo_status = analyze_repo_status(repo_config, base_dir)?;
        
        if repo_status.cloned {
            report.repos_cloned += 1;
        } else {
            report.repos_not_cloned += 1;
        }
        
        report.repos.push(repo_status);
        report.total_repos += 1;
    }
    
    // Print status report
    print_status_report(&report);
    
    Ok(report)
}

fn load_config(config_path: &Path) -> Result<RangerConfig, StatusError> {
    if !config_path.exists() {
        return Err(StatusError::ConfigNotFound(config_path.display().to_string()));
    }
    
    RangerConfig::load_from_file(config_path)
        .map_err(|e| match e {
            ConfigLoadError::ParseError(msg) => StatusError::ConfigParseError(msg),
            other => StatusError::ConfigLoadError(other),
        })
}

fn analyze_repo_status(
    repo_config: &RepoConfig,
    base_dir: &Path,
) -> Result<RepoStatus, StatusError> {
    let repo_name = extract_repo_name(&repo_config.url);
    let local_path = build_local_path(repo_config, base_dir, &repo_name);
    
    // Check if repo is cloned (has .git directory)
    let git_dir = local_path.join(".git");
    let cloned = git_dir.exists();
    
    Ok(RepoStatus {
        name: repo_name,
        local_path,
        cloned,
    })
}

fn extract_repo_name(url: &str) -> String {
    // Extract repo name from URL
    // Examples:
    // - https://github.com/user/repo.git -> repo
    // - git@github.com:user/repo.git -> repo
    // - https://gitlab.com/org/project -> project
    
    let url = url.trim_end_matches('/');
    let url = url.trim_end_matches(".git");
    
    url.rsplit('/')
        .next()
        .unwrap_or("unknown")
        .rsplit(':')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

fn build_local_path(
    repo_config: &RepoConfig,
    base_dir: &Path,
    repo_name: &str,
) -> PathBuf {
    let local_dir = match &repo_config.local_dir {
        Some(dir) => {
            let dir_path = PathBuf::from(dir);
            if dir_path.is_absolute() {
                dir_path
            } else {
                base_dir.join(dir)
            }
        }
        None => base_dir.to_path_buf(),
    };
    
    local_dir.join(repo_name)
}

fn print_status_report(report: &StatusReport) {
    println!("\n=== Repository Status ===");
    println!("Total repositories: {}", report.total_repos);
    println!("Cloned: {}", report.repos_cloned);
    println!("Not cloned: {}", report.repos_not_cloned);
    println!();
    
    if report.repos.is_empty() {
        println!("No repositories configured.");
        return;
    }
    
    for repo in &report.repos {
        let status_icon = if repo.cloned { "✓" } else { "✗" };
        let status_text = if repo.cloned { "cloned" } else { "not cloned" };
        
        println!("{} {} - {} ({})",
            status_icon,
            repo.name,
            status_text,
            repo.local_path.display()
        );
    }
    
    println!();
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    
    #[test]
    fn test_extract_repo_name_from_https_url() {
        assert_eq!(extract_repo_name("https://github.com/user/my-repo.git"), "my-repo");
        assert_eq!(extract_repo_name("https://gitlab.com/org/project.git"), "project");
    }
    
    #[test]
    fn test_extract_repo_name_from_ssh_url() {
        assert_eq!(extract_repo_name("git@github.com:user/my-repo.git"), "my-repo");
        assert_eq!(extract_repo_name("git@gitlab.com:org/project.git"), "project");
    }
    
    #[test]
    fn test_extract_repo_name_without_git_extension() {
        assert_eq!(extract_repo_name("https://github.com/user/my-repo"), "my-repo");
    }
    
    #[test]
    fn test_extract_repo_name_with_trailing_slash() {
        assert_eq!(extract_repo_name("https://github.com/user/my-repo.git/"), "my-repo");
    }
    
    #[test]
    fn test_build_local_path_with_relative_dir() {
        let repo_config = RepoConfig {
            url: "https://github.com/user/repo.git".to_string(),
            local_dir: Some("projects".to_string()),
        };
        let base_dir = Path::new("/home/user/workspace");
        let repo_name = "repo";
        
        let path = build_local_path(&repo_config, base_dir, repo_name);
        
        assert_eq!(path, PathBuf::from("/home/user/workspace/projects/repo"));
    }
    
    #[test]
    fn test_build_local_path_without_local_dir() {
        let repo_config = RepoConfig {
            url: "https://github.com/user/repo.git".to_string(),
            local_dir: None,
        };
        let base_dir = Path::new("/home/user/workspace");
        let repo_name = "repo";
        
        let path = build_local_path(&repo_config, base_dir, repo_name);
        
        assert_eq!(path, PathBuf::from("/home/user/workspace/repo"));
    }
}
