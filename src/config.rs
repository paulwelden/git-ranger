use std::env;
use serde::{Deserialize, Serialize};
use serde::de::{self, Deserializer, Visitor};
use std::fmt;

/// A string value that can be resolved from an environment variable
/// Supports syntax: ${ENV_VAR_NAME} or direct string values
#[derive(Debug, Clone, PartialEq)]
pub struct EnvString(String);

impl EnvString {
    /// Create a new EnvString from a raw value
    #[allow(dead_code)]
    pub fn new(value: String) -> Self {
        EnvString(value)
    }

    /// Resolve the value, expanding environment variables if needed
    /// Syntax: ${VAR_NAME} - reads from environment variable
    /// Plain text is returned as-is
    pub fn resolve(&self) -> Result<String, EnvResolutionError> {
        let value = &self.0;
        
        // Check if this is an environment variable reference
        if value.starts_with("${") && value.ends_with("}") {
            let var_name = &value[2..value.len()-1];
            env::var(var_name)
                .map_err(|_| EnvResolutionError::VariableNotSet {
                    var_name: var_name.to_string(),
                })
        } else {
            // Return the literal value
            Ok(value.clone())
        }
    }

    /// Get the raw value without resolving
    #[allow(dead_code)]
    pub fn raw(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum EnvResolutionError {
    #[error("Environment variable '{var_name}' is not set")]
    VariableNotSet { var_name: String },
}

// Custom serializer for EnvString
impl serde::Serialize for EnvString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

// Custom deserializer for EnvString
impl<'de> Deserialize<'de> for EnvString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EnvStringVisitor;

        impl<'de> Visitor<'de> for EnvStringVisitor {
            type Value = EnvString;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or environment variable reference")
            }

            fn visit_str<E>(self, value: &str) -> Result<EnvString, E>
            where
                E: de::Error,
            {
                Ok(EnvString(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<EnvString, E>
            where
                E: de::Error,
            {
                Ok(EnvString(value))
            }
        }

        deserializer.deserialize_string(EnvStringVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_string() {
        let env_str = EnvString::new("my-token-123".to_string());
        assert_eq!(env_str.resolve().unwrap(), "my-token-123");
    }

    #[test]
    fn test_env_var_resolution() {
        env::set_var("TEST_TOKEN_VAR", "secret-value");
        let env_str = EnvString::new("${TEST_TOKEN_VAR}".to_string());
        assert_eq!(env_str.resolve().unwrap(), "secret-value");
        env::remove_var("TEST_TOKEN_VAR");
    }

    #[test]
    fn test_missing_env_var() {
        env::remove_var("MISSING_VAR");
        let env_str = EnvString::new("${MISSING_VAR}".to_string());
        assert!(env_str.resolve().is_err());
    }

    #[test]
    fn test_raw_value() {
        let env_str = EnvString::new("${MY_VAR}".to_string());
        assert_eq!(env_str.raw(), "${MY_VAR}");
    }

    #[test]
    fn test_deserialize_from_yaml() {
        let yaml = r#"
        token: "${GITHUB_TOKEN}"
        "#;
        
        #[derive(serde::Deserialize)]
        struct TestConfig {
            token: EnvString,
        }
        
        let config: TestConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.token.raw(), "${GITHUB_TOKEN}");
    }
}

/// Main configuration structure for ranger.yaml
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RangerConfig {
    #[serde(default)]
    pub providers: Providers,
    
    #[serde(default)]
    pub groups: Groups,
    
    #[serde(default)]
    pub repos: Vec<RepoConfig>,
}

/// Provider configurations
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct Providers {
    pub gitlab: Option<GitLabProvider>,
    pub github: Option<GitHubProvider>,
}

/// GitLab provider configuration
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GitLabProvider {
    pub host: String,
    pub token: EnvString,
}

/// GitHub provider configuration
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GitHubProvider {
    pub token: EnvString,
}

/// Group configurations by provider
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct Groups {
    #[serde(default)]
    pub gitlab: Vec<GroupConfig>,
    
    #[serde(default)]
    pub github: Vec<GroupConfig>,
}

/// Configuration for a group (GitLab) or organization (GitHub)
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GroupConfig {
    pub name: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_dir: Option<String>,
    
    #[serde(default)]
    pub recursive: bool,
}

/// Configuration for an individual repository
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RepoConfig {
    pub url: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_dir: Option<String>,
}

impl RangerConfig {
    /// Load configuration from a YAML file
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, ConfigLoadError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigLoadError::IoError(e))?;
        
        let config: RangerConfig = serde_yaml::from_str(&content)
            .map_err(|e| ConfigLoadError::ParseError(e.to_string()))?;
        
        Ok(config)
    }
    
    /// Get all repositories from the config (groups will need API calls to expand)
    pub fn get_standalone_repos(&self) -> &[RepoConfig] {
        &self.repos
    }
    
    /// Validate that required environment variables for providers are set
    #[allow(dead_code)]
    pub fn validate_providers(&self) -> Result<(), EnvResolutionError> {
        if let Some(ref gitlab) = self.providers.gitlab {
            gitlab.token.resolve()?;
        }
        
        if let Some(ref github) = self.providers.github {
            github.token.resolve()?;
        }
        
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigLoadError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Failed to parse YAML config: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod config_tests {
    use super::*;
    
    #[test]
    fn test_parse_full_config() {
        let yaml = r#"
providers:
  gitlab:
    host: "https://gitlab.example.com"
    token: "${GITLAB_TOKEN}"
  github:
    token: "${GITHUB_TOKEN}"

groups:
  gitlab:
    - name: "my-org/my-team"
      local_dir: "team-projects"
      recursive: true
  github:
    - name: "my-github-org"
      local_dir: "github-projects"

repos:
  - url: "git@github.com:example/standalone.git"
    local_dir: "standalone"
  - url: "https://gitlab.example.com/user/project.git"
"#;
        
        let config: RangerConfig = serde_yaml::from_str(yaml).unwrap();
        
        assert!(config.providers.gitlab.is_some());
        assert!(config.providers.github.is_some());
        assert_eq!(config.groups.gitlab.len(), 1);
        assert_eq!(config.groups.github.len(), 1);
        assert_eq!(config.repos.len(), 2);
    }
    
    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
repos:
  - url: "https://github.com/example/test.git"
"#;
        
        let config: RangerConfig = serde_yaml::from_str(yaml).unwrap();
        
        assert!(config.providers.gitlab.is_none());
        assert!(config.providers.github.is_none());
        assert_eq!(config.groups.gitlab.len(), 0);
        assert_eq!(config.repos.len(), 1);
    }
    
    #[test]
    fn test_group_recursive_defaults_to_false() {
        let yaml = r#"
groups:
  gitlab:
    - name: "test-group"
      local_dir: "test"
"#;
        
        let config: RangerConfig = serde_yaml::from_str(yaml).unwrap();
        
        assert_eq!(config.groups.gitlab[0].recursive, false);
    }
    
    #[test]
    fn test_optional_local_dir() {
        let yaml = r#"
repos:
  - url: "https://github.com/example/test.git"
"#;
        
        let config: RangerConfig = serde_yaml::from_str(yaml).unwrap();
        
        assert!(config.repos[0].local_dir.is_none());
    }
}
