use assert_fs::TempDir;
use git_ranger::commands::ls::{ls_command, LsError, LsOptions};
use std::fs;
use std::path::PathBuf;

// Unit-style tests that test the ls function directly
mod ls_unit_tests {
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
    local_dir: "projects"
"#;
        fs::write(&config_path, config_content).unwrap();
        config_path
    }

    #[test]
    fn test_ls_fails_if_no_config_exists() {
        let temp_dir = TempDir::new().unwrap();
        let options = LsOptions {
            config_path: temp_dir.path().join("ranger.yaml"),
        };

        let result = ls_command(&options);

        assert!(result.is_err());
        match result {
            Err(LsError::ConfigNotFound(_)) => {
                // Expected error
            }
            _ => panic!("Expected ConfigNotFound error"),
        }
    }

    #[test]
    fn test_ls_parses_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = LsOptions {
            config_path,
        };

        let result = ls_command(&options);
        
        // Should parse successfully
        assert!(result.is_ok());
    }

    #[test]
    fn test_ls_lists_all_repos() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = LsOptions {
            config_path,
        };

        let result = ls_command(&options);
        
        assert!(result.is_ok());
        let repos = result.unwrap();
        
        // Should list all standalone repos from config
        assert_eq!(repos.len(), 2);
    }

    #[test]
    fn test_ls_includes_repo_names() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = LsOptions {
            config_path,
        };

        let result = ls_command(&options);
        
        assert!(result.is_ok());
        let repos = result.unwrap();
        
        // Each repo should have a name extracted from URL
        assert!(repos.iter().all(|r| !r.name.is_empty()));
        assert!(repos.iter().any(|r| r.name == "test-repo"));
        assert!(repos.iter().any(|r| r.name == "another-repo"));
    }

    #[test]
    fn test_ls_includes_local_paths() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = LsOptions {
            config_path,
        };

        let result = ls_command(&options);
        
        assert!(result.is_ok());
        let repos = result.unwrap();
        
        // Each repo should have a local path
        assert!(repos.iter().all(|r| !r.local_path.as_os_str().is_empty()));
    }

    #[test]
    fn test_ls_handles_missing_local_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");
        let config_content = r#"
repos:
  - url: "https://github.com/example/no-local-dir.git"
"#;
        fs::write(&config_path, config_content).unwrap();
        
        let options = LsOptions {
            config_path,
        };

        let result = ls_command(&options);
        
        // Should handle repos without local_dir specified
        assert!(result.is_ok());
        let repos = result.unwrap();
        assert_eq!(repos.len(), 1);
        
        // Should default to current dir + repo name
        let repo_path = &repos[0].local_path;
        assert!(repo_path.to_string_lossy().contains("no-local-dir"));
    }

    #[test]
    fn test_ls_extracts_repo_name_from_url() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("ranger.yaml");
        let config_content = r#"
repos:
  - url: "https://github.com/user/my-awesome-project.git"
    local_dir: "projects"
  - url: "git@gitlab.com:org/another-project.git"
    local_dir: "gitlab-stuff"
"#;
        fs::write(&config_path, config_content).unwrap();
        
        let options = LsOptions {
            config_path,
        };

        let result = ls_command(&options);
        
        assert!(result.is_ok());
        let repos = result.unwrap();
        
        // Should extract repo names correctly from different URL formats
        assert!(repos.iter().any(|r| r.name == "my-awesome-project"));
        assert!(repos.iter().any(|r| r.name == "another-project"));
    }

    #[test]
    fn test_ls_shows_full_local_paths() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(temp_dir.path());
        
        let options = LsOptions {
            config_path,
        };

        let result = ls_command(&options);
        
        assert!(result.is_ok());
        let repos = result.unwrap();
        
        // Paths should include both local_dir and repo name
        assert!(repos.iter().any(|r| {
            let path_str = r.local_path.to_string_lossy();
            path_str.contains("standalone") && path_str.contains("test-repo")
        }));
    }
}

// Integration tests that simulate full workflow
#[cfg(test)]
mod ls_integration_tests {
    use super::*;

    #[test]
    fn test_ls_end_to_end_workflow() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create a config
        let config_path = temp_dir.path().join("ranger.yaml");
        let config_content = r#"
repos:
  - url: "https://github.com/example/project-a.git"
    local_dir: "projects"
  - url: "https://github.com/example/project-b.git"
    local_dir: "projects"
  - url: "git@github.com:user/special-project.git"
    local_dir: "special"
"#;
        fs::write(&config_path, config_content).unwrap();
        
        // Run ls command
        let options = LsOptions {
            config_path,
        };
        
        let result = ls_command(&options);
        
        assert!(result.is_ok());
        let repos = result.unwrap();
        
        // Verify all repos are listed
        assert_eq!(repos.len(), 3);
        
        // Check names
        let names: Vec<String> = repos.iter().map(|r| r.name.clone()).collect();
        assert!(names.contains(&"project-a".to_string()));
        assert!(names.contains(&"project-b".to_string()));
        assert!(names.contains(&"special-project".to_string()));
    }
}
