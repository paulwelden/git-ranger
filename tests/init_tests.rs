use assert_fs::TempDir;
use git_ranger::commands::init::{init_command, InitError};
use serial_test::serial;

// Unit-style tests that test the init function directly
mod init_unit_tests {
    use super::*;

    /// Guard that sets GIT_RANGER_CONFIG_DIR on creation and removes it on drop,
    /// ensuring cleanup even if the test panics.
    struct ConfigGuard(#[allow(dead_code)] TempDir);

    impl ConfigGuard {
        fn new() -> Self {
            let dir = TempDir::new().unwrap();
            std::env::set_var("GIT_RANGER_CONFIG_DIR", dir.path());
            Self(dir)
        }
    }

    impl Drop for ConfigGuard {
        fn drop(&mut self) {
            std::env::remove_var("GIT_RANGER_CONFIG_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_init_creates_ranger_yaml_in_current_directory() {
        let _cfg = ConfigGuard::new();
        let temp_dir = TempDir::new().unwrap();

        let result = init_command(temp_dir.path());

        assert!(result.is_ok());
        let (config_path, _source) = result.unwrap();
        assert!(config_path.exists());
        assert_eq!(config_path.file_name().unwrap(), "ranger.yaml");
    }

    #[test]
    #[serial]
    fn test_init_creates_valid_yaml_structure() {
        let _cfg = ConfigGuard::new();
        let temp_dir = TempDir::new().unwrap();

        let (config_path, _source) = init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();

        // Should contain main sections
        assert!(content.contains("providers:"));
        assert!(content.contains("groups:"));
        assert!(content.contains("repos:"));
    }

    #[test]
    #[serial]
    fn test_init_creates_parseable_yaml() {
        let _cfg = ConfigGuard::new();
        let temp_dir = TempDir::new().unwrap();

        let (config_path, _source) = init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: Result<serde_yml::Value, _> = serde_yml::from_str(&content);

        assert!(parsed.is_ok(), "Generated YAML should be valid");
    }

    #[test]
    #[serial]
    fn test_init_fails_if_config_already_exists() {
        let _cfg = ConfigGuard::new();
        let temp_dir = TempDir::new().unwrap();

        // Create initial config
        init_command(temp_dir.path()).unwrap();

        // Try to init again
        let result = init_command(temp_dir.path());

        assert!(result.is_err());
        match result {
            Err(InitError::ConfigAlreadyExists(_)) => {
                // Expected error type
            }
            _ => panic!("Expected ConfigAlreadyExists error"),
        }
    }

    #[test]
    #[serial]
    fn test_init_includes_example_gitlab_provider() {
        let _cfg = ConfigGuard::new();
        let temp_dir = TempDir::new().unwrap();

        let (config_path, _source) = init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();

        assert!(content.contains("gitlab:"));
        assert!(content.contains("host:"));
        assert!(content.contains("token:"));
    }

    #[test]
    #[serial]
    fn test_init_includes_example_group_with_recursive() {
        let _cfg = ConfigGuard::new();
        let temp_dir = TempDir::new().unwrap();

        let (config_path, _source) = init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();

        assert!(content.contains("recursive:"));
        assert!(content.contains("local_dir:"));
    }

    #[test]
    #[serial]
    fn test_init_includes_example_standalone_repo() {
        let _cfg = ConfigGuard::new();
        let temp_dir = TempDir::new().unwrap();

        let (config_path, _source) = init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();

        // Should have repos section with url example
        assert!(content.contains("- url:"));
    }

    #[test]
    #[serial]
    fn test_init_creates_file_with_comments() {
        let _cfg = ConfigGuard::new();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();

        // Should include helpful comments
        assert!(content.contains("#"));
    }
}

fn get_binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    if cfg!(windows) {
        path.push("git-ranger.exe");
    } else {
        path.push("git-ranger");
    }
    path
}

// Integration tests that test through the CLI
mod init_integration_tests {
    use super::*;
    use std::process::Command;

    /// Creates an isolated config dir and returns a Command builder with it set.
    fn init_cmd(target: &std::path::Path) -> (TempDir, Command) {
        let cfg_dir = TempDir::new().unwrap();
        let mut cmd = Command::new(get_binary_path());
        cmd.arg("init")
            .arg("--dir")
            .arg(target)
            .env("GIT_RANGER_CONFIG_DIR", cfg_dir.path());
        (cfg_dir, cmd)
    }

    #[test]
    fn test_init_creates_ranger_yaml_in_current_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        let (_cfg, mut cmd) = init_cmd(temp_dir.path());
        let output = cmd.output().expect("Failed to execute command");

        assert!(output.status.success());
        assert!(config_path.exists());
    }

    #[test]
    fn test_init_creates_valid_yaml_structure() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        let (_cfg, mut cmd) = init_cmd(temp_dir.path());
        cmd.output().expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();

        assert!(content.contains("providers:"));
        assert!(content.contains("groups:"));
        assert!(content.contains("repos:"));
    }

    #[test]
    fn test_init_creates_parseable_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        let (_cfg, mut cmd) = init_cmd(temp_dir.path());
        cmd.output().expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: Result<serde_yml::Value, _> = serde_yml::from_str(&content);

        assert!(parsed.is_ok(), "Generated YAML should be valid");
    }

    #[test]
    fn test_init_fails_if_config_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let cfg_dir = TempDir::new().unwrap();

        let output1 = Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(temp_dir.path())
            .env("GIT_RANGER_CONFIG_DIR", cfg_dir.path())
            .output()
            .expect("Failed to execute command");

        assert!(output1.status.success());

        let output2 = Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(temp_dir.path())
            .env("GIT_RANGER_CONFIG_DIR", cfg_dir.path())
            .output()
            .expect("Failed to execute command");

        assert!(!output2.status.success());
        let stderr = String::from_utf8_lossy(&output2.stderr);
        assert!(stderr.contains("already exists"));
    }

    #[test]
    fn test_init_includes_example_gitlab_provider() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        let (_cfg, mut cmd) = init_cmd(temp_dir.path());
        cmd.output().expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();

        assert!(content.contains("gitlab:"));
        assert!(content.contains("host:"));
        assert!(content.contains("token:"));
    }

    #[test]
    fn test_init_includes_example_group_with_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        let (_cfg, mut cmd) = init_cmd(temp_dir.path());
        cmd.output().expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();

        assert!(content.contains("recursive:"));
        assert!(content.contains("local_dir:"));
    }

    #[test]
    fn test_init_includes_example_standalone_repo() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        let (_cfg, mut cmd) = init_cmd(temp_dir.path());
        cmd.output().expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();

        assert!(content.contains("- url:"));
    }

    #[test]
    fn test_init_creates_file_with_comments() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        let (_cfg, mut cmd) = init_cmd(temp_dir.path());
        cmd.output().expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();

        assert!(content.contains("#"));
    }
}
