use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitLabError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Failed to parse response: {0}")]
    ParseError(String),
    
    #[error("Group not found: {0}")]
    GroupNotFound(String),
}

/// GitLab project information from API
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GitLabProject {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub path_with_namespace: String,
    pub ssh_url_to_repo: String,
    pub http_url_to_repo: String,
}

/// GitLab group information from API
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GitLabGroup {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub full_path: String,
}

/// GitLab API client
pub struct GitLabClient {
    base_url: String,
    token: String,
    client: reqwest::blocking::Client,
}

impl GitLabClient {
    /// Create a new GitLab client
    pub fn new(base_url: String, token: String) -> Result<Self, GitLabError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| GitLabError::RequestFailed(e.to_string()))?;
        
        Ok(Self {
            base_url,
            token,
            client,
        })
    }
    
    /// Get all projects in a group
    /// If recursive is true, includes projects from subgroups
    pub fn get_group_projects(
        &self,
        group_path: &str,
        recursive: bool,
    ) -> Result<Vec<GitLabProject>, GitLabError> {
        // URL encode the group path
        let encoded_path = urlencoding::encode(group_path);
        
        // Build URL - if recursive, use different endpoint
        let endpoint = if recursive {
            format!(
                "{}/api/v4/groups/{}/projects?include_subgroups=true&per_page=100",
                self.base_url, encoded_path
            )
        } else {
            format!(
                "{}/api/v4/groups/{}/projects?per_page=100",
                self.base_url, encoded_path
            )
        };
        
        let mut all_projects = Vec::new();
        let mut page = 1;
        
        // GitLab uses pagination
        loop {
            let url = format!("{}&page={}", endpoint, page);
            
            let response = self.client
                .get(&url)
                .header("PRIVATE-TOKEN", &self.token)
                .send()
                .map_err(|e| GitLabError::RequestFailed(e.to_string()))?;
            
            // Check for auth errors
            if response.status() == 401 || response.status() == 403 {
                return Err(GitLabError::AuthenticationFailed(
                    "Invalid or expired token".to_string()
                ));
            }
            
            // Check for not found
            if response.status() == 404 {
                return Err(GitLabError::GroupNotFound(group_path.to_string()));
            }
            
            // Check for other errors
            if !response.status().is_success() {
                return Err(GitLabError::RequestFailed(format!(
                    "HTTP {}: {}",
                    response.status(),
                    response.text().unwrap_or_default()
                )));
            }
            
            let projects: Vec<GitLabProject> = response
                .json()
                .map_err(|e| GitLabError::ParseError(e.to_string()))?;
            
            // If no more projects, we're done
            if projects.is_empty() {
                break;
            }
            
            all_projects.extend(projects);
            page += 1;
            
            // Safety limit to avoid infinite loops
            if page > 100 {
                break;
            }
        }
        
        Ok(all_projects)
    }
    
    /// Verify the token is valid by making a simple API call
    pub fn verify_token(&self) -> Result<(), GitLabError> {
        let url = format!("{}/api/v4/user", self.base_url);
        
        let response = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .map_err(|e| GitLabError::RequestFailed(e.to_string()))?;
        
        if response.status() == 401 || response.status() == 403 {
            return Err(GitLabError::AuthenticationFailed(
                "Invalid or expired token".to_string()
            ));
        }
        
        if !response.status().is_success() {
            return Err(GitLabError::RequestFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gitlab_project_deserialize() {
        let json = r#"{
            "id": 123,
            "name": "test-project",
            "path": "test-project",
            "path_with_namespace": "group/test-project",
            "ssh_url_to_repo": "git@gitlab.example.com:group/test-project.git",
            "http_url_to_repo": "https://gitlab.example.com/group/test-project.git"
        }"#;
        
        let project: GitLabProject = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, 123);
        assert_eq!(project.name, "test-project");
        assert_eq!(project.path_with_namespace, "group/test-project");
    }
    
    #[test]
    fn test_gitlab_client_creation() {
        let client = GitLabClient::new(
            "https://gitlab.example.com".to_string(),
            "test-token".to_string(),
        );
        
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_url_encoding_group_path() {
        // Test that group paths with slashes are properly encoded
        let path = "parent/child/grandchild";
        let encoded = urlencoding::encode(path);
        assert_eq!(encoded, "parent%2Fchild%2Fgrandchild");
    }
}
