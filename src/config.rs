use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;

/// Wraps string values that may reference environment variables (e.g. `${GITLAB_TOKEN}`), keeping secrets out of config files.
/// Supports syntax: ${ENV_VAR_NAME} or direct string values
#[derive(Debug, Clone, PartialEq)]
pub struct EnvString(String);

impl EnvString {
    /// Create a new EnvString from a raw value
    #[allow(dead_code)] // Public library API, unused in binary crate
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
            let var_name = &value[2..value.len() - 1];
            env::var(var_name).map_err(|_| EnvResolutionError::VariableNotSet {
                var_name: var_name.to_string(),
            })
        } else {
            // Return the literal value
            Ok(value.clone())
        }
    }

    /// Get the raw value without resolving
    #[allow(dead_code)] // Public library API, unused in binary crate
    pub fn raw(&self) -> &str {
        &self.0
    }
}

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
    use serial_test::serial;

    #[test]
    fn test_literal_string() {
        let env_str = EnvString::new("my-token-123".to_string());
        assert_eq!(env_str.resolve().unwrap(), "my-token-123");
    }

    #[test]
    #[serial]
    fn test_env_var_resolution() {
        // SAFETY: test-only, serialized execution prevents concurrent env mutation
        unsafe { env::set_var("TEST_TOKEN_VAR", "secret-value") };
        let env_str = EnvString::new("${TEST_TOKEN_VAR}".to_string());
        assert_eq!(env_str.resolve().unwrap(), "secret-value");
        unsafe { env::remove_var("TEST_TOKEN_VAR") };
    }

    #[test]
    #[serial]
    fn test_missing_env_var() {
        // SAFETY: test-only, serialized execution prevents concurrent env mutation
        unsafe { env::remove_var("MISSING_VAR") };
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

        let config: TestConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.token.raw(), "${GITHUB_TOKEN}");
    }

    #[test]
    fn test_empty_env_var_name_treated_as_literal() {
        // `${}` — empty var name, should be treated as literal
        let env_str = EnvString::new("${}".to_string());
        // starts_with("${") && ends_with("}") is true, so it tries to resolve ""
        // env::var("") returns Err, so this should error
        assert!(env_str.resolve().is_err());
    }

    #[test]
    fn test_unclosed_env_var_treated_as_literal() {
        // `${` without `}` — not a valid env var reference
        let env_str = EnvString::new("${OPEN".to_string());
        assert_eq!(env_str.resolve().unwrap(), "${OPEN");
    }

    #[test]
    fn test_closing_brace_only_treated_as_literal() {
        let env_str = EnvString::new("}".to_string());
        assert_eq!(env_str.resolve().unwrap(), "}");
    }

    #[test]
    fn test_dollar_brace_only_treated_as_literal() {
        let env_str = EnvString::new("${".to_string());
        assert_eq!(env_str.resolve().unwrap(), "${");
    }

    #[test]
    fn test_new_raw_round_trip() {
        let env_str = EnvString::new("hello world".to_string());
        assert_eq!(env_str.raw(), "hello world");
    }

    #[test]
    #[serial]
    fn test_env_resolution_error_contains_var_name() {
        unsafe { env::remove_var("MY_MISSING_XYZ") };
        let env_str = EnvString::new("${MY_MISSING_XYZ}".to_string());
        let err = env_str.resolve().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("MY_MISSING_XYZ"), "Error should contain var name, got: {}", msg);
        assert!(msg.contains("not set"), "Error should mention 'not set', got: {}", msg);
    }

    #[test]
    fn test_serialize_deserialize_round_trip() {
        let original = EnvString::new("${SOME_TOKEN}".to_string());
        let serialized = serde_yml::to_string(&original).unwrap();
        let deserialized: EnvString = serde_yml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.raw(), original.raw());
        assert_eq!(deserialized, original);
    }

    #[test]
    fn test_serialize_literal_round_trip() {
        let original = EnvString::new("plain-text".to_string());
        let serialized = serde_yml::to_string(&original).unwrap();
        let deserialized: EnvString = serde_yml::from_str(&serialized).unwrap();
        assert_eq!(deserialized, original);
    }

    #[test]
    fn test_deserialize_wrong_type_produces_expecting_message() {
        // Deserializing a sequence where EnvString is expected triggers the
        // expecting() formatter, whose message must appear in the error.
        let yaml = "token: [1, 2, 3]";

        #[derive(Debug, serde::Deserialize)]
        struct TestConfig {
            token: EnvString,
        }

        let err = serde_yml::from_str::<TestConfig>(yaml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("a string or environment variable reference"),
            "Error should contain expecting message, got: {}",
            msg
        );
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
        let content = std::fs::read_to_string(path).map_err(ConfigLoadError::IoError)?;

        let config: RangerConfig = serde_yml::from_str(&content)
            .map_err(|e| ConfigLoadError::ParseError(e.to_string()))?;

        Ok(config)
    }

    /// Get all repositories from the config (groups will need API calls to expand)
    pub fn get_standalone_repos(&self) -> &[RepoConfig] {
        &self.repos
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

        let config: RangerConfig = serde_yml::from_str(yaml).unwrap();

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

        let config: RangerConfig = serde_yml::from_str(yaml).unwrap();

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

        let config: RangerConfig = serde_yml::from_str(yaml).unwrap();

        assert!(!config.groups.gitlab[0].recursive);
    }

    #[test]
    fn test_optional_local_dir() {
        let yaml = r#"
repos:
  - url: "https://github.com/example/test.git"
"#;

        let config: RangerConfig = serde_yml::from_str(yaml).unwrap();

        assert!(config.repos[0].local_dir.is_none());
    }

    #[test]
    fn test_load_from_file_nonexistent_path() {
        let result = RangerConfig::load_from_file(std::path::Path::new("/nonexistent/path/ranger.yaml"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigLoadError::IoError(_)));
    }

    #[test]
    fn test_load_from_file_invalid_yaml() {
        let temp = assert_fs::TempDir::new().unwrap();
        let path = temp.path().join("ranger.yaml");
        std::fs::write(&path, "invalid: [yaml: {broken").unwrap();
        let result = RangerConfig::load_from_file(&path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigLoadError::ParseError(_)));
    }

    #[test]
    fn test_get_standalone_repos_returns_correct_slice() {
        let yaml = r#"
repos:
  - url: "https://github.com/a/one.git"
  - url: "https://github.com/b/two.git"
  - url: "https://github.com/c/three.git"
"#;
        let config: RangerConfig = serde_yml::from_str(yaml).unwrap();
        let repos = config.get_standalone_repos();
        assert_eq!(repos.len(), 3);
        assert_eq!(repos[0].url, "https://github.com/a/one.git");
        assert_eq!(repos[1].url, "https://github.com/b/two.git");
        assert_eq!(repos[2].url, "https://github.com/c/three.git");
    }

    #[test]
    fn test_get_standalone_repos_empty() {
        let yaml = "repos: []";
        let config: RangerConfig = serde_yml::from_str(yaml).unwrap();
        assert!(config.get_standalone_repos().is_empty());
    }

    #[test]
    fn test_config_load_error_io_display() {
        let err = ConfigLoadError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        let msg = err.to_string();
        assert!(msg.contains("Failed to read config file"), "got: {}", msg);
    }

    #[test]
    fn test_config_load_error_parse_display() {
        let err = ConfigLoadError::ParseError("bad yaml".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Failed to parse YAML config"), "got: {}", msg);
        assert!(msg.contains("bad yaml"), "got: {}", msg);
    }
}
