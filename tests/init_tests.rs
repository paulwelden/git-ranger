use assert_fs::TempDir;
use git_ranger::commands::init::{init_command, InitError};

// Unit-style tests that test the init function directly
mod init_unit_tests {
    use super::*;

    #[test]
    fn test_init_creates_ranger_yaml_in_current_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        let result = init_command(temp_dir.path());

        assert!(result.is_ok());
        assert!(config_path.exists());
    }

    #[test]
    fn test_init_creates_valid_yaml_structure() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        
        // Should contain main sections
        assert!(content.contains("providers:"));
        assert!(content.contains("groups:"));
        assert!(content.contains("repos:"));
    }

    #[test]
    fn test_init_creates_parseable_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: Result<serde_yaml::Value, _> = serde_yaml::from_str(&content);
        
        assert!(parsed.is_ok(), "Generated YAML should be valid");
    }

    #[test]
    fn test_init_fails_if_config_already_exists() {
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
    fn test_init_includes_example_gitlab_provider() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        
        assert!(content.contains("gitlab:"));
        assert!(content.contains("host:"));
        assert!(content.contains("token:"));
    }

    #[test]
    fn test_init_includes_example_group_with_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        
        assert!(content.contains("recursive:"));
        assert!(content.contains("local_dir:"));
    }

    #[test]
    fn test_init_includes_example_standalone_repo() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        
        // Should have repos section with url example
        assert!(content.contains("- url:"));
    }

    #[test]
    fn test_init_creates_file_with_comments() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        init_command(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        
        // Should include helpful comments
        assert!(content.contains("#"));
    }
}

// Integration tests that test through the CLI
mod init_integration_tests {
    use super::*;
    use std::process::Command;
    use std::path::PathBuf;

    fn get_binary_path() -> PathBuf {
        // Get the path to the compiled binary
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // Remove test executable name
        if path.ends_with("deps") {
            path.pop();
        }
        path.push("git-ranger.exe");
        path
    }

    #[test]
    fn test_init_creates_ranger_yaml_in_current_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        let output = Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(temp_dir.path())
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());
        assert!(config_path.exists());
    }

    #[test]
    fn test_init_creates_valid_yaml_structure() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(temp_dir.path())
            .output()
            .expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();
        
        // Should contain main sections
        assert!(content.contains("providers:"));
        assert!(content.contains("groups:"));
        assert!(content.contains("repos:"));
    }

    #[test]
    fn test_init_creates_parseable_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(temp_dir.path())
            .output()
            .expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: Result<serde_yaml::Value, _> = serde_yaml::from_str(&content);
        
        assert!(parsed.is_ok(), "Generated YAML should be valid");
    }

    #[test]
    fn test_init_fails_if_config_already_exists() {
        let temp_dir = TempDir::new().unwrap();

        // Create initial config
        let output1 = Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(temp_dir.path())
            .output()
            .expect("Failed to execute command");
        
        assert!(output1.status.success());
        
        // Try to init again
        let output2 = Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(temp_dir.path())
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

        Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(temp_dir.path())
            .output()
            .expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();
        
        assert!(content.contains("gitlab:"));
        assert!(content.contains("host:"));
        assert!(content.contains("token:"));
    }

    #[test]
    fn test_init_includes_example_group_with_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(temp_dir.path())
            .output()
            .expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();
        
        assert!(content.contains("recursive:"));
        assert!(content.contains("local_dir:"));
    }

    #[test]
    fn test_init_includes_example_standalone_repo() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(temp_dir.path())
            .output()
            .expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();
        
        // Should have repos section with url example
        assert!(content.contains("- url:"));
    }

    #[test]
    fn test_init_creates_file_with_comments() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");

        Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(temp_dir.path())
            .output()
            .expect("Failed to execute command");

        let content = std::fs::read_to_string(&config_path).unwrap();
        
        // Should include helpful comments
        assert!(content.contains("#"));
    }
}
