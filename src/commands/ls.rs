use crate::commands::common::{self, CommonError};
use crate::config::RepoConfig;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LsError {
    #[error("{0}")]
    Common(#[from] CommonError),

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
    let config = common::load_config(&options.config_path)?;
    let base_dir = options
        .config_path
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let mut repos = Vec::new();

    // List standalone repos
    for repo_config in config.get_standalone_repos() {
        let repo_info = build_repo_info(repo_config, base_dir);
        repos.push(repo_info);
    }

    // Print listing
    print_repo_listing(&repos);

    Ok(repos)
}

fn build_repo_info(repo_config: &RepoConfig, base_dir: &Path) -> RepoInfo {
    let repo_name = common::extract_repo_name(&repo_config.url);
    let local_path = common::build_local_path(repo_config, base_dir, &repo_name);

    RepoInfo {
        name: repo_name,
        url: repo_config.url.clone(),
        local_path,
    }
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
    fn test_build_repo_info_complete() {
        let repo_config = RepoConfig {
            url: "https://github.com/user/awesome-project.git".to_string(),
            local_dir: Some("projects".to_string()),
        };
        let base_dir = Path::new("/home/user/workspace");

        let info = build_repo_info(&repo_config, base_dir);

        assert_eq!(info.name, "awesome-project");
        assert_eq!(info.url, "https://github.com/user/awesome-project.git");
        assert_eq!(
            info.local_path,
            PathBuf::from("/home/user/workspace/projects/awesome-project")
        );
    }

    #[test]
    fn test_build_repo_info_ssh_url() {
        let repo_config = RepoConfig {
            url: "git@github.com:user/ssh-project.git".to_string(),
            local_dir: Some("repos".to_string()),
        };
        let base_dir = Path::new("/home/user");

        let info = build_repo_info(&repo_config, base_dir);

        assert_eq!(info.name, "ssh-project");
        assert_eq!(info.url, "git@github.com:user/ssh-project.git");
    }

    #[test]
    fn test_build_repo_info_without_local_dir() {
        let repo_config = RepoConfig {
            url: "https://github.com/user/fallback-repo.git".to_string(),
            local_dir: None,
        };
        let base_dir = Path::new("/home/user/workspace");

        let info = build_repo_info(&repo_config, base_dir);

        assert_eq!(info.name, "fallback-repo");
        assert_eq!(
            info.local_path,
            PathBuf::from("/home/user/workspace/fallback-repo")
        );
    }

    #[test]
    fn test_ls_error_common_display() {
        let inner = CommonError::ConfigNotFound("/some/path".to_string());
        let err = LsError::Common(inner);
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {}", msg);
    }

    #[test]
    fn test_ls_error_io_display() {
        let err = LsError::IoError(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "access denied",
        ));
        let msg = err.to_string();
        assert!(msg.contains("IO error"), "got: {}", msg);
    }
}
