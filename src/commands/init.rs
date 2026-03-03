use std::path::{Path, PathBuf};
use thiserror::Error;

use super::template;

#[derive(Error, Debug)]
pub enum InitError {
    #[error("Configuration file already exists at {0}")]
    ConfigAlreadyExists(String),

    #[error("Failed to write configuration file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Template error: {0}")]
    TemplateError(#[from] template::TemplateError),
}

#[derive(Debug, PartialEq)]
pub enum TemplateSource {
    Default,
    SavedTemplate(PathBuf),
}

const DEFAULT_CONFIG_TEMPLATE: &str = r#"# Git Ranger Configuration
# This file defines which repositories to manage and where to sync them locally.

# SECURITY: Never commit tokens directly to this file!
# Use environment variables to keep credentials secure.
# Syntax: ${ENV_VAR_NAME} - reads from environment variable
# Example: token: "${GITLAB_TOKEN}"

# Provider configurations (GitLab, GitHub, etc.)
providers:
  # GitLab configuration
  gitlab:
    host: "https://gitlab.example.com"  # Your GitLab instance URL
    token: "${GITLAB_TOKEN}"            # Set via: export GITLAB_TOKEN="your-token-here"

  # GitHub configuration (uncomment to use)
  # github:
  #   token: "${GITHUB_TOKEN}"          # Set via: export GITHUB_TOKEN="your-token-here"

# Groups to sync (provider-specific)
groups:
  # GitLab groups
  gitlab:
    - name: "my-org/my-team"            # Group path on GitLab
      local_dir: "team-projects"        # Where to clone repos locally
      recursive: true                   # Include nested subgroups

    # - name: "another-group"
    #   local_dir: "other-projects"
    #   recursive: false

  # GitHub organizations (uncomment to use)
  # github:
  #   - name: "my-github-org"
  #     local_dir: "github-projects"

# Individual repositories to sync
repos:
  # Standalone repos not part of a group
  - url: "git@github.com:example/standalone-tool.git"
    local_dir: "standalone"

  # - url: "https://gitlab.example.com/user/project.git"
  #   local_dir: "special-projects"

# Configuration notes:
# - local_dir is optional and can be relative or absolute
# - If local_dir is not specified, repos clone to the current directory
# - recursive: true will discover all nested subgroups (GitLab)
# - Run 'git-ranger sync' after editing this file to apply changes
#
# Setting up tokens:
#   Windows (PowerShell):
#     $env:GITLAB_TOKEN = "your-token-here"
#     $env:GITHUB_TOKEN = "your-token-here"
#
#   Linux/macOS (bash/zsh):
#     export GITLAB_TOKEN="your-token-here"
#     export GITHUB_TOKEN="your-token-here"
#
#   Or add to your shell profile (~/.bashrc, ~/.zshrc, or PowerShell profile)
#   to persist across sessions.
#
# IMPORTANT: Add ranger.yaml to .gitignore to prevent accidental commits!
"#;

pub fn init_command(target_dir: &Path) -> Result<(PathBuf, TemplateSource), InitError> {
    let config_path = target_dir.join("ranger.yaml");

    // Check if config already exists
    if config_path.exists() {
        return Err(InitError::ConfigAlreadyExists(
            config_path.display().to_string(),
        ));
    }

    // Try to use a saved template, fall back to default
    let (content, source) = match template::load_saved_template() {
        Some(saved) => match template::template_path() {
            Some(path) => (saved, TemplateSource::SavedTemplate(path)),
            None => (DEFAULT_CONFIG_TEMPLATE.to_string(), TemplateSource::Default),
        },
        None => (DEFAULT_CONFIG_TEMPLATE.to_string(), TemplateSource::Default),
    };

    std::fs::write(&config_path, content)?;

    Ok((config_path, source))
}

pub fn save_template_command(target_dir: &Path) -> Result<PathBuf, InitError> {
    Ok(template::save_template(target_dir)?)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid_yaml() {
        let parsed: Result<serde_yml::Value, _> = serde_yml::from_str(DEFAULT_CONFIG_TEMPLATE);
        assert!(parsed.is_ok(), "Default template must be valid YAML");
    }

    #[test]
    fn test_default_config_has_required_sections() {
        let parsed: serde_yml::Value = serde_yml::from_str(DEFAULT_CONFIG_TEMPLATE).unwrap();

        assert!(parsed.get("providers").is_some());
        assert!(parsed.get("groups").is_some());
        assert!(parsed.get("repos").is_some());
    }

    #[test]
    fn test_default_config_parseable_as_ranger_config() {
        let config: crate::config::RangerConfig =
            serde_yml::from_str(DEFAULT_CONFIG_TEMPLATE).unwrap();
        // Verify it has the expected structure, not just valid YAML
        assert!(config.providers.gitlab.is_some());
        assert!(!config.groups.gitlab.is_empty());
        assert!(!config.repos.is_empty());
    }

    #[test]
    fn test_default_template_gitlab_group_has_recursive_true() {
        let config: crate::config::RangerConfig =
            serde_yml::from_str(DEFAULT_CONFIG_TEMPLATE).unwrap();
        let gitlab_group = &config.groups.gitlab[0];
        assert!(gitlab_group.recursive, "Default template group should have recursive=true");
    }

    #[test]
    fn test_default_template_contains_env_var_placeholder() {
        assert!(
            DEFAULT_CONFIG_TEMPLATE.contains("${GITLAB_TOKEN}"),
            "Template should contain ${{GITLAB_TOKEN}} placeholder"
        );
    }

    #[test]
    fn test_template_source_equality() {
        assert_eq!(TemplateSource::Default, TemplateSource::Default);
        assert_eq!(
            TemplateSource::SavedTemplate(PathBuf::from("/a")),
            TemplateSource::SavedTemplate(PathBuf::from("/a"))
        );
        assert_ne!(TemplateSource::Default, TemplateSource::SavedTemplate(PathBuf::from("/a")));
        assert_ne!(
            TemplateSource::SavedTemplate(PathBuf::from("/a")),
            TemplateSource::SavedTemplate(PathBuf::from("/b"))
        );
    }

    #[test]
    fn test_init_error_config_already_exists_message() {
        let err = InitError::ConfigAlreadyExists("/some/path/ranger.yaml".to_string());
        let msg = err.to_string();
        assert!(msg.contains("/some/path/ranger.yaml"), "got: {}", msg);
        assert!(msg.contains("already exists"), "got: {}", msg);
    }

    #[test]
    fn test_init_error_io_error_message() {
        let err = InitError::IoError(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "permission denied",
        ));
        let msg = err.to_string();
        assert!(msg.contains("permission denied"), "got: {}", msg);
    }
}
