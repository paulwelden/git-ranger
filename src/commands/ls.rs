use std::path::{Path, PathBuf};
use thiserror::Error;
use crate::config::{RangerConfig, ConfigLoadError, RepoConfig};

#[derive(Error, Debug)]
pub enum LsError {
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
pub struct LsOptions {
    pub config_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub name: String,
    pub url: String,
    pub local_path: PathBuf,
}

pub fn ls_command(options: &LsOptions) -> Result<Vec<RepoInfo>, LsError> {
    let config = load_config(&options.config_path)?;
    let base_dir = options.config_path.parent().unwrap_or_else(|| Path::new("."));
    
    let mut repos = Vec::new();
    
    // List standalone repos
    for repo_config in config.get_standalone_repos() {
        let repo_info = build_repo_info(repo_config, base_dir)?;
        repos.push(repo_info);
    }
    
    // Print listing
    print_repo_listing(&repos);
    
    Ok(repos)
}

fn load_config(config_path: &Path) -> Result<RangerConfig, LsError> {
    if !config_path.exists() {
        return Err(LsError::ConfigNotFound(config_path.display().to_string()));
    }
    
    RangerConfig::load_from_file(config_path)
        .map_err(|e| match e {
            ConfigLoadError::ParseError(msg) => LsError::ConfigParseError(msg),
            other => LsError::ConfigLoadError(other),
        })
}

fn build_repo_info(
    repo_config: &RepoConfig,
    base_dir: &Path,
) -> Result<RepoInfo, LsError> {
    let repo_name = extract_repo_name(&repo_config.url);
    let local_path = build_local_path(repo_config, base_dir, &repo_name);
    
    Ok(RepoInfo {
        name: repo_name,
        url: repo_config.url.clone(),
        local_path,
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

fn print_repo_listing(repos: &[RepoInfo]) {
    if repos.is_empty() {
        println!("No repositories configured.");
        return;
    }
    
    println!("\n=== Configured Repositories ===");
    println!();
    
    for repo in repos {
        println!("{}", repo.name);
        println!("  URL: {}", repo.url);
        println!("  Local Path: {}", repo.local_path.display());
        println!();
    }
    
    println!("Total: {} repositories", repos.len());
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
    
    #[test]
    fn test_build_repo_info_complete() {
        let repo_config = RepoConfig {
            url: "https://github.com/user/awesome-project.git".to_string(),
            local_dir: Some("projects".to_string()),
        };
        let base_dir = Path::new("/home/user/workspace");
        
        let info = build_repo_info(&repo_config, base_dir).unwrap();
        
        assert_eq!(info.name, "awesome-project");
        assert_eq!(info.url, "https://github.com/user/awesome-project.git");
        assert_eq!(info.local_path, PathBuf::from("/home/user/workspace/projects/awesome-project"));
    }
}
