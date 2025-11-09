use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, read_to_string, File};
use std::process::Command;
use tempfile::TempDir;

mod common;

#[test]
fn test_init_creates_iwe_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let output = run_init_command(&temp_path);

    assert!(output.status.success(), "Init command should succeed");

    let iwe_dir = temp_path.join(".iwe");
    assert!(iwe_dir.exists(), ".iwe directory should be created");
    assert!(iwe_dir.is_dir(), ".iwe should be a directory");
}

#[test]
fn test_init_creates_config_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let output = run_init_command(&temp_path);

    assert!(output.status.success(), "Init command should succeed");

    let config_file = temp_path.join(".iwe").join("config.toml");
    assert!(config_file.exists(), "config.toml should be created");
    assert!(config_file.is_file(), "config.toml should be a file");

    let config_content = read_to_string(&config_file).expect("Should be able to read config file");
    let parsed_config: toml::Value =
        toml::from_str(&config_content).expect("config.toml should contain valid TOML");

    assert!(
        parsed_config.get("library").is_some(),
        "Config should have library section"
    );
    assert!(
        parsed_config.get("markdown").is_some(),
        "Config should have markdown section"
    );
}

#[test]
fn test_init_already_initialized() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let output1 = run_init_command(&temp_path);
    assert!(output1.status.success(), "First init should succeed");

    let output2 = Command::new(common::get_iwe_binary_path())
        .arg("init")
        .arg("-v")
        .arg("2")
        .current_dir(&temp_path)
        .output()
        .expect("Failed to execute iwe init");

    assert!(output2.status.success(), "Command should not crash");

    let stderr = String::from_utf8(output2.stderr).expect("Valid UTF-8 stderr");
    assert!(
        stderr.contains("already initialized") || stderr.is_empty(),
        "Should indicate already initialized"
    );
}

#[test]
fn test_init_existing_iwe_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let iwe_file = temp_path.join(".iwe");
    File::create(&iwe_file).expect("Should create .iwe file");

    let output = Command::new(common::get_iwe_binary_path())
        .arg("init")
        .arg("-v")
        .arg("2")
        .current_dir(&temp_path)
        .output()
        .expect("Failed to execute iwe init");

    assert!(output.status.success(), "Command should not crash");

    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 stderr");
    assert!(
        stderr.contains("already exists") || stderr.contains("failed"),
        "Should indicate failure due to existing file"
    );
}

#[test]
fn test_init_config_file_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let output = run_init_command(&temp_path);
    assert!(output.status.success(), "Init command should succeed");

    let config_file = temp_path.join(".iwe").join("config.toml");
    let config_content = read_to_string(&config_file).expect("Should be able to read config file");

    assert!(
        config_content.contains("[library]"),
        "Should have library section"
    );
    assert!(
        config_content.contains("[markdown]"),
        "Should have markdown section"
    );
    assert!(
        config_content.contains("path"),
        "Should have path configuration"
    );
}

#[test]
fn test_init_nested_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let nested_path = temp_path.join("nested").join("directory");
    create_dir_all(&nested_path).expect("Should create nested directory");

    let output = Command::new(common::get_iwe_binary_path())
        .arg("init")
        .current_dir(&nested_path)
        .output()
        .expect("Failed to execute iwe init");

    assert!(
        output.status.success(),
        "Init should work in nested directory"
    );

    let iwe_dir = nested_path.join(".iwe");
    assert!(
        iwe_dir.exists(),
        ".iwe directory should be created in nested location"
    );
    assert!(
        iwe_dir.join("config.toml").exists(),
        "config.toml should be created"
    );
}

#[test]
fn test_init_with_verbose_flag() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let output = Command::new(common::get_iwe_binary_path())
        .arg("init")
        .arg("--verbose")
        .arg("1")
        .current_dir(&temp_path)
        .output()
        .expect("Failed to execute iwe init");

    assert!(
        output.status.success(),
        "Init with verbose flag should succeed"
    );

    let iwe_dir = temp_path.join(".iwe");
    assert!(iwe_dir.exists(), ".iwe directory should be created");
    assert!(
        iwe_dir.join("config.toml").exists(),
        "config.toml should be created"
    );
}

#[test]
fn test_init_preserves_existing_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let iwe_dir = temp_path.join(".iwe");
    create_dir_all(&iwe_dir).expect("Should create .iwe directory");

    let config_file = iwe_dir.join("config.toml");
    let config = Configuration {
        library: LibraryOptions {
            path: "custom_path".to_string(),
            ..Default::default()
        },
        markdown: MarkdownOptions {
            refs_extension: "".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let custom_config = toml::to_string(&config).expect("Failed to serialize config to TOML");
    std::fs::write(&config_file, custom_config).expect("Should write custom config");

    let output = run_init_command(&temp_path);
    assert!(output.status.success(), "Command should not crash");

    let final_config = read_to_string(&config_file).expect("Should read config");
    assert!(
        final_config.contains("custom_path"),
        "Custom config should be preserved"
    );
}

fn run_init_command(work_dir: &std::path::Path) -> std::process::Output {
    Command::new(common::get_iwe_binary_path())
        .arg("init")
        .current_dir(work_dir)
        .output()
        .expect("Failed to execute iwe init")
}
