use assert_fs::TempDir;
use git_ranger::commands::sync::{sync_command, SyncError, SyncOptions};
use std::fs;
use std::path::PathBuf;

// Unit-style tests that test the sync function directly
mod sync_unit_tests {
    use super::*;

    fn create_test_config(dir: &std::path::Path) -> PathBuf {
        let config_path = dir.join("ranger.yaml");
        let config_content = r#"
providers:
  gitlab:
    host: "https://gitlab.example.com"
    token: "test-token"

groups:
  gitlab:
    - name: "test-group"
      local_dir: "test-projects"
      recursive: false

repos:
  - url: "https://github.com/example/test-repo.git"
    local_dir: "standalone"
"#;
        fs::write(&config_path, config_content).unwrap();
        config_path
    }

    #[test]
    fn test_sync_fails_if_no_config_exists() {
        let temp_dir = TempDir::new().unwrap();
        let options = SyncOptions {
            config_path: temp_dir.path().join("ranger.yaml"),
            target: None,
            dry_run: false,
        };

        let result = sync_command(&options);

        assert!(result.is_err());
        match result {
            Err(SyncError::ConfigNotFound(_)) => {
                // Expected error
            }
            _ => panic!("Expected ConfigNotFound error"),
        }
    }

    #[test]
    fn test_sync_parses_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = SyncOptions {
            config_path,
            target: None,
            dry_run: true, // Use dry-run so it doesn't try to actually clone
        };

        let result = sync_command(&options);
        
        // Should parse successfully even if no repos can be cloned
        assert!(result.is_ok());
    }

    #[test]
    fn test_sync_dry_run_reports_actions() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = SyncOptions {
            config_path,
            target: None,
            dry_run: true,
        };

        let result = sync_command(&options);
        
        assert!(result.is_ok());
        let report = result.unwrap();
        
        // Should report what would be done
        assert!(report.repos_to_clone > 0 || report.repos_to_fetch > 0);
    }

    #[test]
    fn test_sync_respects_target_filter() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = SyncOptions {
            config_path,
            target: Some("test-group".to_string()),
            dry_run: true,
        };

        let result = sync_command(&options);
        
        // Should succeed even with target filter
        assert!(result.is_ok());
    }

    #[test]
    fn test_sync_fails_on_invalid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");
        fs::write(&config_path, "invalid: yaml: content:").unwrap();
        
        let options = SyncOptions {
            config_path,
            target: None,
            dry_run: true,
        };

        let result = sync_command(&options);

        assert!(result.is_err());
        match result {
            Err(SyncError::ConfigParseError(_)) => {
                // Expected error
            }
            _ => panic!("Expected ConfigParseError"),
        }
    }

    #[test]
    fn test_sync_reports_repo_count() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = SyncOptions {
            config_path,
            target: None,
            dry_run: true,
        };

        let result = sync_command(&options);
        
        assert!(result.is_ok());
        let report = result.unwrap();
        
        // Should have at least one repo from config
        assert!(report.total_repos > 0);
    }

    #[test]
    fn test_sync_identifies_existing_repos() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create a config with a repo
        let config_content = r#"
repos:
  - url: "https://github.com/example/existing-repo.git"
    local_dir: "existing"
"#;
        let config_path = temp_dir.path().join("ranger.yaml");
        fs::write(&config_path, config_content).unwrap();
        
        // Create a fake repo directory to simulate existing repo
        let repo_dir = temp_dir.path().join("existing").join("existing-repo");
        fs::create_dir_all(&repo_dir).unwrap();
        fs::create_dir_all(repo_dir.join(".git")).unwrap();
        
        let options = SyncOptions {
            config_path,
            target: None,
            dry_run: true,
        };

        let result = sync_command(&options);
        
        assert!(result.is_ok());
        let report = result.unwrap();
        
        // Should identify existing repo
        assert!(report.repos_to_fetch > 0);
        assert_eq!(report.repos_to_clone, 0);
    }

    #[test]
    fn test_sync_identifies_missing_repos() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = SyncOptions {
            config_path,
            target: None,
            dry_run: true,
        };

        let result = sync_command(&options);
        
        assert!(result.is_ok());
        let report = result.unwrap();
        
        // Should identify repos that need cloning
        assert!(report.repos_to_clone > 0);
    }
}

// Integration tests that test through the CLI
mod sync_integration_tests {
    use super::*;
    use std::process::Command;

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

    fn create_test_config(dir: &std::path::Path) -> PathBuf {
        let config_path = dir.join("ranger.yaml");
        let config_content = r#"
providers:
  gitlab:
    host: "https://gitlab.example.com"
    token: "test-token"

groups:
  gitlab:
    - name: "test-group"
      local_dir: "test-projects"
      recursive: false

repos:
  - url: "https://github.com/example/test-repo.git"
    local_dir: "standalone"
"#;
        fs::write(&config_path, config_content).unwrap();
        config_path
    }

    #[test]
    fn test_sync_fails_without_config() {
        let temp_dir = TempDir::new().unwrap();

        let output = Command::new(get_binary_path())
            .arg("sync")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute command");

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("not found") || stderr.contains("No such file"));
    }

    #[test]
    fn test_sync_dry_run_succeeds_with_config() {
        let temp_dir = TempDir::new().unwrap();
        create_test_config(temp_dir.path());

        let output = Command::new(get_binary_path())
            .arg("sync")
            .arg("--dry-run")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Check for dry run indicators
        assert!(
            stdout.contains("Dry") || 
            stdout.contains("Would") || 
            stdout.contains("would") ||
            stdout.contains("repo")
        );
    }

    #[test]
    fn test_sync_with_target_argument() {
        let temp_dir = TempDir::new().unwrap();
        create_test_config(temp_dir.path());

        let output = Command::new(get_binary_path())
            .arg("sync")
            .arg("test-group")
            .arg("--dry-run")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());
    }

    #[test]
    fn test_sync_displays_summary() {
        let temp_dir = TempDir::new().unwrap();
        create_test_config(temp_dir.path());

        let output = Command::new(get_binary_path())
            .arg("sync")
            .arg("--dry-run")
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Should display some kind of summary
        assert!(stdout.contains("repo") || stdout.contains("clone") || stdout.contains("fetch"));
    }

    #[test]
    fn test_sync_help_displays_options() {
        let output = Command::new(get_binary_path())
            .arg("sync")
            .arg("--help")
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("dry-run") || stdout.contains("dry_run"));
    }
}
