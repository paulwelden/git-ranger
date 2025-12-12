use git_ranger::config::EnvString;
use serial_test::serial;
use std::env;

#[test]
#[serial]
fn test_env_string_resolves_environment_variables() {
    // Set up test environment variable
    unsafe { env::set_var("TEST_CONFIG_TOKEN", "secret-test-token-12345"); }
    
    let env_str = EnvString::new("${TEST_CONFIG_TOKEN}".to_string());
    let resolved = env_str.resolve().expect("Should resolve successfully");
    
    assert_eq!(resolved, "secret-test-token-12345");
    
    // Clean up
    unsafe { env::remove_var("TEST_CONFIG_TOKEN"); }
}

#[test]
fn test_env_string_returns_literal_when_not_env_var() {
    let env_str = EnvString::new("literal-value".to_string());
    let resolved = env_str.resolve().expect("Should resolve successfully");
    
    assert_eq!(resolved, "literal-value");
}

#[test]
#[serial]
fn test_env_string_error_when_var_not_set() {
    // Ensure variable doesn't exist
    unsafe { env::remove_var("NONEXISTENT_VAR_12345"); }
    
    let env_str = EnvString::new("${NONEXISTENT_VAR_12345}".to_string());
    let result = env_str.resolve();
    
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("NONEXISTENT_VAR_12345"));
}

#[test]
fn test_config_with_env_vars_parses_correctly() {
    use serde::Deserialize;
    
    #[derive(Deserialize)]
    struct TestProvider {
        token: EnvString,
    }
    
    #[derive(Deserialize)]
    struct TestProviders {
        gitlab: TestProvider,
    }
    
    #[derive(Deserialize)]
    struct TestConfig {
        providers: TestProviders,
    }
    
    let yaml = r#"
providers:
  gitlab:
    token: "${GITLAB_TOKEN}"
"#;
    
    let config: TestConfig = serde_yaml::from_str(yaml).expect("Should parse YAML");
    assert_eq!(config.providers.gitlab.token.raw(), "${GITLAB_TOKEN}");
}

#[test]
#[serial]
fn test_full_config_with_both_literal_and_env_tokens() {
    use serde::Deserialize;
    
    #[derive(Deserialize)]
    struct Provider {
        #[serde(default)]
        #[allow(dead_code)]
        host: Option<String>,
        token: EnvString,
    }
    
    #[derive(Deserialize)]
    struct Providers {
        gitlab: Provider,
        github: Option<Provider>,
    }
    
    #[derive(Deserialize)]
    struct Config {
        providers: Providers,
    }
    
    let yaml = r#"
providers:
  gitlab:
    host: "https://gitlab.example.com"
    token: "${GITLAB_TOKEN}"
  github:
    token: "literal-token-for-testing"
"#;
    
    // Set up environment
    unsafe { env::set_var("GITLAB_TOKEN", "gitlab-secret-token"); }
    
    let config: Config = serde_yaml::from_str(yaml).expect("Should parse YAML");
    
    // Verify GitLab token resolves from env
    assert_eq!(
        config.providers.gitlab.token.resolve().unwrap(),
        "gitlab-secret-token"
    );
    
    // Verify GitHub token is literal
    assert_eq!(
        config.providers.github.unwrap().token.resolve().unwrap(),
        "literal-token-for-testing"
    );
    
    // Clean up
    unsafe { env::remove_var("GITLAB_TOKEN"); }
}
