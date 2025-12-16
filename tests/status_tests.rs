use assert_fs::TempDir;
use git_ranger::commands::status::{status_command, StatusError, StatusOptions};
use std::fs;
use std::path::PathBuf;

// Unit-style tests that test the status function directly
mod status_unit_tests {
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
  - url: "https://github.com/example/another-repo.git"
"#;
        fs::write(&config_path, config_content).unwrap();
        config_path
    }

    #[test]
    fn test_status_fails_if_no_config_exists() {
        let temp_dir = TempDir::new().unwrap();
        let options = StatusOptions {
            config_path: temp_dir.path().join("ranger.yaml"),
        };

        let result = status_command(&options);

        assert!(result.is_err());
        match result {
            Err(StatusError::ConfigNotFound(_)) => {
                // Expected error
            }
            _ => panic!("Expected ConfigNotFound error"),
        }
    }

    #[test]
    fn test_status_parses_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = StatusOptions {
            config_path,
        };

        let result = status_command(&options);
        
        // Should parse successfully
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_reports_repo_not_cloned() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = StatusOptions {
            config_path,
        };

        let result = status_command(&options);
        
        assert!(result.is_ok());
        let report = result.unwrap();
        
        // Should identify that repos are not yet cloned
        assert!(report.total_repos > 0);
        assert!(report.repos_not_cloned > 0);
    }

    #[test]
    fn test_status_detects_cloned_repo() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        // Create a fake git repo directory
        let standalone_dir = temp_dir.path().join("standalone").join("test-repo");
        fs::create_dir_all(&standalone_dir).unwrap();
        fs::create_dir_all(standalone_dir.join(".git")).unwrap();
        
        let options = StatusOptions {
            config_path,
        };

        let result = status_command(&options);
        
        assert!(result.is_ok());
        let report = result.unwrap();
        
        // Should detect at least one cloned repo
        assert!(report.repos_cloned >= 1);
    }

    #[test]
    fn test_status_counts_all_repos_correctly() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = StatusOptions {
            config_path,
        };

        let result = status_command(&options);
        
        assert!(result.is_ok());
        let report = result.unwrap();
        
        // Config has 2 standalone repos
        assert!(report.total_repos >= 2);
    }

    #[test]
    fn test_status_handles_missing_local_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");
        let config_content = r#"
repos:
  - url: "https://github.com/example/no-local-dir.git"
"#;
        fs::write(&config_path, config_content).unwrap();
        
        let options = StatusOptions {
            config_path,
        };

        let result = status_command(&options);
        
        // Should handle repos without local_dir specified
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_identifies_repo_name_from_url() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = StatusOptions {
            config_path,
        };

        let result = status_command(&options);
        
        assert!(result.is_ok());
        let report = result.unwrap();
        
        // Should have repo entries with proper names extracted from URLs
        assert!(!report.repos.is_empty());
        assert!(report.repos.iter().any(|r| r.name.contains("test-repo")));
    }
}

// Integration tests that simulate full workflow
#[cfg(test)]
mod status_integration_tests {
    use super::*;

    #[test]
    fn test_status_end_to_end_workflow() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create a config
        let config_path = temp_dir.path().join("ranger.yaml");
        let config_content = r#"
repos:
  - url: "https://github.com/example/project-a.git"
    local_dir: "projects"
  - url: "https://github.com/example/project-b.git"
    local_dir: "projects"
"#;
        fs::write(&config_path, config_content).unwrap();
        
        // Run status command
        let options = StatusOptions {
            config_path,
        };
        
        let result = status_command(&options);
        
        assert!(result.is_ok());
        let report = result.unwrap();
        
        // Verify the report structure
        assert_eq!(report.total_repos, 2);
        assert_eq!(report.repos.len(), 2);
    }
}
