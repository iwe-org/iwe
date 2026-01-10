use std::fs::read_to_string;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;

use liwe::model::config::{Configuration, NoteTemplate};

mod common;

fn setup_iwe_project() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    Command::new(common::get_iwe_binary_path())
        .arg("init")
        .current_dir(temp_path)
        .output()
        .expect("Failed to initialize IWE");

    temp_dir
}

fn add_template(temp_dir: &TempDir, name: &str, template: NoteTemplate) {
    let config_path = temp_dir.path().join(".iwe").join("config.toml");
    let config_content = read_to_string(&config_path).expect("Read config");
    let mut config: Configuration = toml::from_str(&config_content).expect("Parse config");

    config.templates.insert(name.to_string(), template);

    let updated_config = toml::to_string(&config).expect("Serialize config");
    std::fs::write(&config_path, updated_config).expect("Write config");
}

#[test]
fn test_new_creates_file_with_default_template() {
    let temp_dir = setup_iwe_project();
    let temp_path = temp_dir.path();

    let output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("My Test Note")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8");
    let created_path = stdout.trim();

    assert!(
        std::path::Path::new(created_path).exists(),
        "Created file should exist"
    );

    let content = read_to_string(created_path).expect("Should read file");
    assert!(
        content.contains("# My Test Note"),
        "Should have title as header"
    );
}

#[test]
fn test_new_with_slug_in_filename() {
    let temp_dir = setup_iwe_project();
    let temp_path = temp_dir.path();

    let output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Hello World Test")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8");
    assert!(
        stdout.contains("hello-world-test.md"),
        "Filename should be slugified"
    );
}

#[test]
fn test_new_with_content_argument() {
    let temp_dir = setup_iwe_project();
    let temp_path = temp_dir.path();

    let output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Note With Content")
        .arg("--content")
        .arg("This is the initial content")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8");
    let created_path = stdout.trim();
    let content = read_to_string(created_path).expect("Should read file");

    assert!(
        content.contains("# Note With Content"),
        "Should have title as header"
    );
    assert!(
        content.contains("This is the initial content"),
        "Should have content"
    );
}

#[test]
fn test_new_with_stdin_content() {
    let temp_dir = setup_iwe_project();
    let temp_path = temp_dir.path();

    let mut child = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Stdin Note")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .current_dir(temp_path)
        .spawn()
        .expect("Failed to spawn command");

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"This is piped content")
            .expect("Failed to write stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8");
    let created_path = stdout.trim();
    let content = read_to_string(created_path).expect("Should read file");

    assert!(
        content.contains("This is piped content"),
        "File should contain stdin content"
    );
}

#[test]
fn test_new_default_appends_suffix_for_existing_file() {
    let temp_dir = setup_iwe_project();
    let temp_path = temp_dir.path();

    let first_output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Duplicate Note")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(first_output.status.success());
    let first_path = String::from_utf8(first_output.stdout).expect("Valid UTF-8");
    assert!(
        first_path.contains("duplicate-note.md"),
        "First file should be duplicate-note.md"
    );

    let second_output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Duplicate Note")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(
        second_output.status.success(),
        "Second creation should succeed with suffix"
    );
    let second_path = String::from_utf8(second_output.stdout).expect("Valid UTF-8");
    assert!(
        second_path.contains("duplicate-note-1.md"),
        "Second file should be duplicate-note-1.md, got: {}",
        second_path
    );
}

#[test]
fn test_new_creates_parent_directories() {
    let temp_dir = setup_iwe_project();
    let temp_path = temp_dir.path();

    add_template(
        &temp_dir,
        "nested",
        NoteTemplate {
            key_template: "notes/{{slug}}".to_string(),
            document_template: "# {{title}}\n".to_string(),
        },
    );

    let output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Nested Note")
        .arg("--template")
        .arg("nested")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8");
    assert!(stdout.contains("notes/"), "Should create nested path");

    let created_path = stdout.trim();
    assert!(
        std::path::Path::new(created_path).exists(),
        "File should exist in nested directory"
    );
}

#[test]
fn test_new_unknown_template_fails() {
    let temp_dir = setup_iwe_project();
    let temp_path = temp_dir.path();

    let output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Test")
        .arg("--template")
        .arg("nonexistent")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(!output.status.success(), "Should fail for unknown template");
}

#[test]
fn test_new_content_argument_takes_precedence_over_stdin() {
    let temp_dir = setup_iwe_project();
    let temp_path = temp_dir.path();

    let mut child = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Priority Test")
        .arg("--content")
        .arg("Content from argument")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .current_dir(temp_path)
        .spawn()
        .expect("Failed to spawn command");

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"Content from stdin")
            .expect("Failed to write stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8");
    let created_path = stdout.trim();
    let content = read_to_string(created_path).expect("Should read file");

    assert!(
        content.contains("Content from argument"),
        "Should use content from argument"
    );
    assert!(
        !content.contains("Content from stdin"),
        "Should not use stdin content"
    );
}

#[test]
fn test_new_if_exists_suffix_appends_numbers() {
    let temp_dir = setup_iwe_project();
    let temp_path = temp_dir.path();

    let first_output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Suffix Test")
        .arg("--if-exists")
        .arg("suffix")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(first_output.status.success());
    let first_path = String::from_utf8(first_output.stdout).expect("Valid UTF-8");
    assert!(
        first_path.contains("suffix-test.md"),
        "First file should be suffix-test.md"
    );

    let second_output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Suffix Test")
        .arg("--if-exists")
        .arg("suffix")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(
        second_output.status.success(),
        "Second creation should succeed"
    );
    let second_path = String::from_utf8(second_output.stdout).expect("Valid UTF-8");
    assert!(
        second_path.contains("suffix-test-1.md"),
        "Second file should be suffix-test-1.md, got: {}",
        second_path
    );

    let third_output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Suffix Test")
        .arg("--if-exists")
        .arg("suffix")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(
        third_output.status.success(),
        "Third creation should succeed"
    );
    let third_path = String::from_utf8(third_output.stdout).expect("Valid UTF-8");
    assert!(
        third_path.contains("suffix-test-2.md"),
        "Third file should be suffix-test-2.md, got: {}",
        third_path
    );
}

#[test]
fn test_new_if_exists_override_replaces_file() {
    let temp_dir = setup_iwe_project();
    let temp_path = temp_dir.path();

    let first_output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Override Test")
        .arg("--content")
        .arg("First content")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(first_output.status.success());
    let first_path = String::from_utf8(first_output.stdout).expect("Valid UTF-8");
    let first_path = first_path.trim();
    let first_content = read_to_string(first_path).expect("Read first file");
    assert!(
        first_content.contains("First content"),
        "First file should have first content"
    );

    let second_output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Override Test")
        .arg("--content")
        .arg("Second content")
        .arg("--if-exists")
        .arg("override")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(second_output.status.success(), "Override should succeed");
    let second_path = String::from_utf8(second_output.stdout).expect("Valid UTF-8");
    assert!(
        second_path.contains("override-test.md"),
        "Should use same filename"
    );

    let overwritten_content = read_to_string(first_path).expect("Read overwritten file");
    assert!(
        overwritten_content.contains("Second content"),
        "File should have second content after override"
    );
    assert!(
        !overwritten_content.contains("First content"),
        "File should not have first content after override"
    );
}

#[test]
fn test_new_if_exists_skip_does_nothing() {
    let temp_dir = setup_iwe_project();
    let temp_path = temp_dir.path();

    let first_output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Skip Test")
        .arg("--content")
        .arg("Original content")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(first_output.status.success());
    let first_path = String::from_utf8(first_output.stdout).expect("Valid UTF-8");
    let first_path = first_path.trim();

    let second_output = Command::new(common::get_iwe_binary_path())
        .arg("new")
        .arg("Skip Test")
        .arg("--content")
        .arg("New content")
        .arg("--if-exists")
        .arg("skip")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe new");

    assert!(second_output.status.success(), "Skip should succeed");
    let second_stdout = String::from_utf8(second_output.stdout).expect("Valid UTF-8");
    assert!(
        second_stdout.trim().is_empty(),
        "Skip should print nothing, got: {}",
        second_stdout
    );

    let content = read_to_string(first_path).expect("Read file");
    assert!(
        content.contains("Original content"),
        "File should still have original content"
    );
    assert!(
        !content.contains("New content"),
        "File should not have new content"
    );
}
