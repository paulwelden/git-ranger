use assert_fs::TempDir;
use std::process::Command;

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

mod template_integration_tests {
    use super::*;

    #[test]
    fn test_save_template_fails_when_no_ranger_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();

        let output = Command::new(get_binary_path())
            .arg("init")
            .arg("--save-template")
            .arg("--dir")
            .arg(temp_dir.path())
            .env("GIT_RANGER_CONFIG_DIR", config_dir.path())
            .output()
            .expect("Failed to execute command");

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("No ranger.yaml found to save as template"));
    }

    #[test]
    fn test_save_template_succeeds_and_prints_confirmation() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();

        // Create a ranger.yaml first
        std::fs::write(
            temp_dir.path().join("ranger.yaml"),
            "providers:\n  gitlab:\n    host: test",
        )
        .unwrap();

        let output = Command::new(get_binary_path())
            .arg("init")
            .arg("--save-template")
            .arg("--dir")
            .arg(temp_dir.path())
            .env("GIT_RANGER_CONFIG_DIR", config_dir.path())
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Saved template to"));
        assert!(stdout.contains("Future `git-ranger init` will use this template"));
    }

    #[test]
    fn test_init_uses_saved_template_content() {
        let config_dir = TempDir::new().unwrap();
        let template_content = "providers:\n  custom:\n    host: saved-template-test";

        // Save template manually
        std::fs::write(config_dir.path().join("template.yaml"), template_content).unwrap();

        // Init in a fresh directory
        let target_dir = TempDir::new().unwrap();
        let output = Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(target_dir.path())
            .env("GIT_RANGER_CONFIG_DIR", config_dir.path())
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Using saved template from"));

        let content = std::fs::read_to_string(target_dir.path().join("ranger.yaml")).unwrap();
        assert_eq!(content, template_content);
    }

    #[test]
    fn test_init_without_saved_template_does_not_mention_templates() {
        let config_dir = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        let output = Command::new(get_binary_path())
            .arg("init")
            .arg("--dir")
            .arg(target_dir.path())
            .env("GIT_RANGER_CONFIG_DIR", config_dir.path())
            .output()
            .expect("Failed to execute command");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.contains("template"));
    }

    #[test]
    fn test_help_shows_save_template_flag() {
        let output = Command::new(get_binary_path())
            .arg("init")
            .arg("--help")
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("--save-template"));
    }
}
