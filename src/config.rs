use std::env;
use serde::de::{self, Deserialize, Deserializer, Visitor};
use std::fmt;

/// A string value that can be resolved from an environment variable
/// Supports syntax: ${ENV_VAR_NAME} or direct string values
#[derive(Debug, Clone, PartialEq)]
pub struct EnvString(String);

impl EnvString {
    /// Create a new EnvString from a raw value
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
    pub fn raw(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EnvResolutionError {
    #[error("Environment variable '{var_name}' is not set")]
    VariableNotSet { var_name: String },
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
