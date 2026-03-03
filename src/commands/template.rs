use std::path::{Path, PathBuf};
use thiserror::Error;

const APP_DIR_NAME: &str = "git-ranger";
const TEMPLATE_FILENAME: &str = "template.yaml";

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("No ranger.yaml found to save as template at {0}")]
    ConfigNotFound(String),

    #[error("Could not determine config directory")]
    NoConfigDir,

    #[error("{0}")]
    IoError(#[from] std::io::Error),
}

/// Returns the config directory for git-ranger.
/// Checks `GIT_RANGER_CONFIG_DIR` env var first (for testing), then falls back to `dirs::config_dir()`.
pub fn config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("GIT_RANGER_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    dirs::config_dir().map(|d| d.join(APP_DIR_NAME))
}

/// Returns the full path to the saved template file.
pub fn template_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join(TEMPLATE_FILENAME))
}

/// Reads the saved template if it exists. Returns `None` on any failure (silent fallback).
pub fn load_saved_template() -> Option<String> {
    let path = template_path()?;
    std::fs::read_to_string(path).ok()
}

/// Reads `ranger.yaml` from `source_dir`, creates config dir if needed, writes to template path.
pub fn save_template(source_dir: &Path) -> Result<PathBuf, TemplateError> {
    let source_path = source_dir.join("ranger.yaml");
    if !source_path.exists() {
        return Err(TemplateError::ConfigNotFound(
            source_path.display().to_string(),
        ));
    }

    let config = config_dir().ok_or(TemplateError::NoConfigDir)?;
    std::fs::create_dir_all(&config)?;

    let dest = config.join(TEMPLATE_FILENAME);
    let content = std::fs::read_to_string(&source_path)?;
    std::fs::write(&dest, content)?;

    Ok(dest)
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// Guard that sets `GIT_RANGER_CONFIG_DIR` on creation and removes it on drop,
    /// ensuring cleanup even if the test panics.
    struct ConfigDirGuard {
        _temp: assert_fs::TempDir,
        config_path: PathBuf,
    }

    impl ConfigDirGuard {
        fn new() -> Self {
            let temp = assert_fs::TempDir::new().unwrap();
            let config_path = temp.path().join("config");
            env::set_var("GIT_RANGER_CONFIG_DIR", &config_path);
            Self {
                _temp: temp,
                config_path,
            }
        }

        fn path(&self) -> &Path {
            &self.config_path
        }
    }

    impl Drop for ConfigDirGuard {
        fn drop(&mut self) {
            env::remove_var("GIT_RANGER_CONFIG_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_config_dir_uses_env_var() {
        let guard = ConfigDirGuard::new();
        let result = config_dir();
        assert_eq!(result, Some(guard.path().to_path_buf()));
    }

    #[test]
    #[serial]
    fn test_template_path_returns_full_path() {
        let guard = ConfigDirGuard::new();
        let result = template_path();
        assert_eq!(result, Some(guard.path().join(TEMPLATE_FILENAME)));
    }

    #[test]
    #[serial]
    fn test_load_saved_template_returns_none_when_missing() {
        let _guard = ConfigDirGuard::new();
        let result = load_saved_template();
        assert!(result.is_none());
    }

    #[test]
    #[serial]
    fn test_load_saved_template_returns_content() {
        let guard = ConfigDirGuard::new();
        std::fs::create_dir_all(guard.path()).unwrap();
        std::fs::write(guard.path().join(TEMPLATE_FILENAME), "test: content").unwrap();

        let result = load_saved_template();
        assert_eq!(result, Some("test: content".to_string()));
    }

    #[test]
    #[serial]
    fn test_save_template_fails_when_no_ranger_yaml() {
        let _guard = ConfigDirGuard::new();
        let temp = assert_fs::TempDir::new().unwrap();
        let result = save_template(temp.path());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TemplateError::ConfigNotFound(_)
        ));
    }

    #[test]
    #[serial]
    fn test_save_template_copies_ranger_yaml() {
        let guard = ConfigDirGuard::new();
        let temp = assert_fs::TempDir::new().unwrap();
        let source = temp.path().join("ranger.yaml");
        std::fs::write(&source, "providers:\n  gitlab:\n    host: test").unwrap();

        let result = save_template(temp.path());
        assert!(result.is_ok());

        let dest = result.unwrap();
        assert_eq!(dest, guard.path().join(TEMPLATE_FILENAME));

        let saved_content = std::fs::read_to_string(&dest).unwrap();
        assert_eq!(saved_content, "providers:\n  gitlab:\n    host: test");
    }

    #[test]
    #[serial]
    fn test_save_template_creates_config_dir() {
        let guard = ConfigDirGuard::new();
        assert!(!guard.path().exists());

        let temp = assert_fs::TempDir::new().unwrap();
        std::fs::write(temp.path().join("ranger.yaml"), "test: data").unwrap();

        save_template(temp.path()).unwrap();
        assert!(guard.path().exists());
    }

    #[test]
    #[serial]
    fn test_config_dir_fallback_ends_with_git_ranger() {
        // Unset the override env var so the fallback to dirs::config_dir() kicks in
        env::remove_var("GIT_RANGER_CONFIG_DIR");
        let result = config_dir();
        // dirs::config_dir() may return None on some CI, so just check when it's Some
        if let Some(path) = result {
            assert!(
                path.ends_with(APP_DIR_NAME),
                "Fallback config dir should end with '{}', got: {:?}",
                APP_DIR_NAME,
                path
            );
        }
    }

    #[test]
    #[serial]
    fn test_template_path_ends_with_template_yaml() {
        let _guard = ConfigDirGuard::new();
        let result = template_path().unwrap();
        assert!(
            result.ends_with(TEMPLATE_FILENAME),
            "Template path should end with '{}', got: {:?}",
            TEMPLATE_FILENAME,
            result
        );
    }

    #[test]
    fn test_template_error_config_not_found_message() {
        let err = TemplateError::ConfigNotFound("/path/to/ranger.yaml".to_string());
        let msg = err.to_string();
        assert!(msg.contains("No ranger.yaml found"), "got: {}", msg);
        assert!(msg.contains("/path/to/ranger.yaml"), "got: {}", msg);
    }

    #[test]
    fn test_template_error_no_config_dir_message() {
        let err = TemplateError::NoConfigDir;
        let msg = err.to_string();
        assert!(msg.contains("Could not determine config directory"), "got: {}", msg);
    }

    #[test]
    fn test_template_error_io_error_message() {
        let err = TemplateError::IoError(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "access denied",
        ));
        let msg = err.to_string();
        assert!(msg.contains("access denied"), "got: {}", msg);
    }
}
