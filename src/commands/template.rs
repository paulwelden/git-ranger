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

    fn with_temp_config_dir(test: impl FnOnce(&Path)) {
        let temp = assert_fs::TempDir::new().unwrap();
        let config_path = temp.path().join("config");
        env::set_var("GIT_RANGER_CONFIG_DIR", &config_path);
        test(&config_path);
        env::remove_var("GIT_RANGER_CONFIG_DIR");
    }

    #[test]
    #[serial]
    fn test_config_dir_uses_env_var() {
        with_temp_config_dir(|config_path| {
            let result = config_dir();
            assert_eq!(result, Some(config_path.to_path_buf()));
        });
    }

    #[test]
    #[serial]
    fn test_template_path_returns_full_path() {
        with_temp_config_dir(|config_path| {
            let result = template_path();
            assert_eq!(result, Some(config_path.join(TEMPLATE_FILENAME)));
        });
    }

    #[test]
    #[serial]
    fn test_load_saved_template_returns_none_when_missing() {
        with_temp_config_dir(|_| {
            let result = load_saved_template();
            assert!(result.is_none());
        });
    }

    #[test]
    #[serial]
    fn test_load_saved_template_returns_content() {
        with_temp_config_dir(|config_path| {
            std::fs::create_dir_all(config_path).unwrap();
            std::fs::write(config_path.join(TEMPLATE_FILENAME), "test: content").unwrap();

            let result = load_saved_template();
            assert_eq!(result, Some("test: content".to_string()));
        });
    }

    #[test]
    #[serial]
    fn test_save_template_fails_when_no_ranger_yaml() {
        with_temp_config_dir(|_| {
            let temp = assert_fs::TempDir::new().unwrap();
            let result = save_template(temp.path());
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                TemplateError::ConfigNotFound(_)
            ));
        });
    }

    #[test]
    #[serial]
    fn test_save_template_copies_ranger_yaml() {
        with_temp_config_dir(|config_path| {
            let temp = assert_fs::TempDir::new().unwrap();
            let source = temp.path().join("ranger.yaml");
            std::fs::write(&source, "providers:\n  gitlab:\n    host: test").unwrap();

            let result = save_template(temp.path());
            assert!(result.is_ok());

            let dest = result.unwrap();
            assert_eq!(dest, config_path.join(TEMPLATE_FILENAME));

            let saved_content = std::fs::read_to_string(&dest).unwrap();
            assert_eq!(saved_content, "providers:\n  gitlab:\n    host: test");
        });
    }

    #[test]
    #[serial]
    fn test_save_template_creates_config_dir() {
        with_temp_config_dir(|config_path| {
            assert!(!config_path.exists());

            let temp = assert_fs::TempDir::new().unwrap();
            std::fs::write(temp.path().join("ranger.yaml"), "test: data").unwrap();

            save_template(temp.path()).unwrap();
            assert!(config_path.exists());
        });
    }
}
