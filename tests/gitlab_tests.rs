use git_ranger::providers::gitlab::{GitLabClient, GitLabProject, GitLabError};

// Note: These tests require a running GitLab instance or mock server
// For now, we test basic functionality and error handling

#[test]
fn test_gitlab_client_creation() {
    let client = GitLabClient::new(
        "https://gitlab.example.com".to_string(),
        "test-token".to_string(),
    );
    
    assert!(client.is_ok());
}

#[test]
fn test_gitlab_project_structure() {
    // Test that our project structure can deserialize real GitLab API responses
    let json = r#"{
        "id": 42,
        "name": "my-awesome-project",
        "path": "my-awesome-project",
        "path_with_namespace": "mygroup/subgroup/my-awesome-project",
        "ssh_url_to_repo": "git@gitlab.example.com:mygroup/subgroup/my-awesome-project.git",
        "http_url_to_repo": "https://gitlab.example.com/mygroup/subgroup/my-awesome-project.git"
    }"#;
    
    let project: Result<GitLabProject, _> = serde_json::from_str(json);
    assert!(project.is_ok());
    
    let project = project.unwrap();
    assert_eq!(project.id, 42);
    assert_eq!(project.name, "my-awesome-project");
    assert_eq!(project.path_with_namespace, "mygroup/subgroup/my-awesome-project");
    assert!(project.ssh_url_to_repo.ends_with(".git"));
}

#[test]
fn test_gitlab_projects_array_parsing() {
    // Test that we can parse an array of projects
    let json = r#"[
        {
            "id": 1,
            "name": "project-one",
            "path": "project-one",
            "path_with_namespace": "group/project-one",
            "ssh_url_to_repo": "git@gitlab.example.com:group/project-one.git",
            "http_url_to_repo": "https://gitlab.example.com/group/project-one.git"
        },
        {
            "id": 2,
            "name": "project-two",
            "path": "project-two",
            "path_with_namespace": "group/project-two",
            "ssh_url_to_repo": "git@gitlab.example.com:group/project-two.git",
            "http_url_to_repo": "https://gitlab.example.com/group/project-two.git"
        }
    ]"#;
    
    let projects: Result<Vec<GitLabProject>, _> = serde_json::from_str(json);
    assert!(projects.is_ok());
    
    let projects = projects.unwrap();
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0].name, "project-one");
    assert_eq!(projects[1].name, "project-two");
}

// Test that projects with subgroup namespaces are correctly parsed
#[test]
fn test_gitlab_projects_with_subgroups() {
    // Test parsing projects that are nested in subgroups
    let json = r#"[
        {
            "id": 1,
            "name": "repo-in-root",
            "path": "repo-in-root",
            "path_with_namespace": "mygroup/repo-in-root",
            "ssh_url_to_repo": "git@gitlab.example.com:mygroup/repo-in-root.git",
            "http_url_to_repo": "https://gitlab.example.com/mygroup/repo-in-root.git"
        },
        {
            "id": 2,
            "name": "repo-in-subgroup",
            "path": "repo-in-subgroup",
            "path_with_namespace": "mygroup/subgroup1/repo-in-subgroup",
            "ssh_url_to_repo": "git@gitlab.example.com:mygroup/subgroup1/repo-in-subgroup.git",
            "http_url_to_repo": "https://gitlab.example.com/mygroup/subgroup1/repo-in-subgroup.git"
        },
        {
            "id": 3,
            "name": "repo-in-nested",
            "path": "repo-in-nested",
            "path_with_namespace": "mygroup/subgroup1/nested/repo-in-nested",
            "ssh_url_to_repo": "git@gitlab.example.com:mygroup/subgroup1/nested/repo-in-nested.git",
            "http_url_to_repo": "https://gitlab.example.com/mygroup/subgroup1/nested/repo-in-nested.git"
        }
    ]"#;
    
    let projects: Result<Vec<GitLabProject>, _> = serde_json::from_str(json);
    assert!(projects.is_ok());
    
    let projects = projects.unwrap();
    assert_eq!(projects.len(), 3);
    
    // Verify the path_with_namespace includes the full hierarchy
    assert_eq!(projects[0].path_with_namespace, "mygroup/repo-in-root");
    assert_eq!(projects[1].path_with_namespace, "mygroup/subgroup1/repo-in-subgroup");
    assert_eq!(projects[2].path_with_namespace, "mygroup/subgroup1/nested/repo-in-nested");
}

// Test error type conversions
#[test]
fn test_gitlab_error_types() {
    let auth_error = GitLabError::AuthenticationFailed("test".to_string());
    assert!(auth_error.to_string().contains("Authentication failed"));
    
    let not_found = GitLabError::GroupNotFound("mygroup".to_string());
    assert!(not_found.to_string().contains("Group not found"));
    
    let request_failed = GitLabError::RequestFailed("network error".to_string());
    assert!(request_failed.to_string().contains("HTTP request failed"));
}

// Integration test for sync with GitLab groups (requires manual testing with real credentials)
#[test]
#[ignore] // Ignored by default, run with --ignored when you have credentials
fn test_sync_with_real_gitlab_group() {
    // This test would need:
    // 1. Real GitLab instance URL
    // 2. Valid token in environment
    // 3. Known group with repos
    
    // Example usage:
    // cargo test test_sync_with_real_gitlab_group -- --ignored
    
    // For now, this serves as documentation
    println!("To test with real GitLab:");
    println!("1. Set GITLAB_TOKEN environment variable");
    println!("2. Create a test ranger.yaml with a real group");
    println!("3. Run: git-ranger sync --dry-run");
}
