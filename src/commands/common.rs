use crate::config::{ConfigLoadError, RangerConfig, RepoConfig};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommonError {
    #[error("Configuration file not found at {0}")]
    ConfigNotFound(String),

    #[error("Failed to parse configuration: {0}")]
    ConfigParseError(String),

    #[error("Failed to load configuration: {0}")]
    ConfigLoadError(ConfigLoadError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub fn load_config(config_path: &Path) -> Result<RangerConfig, CommonError> {
    if !config_path.exists() {
        return Err(CommonError::ConfigNotFound(
            config_path.display().to_string(),
        ));
    }

    RangerConfig::load_from_file(config_path).map_err(|e| match e {
        ConfigLoadError::ParseError(msg) => CommonError::ConfigParseError(msg),
        e @ ConfigLoadError::IoError(_) => CommonError::ConfigLoadError(e),
    })
}

/// Extract repository name from a URL.
///
/// Handles HTTPS and SSH-style URLs:
/// - `https://github.com/user/repo.git` -> `repo`
/// - `git@github.com:user/repo.git` -> `repo`
/// - `https://gitlab.com/org/project` -> `project`
pub fn extract_repo_name(url: &str) -> String {
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

/// Determine the local filesystem path for a repository.
///
/// If `local_dir` is set and absolute, uses it directly.
/// If `local_dir` is set and relative, joins it with `base_dir`.
/// If `local_dir` is not set, uses `base_dir` directly.
/// The repo name is always appended as the final path component.
pub fn build_local_path(repo_config: &RepoConfig, base_dir: &Path, repo_name: &str) -> PathBuf {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name_from_https_url() {
        assert_eq!(
            extract_repo_name("https://github.com/user/my-repo.git"),
            "my-repo"
        );
        assert_eq!(
            extract_repo_name("https://gitlab.com/org/project.git"),
            "project"
        );
    }

    #[test]
    fn test_extract_repo_name_from_ssh_url() {
        assert_eq!(
            extract_repo_name("git@github.com:user/my-repo.git"),
            "my-repo"
        );
        assert_eq!(
            extract_repo_name("git@gitlab.com:org/project.git"),
            "project"
        );
    }

    #[test]
    fn test_extract_repo_name_without_git_extension() {
        assert_eq!(
            extract_repo_name("https://github.com/user/my-repo"),
            "my-repo"
        );
    }

    #[test]
    fn test_extract_repo_name_with_trailing_slash() {
        assert_eq!(
            extract_repo_name("https://github.com/user/my-repo.git/"),
            "my-repo"
        );
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
