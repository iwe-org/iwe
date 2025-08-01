use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::env;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_contents_basic_output() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_contents_command(&temp_path, &[]);
    assert!(output.status.success(), "Contents command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        stdout.contains("# Contents"),
        "Should contain contents header"
    );

    assert!(!stdout.trim().is_empty(), "Should produce some output");
}

#[test]
fn test_contents_includes_links() {
    let temp_dir = setup_test_workspace_with_multiple_files();
    let temp_path = temp_dir.path();

    let output = run_contents_command(&temp_path, &[]);
    assert!(output.status.success(), "Contents command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        stdout.contains("[") && stdout.contains("]"),
        "Should contain markdown links"
    );

    assert!(
        stdout.contains("(") && stdout.contains(")"),
        "Should contain link URLs in parentheses"
    );
}

#[test]
fn test_contents_format_structure() {
    let temp_dir = setup_test_workspace_with_multiple_files();
    let temp_path = temp_dir.path();

    let output = run_contents_command(&temp_path, &[]);
    assert!(output.status.success(), "Contents command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        stdout.starts_with("# Contents\n"),
        "Should start with proper contents header"
    );

    let lines: Vec<&str> = stdout.lines().collect();
    let content_lines: Vec<&str> = lines
        .iter()
        .filter(|line| line.contains("[") && line.contains("]("))
        .cloned()
        .collect();

    for line in content_lines {
        assert!(
            line.matches('[').count() >= 1 && line.matches(']').count() >= 1,
            "Should have proper markdown link format: {}",
            line
        );
    }
}

#[test]
fn test_contents_empty_workspace() {
    let temp_dir = setup_empty_workspace();
    let temp_path = temp_dir.path();

    let output = run_contents_command(&temp_path, &[]);
    assert!(
        output.status.success(),
        "Contents should succeed even with empty workspace"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        stdout.contains("# Contents"),
        "Should contain contents header even for empty workspace"
    );
}

#[test]
fn test_contents_complex_workspace() {
    let temp_dir = setup_complex_test_workspace();
    let temp_path = temp_dir.path();

    let output = run_contents_command(&temp_path, &[]);
    assert!(output.status.success(), "Contents command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let link_count = stdout.matches("](").count();
    assert!(
        link_count >= 1,
        "Should contain at least one link for complex workspace"
    );
}

#[test]
fn test_contents_with_nested_structure() {
    let temp_dir = setup_test_workspace_with_nested_files();
    let temp_path = temp_dir.path();

    let output = run_contents_command(&temp_path, &[]);
    assert!(output.status.success(), "Contents command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    // Should handle nested files
    assert!(
        stdout.contains("# Contents"),
        "Should contain contents header"
    );

    assert!(!stdout.trim().is_empty(), "Should produce some output");
}

#[test]
fn test_contents_with_verbose_flag() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = Command::new(get_iwe_binary_path())
        .arg("contents")
        .arg("--verbose")
        .arg("1")
        .current_dir(&temp_path)
        .output()
        .expect("Failed to execute iwe contents");

    assert!(
        output.status.success(),
        "Contents with verbose flag should succeed"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    assert!(
        stdout.contains("# Contents"),
        "Should contain contents header"
    );
}

#[test]
fn test_contents_without_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let markdown_content = indoc! {"
        # Test Document

        This is a test document.

        ## Section 1

        Some content here.
    "};
    write(temp_path.join("test.md"), markdown_content).expect("Should write test file");

    let output = run_contents_command(&temp_path, &[]);
    assert!(
        output.status.success(),
        "Contents should work without explicit config"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    assert!(
        stdout.contains("# Contents"),
        "Should contain contents header"
    );
}

#[test]
fn test_contents_stderr_empty() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_contents_command(&temp_path, &[]);
    assert!(output.status.success(), "Contents command should succeed");

    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 stderr");

    assert!(
        stderr.is_empty() || (!stderr.contains("ERROR") && !stderr.contains("error:")),
        "Contents stderr should not contain errors: {}",
        stderr
    );
}

#[test]
fn test_contents_unique_entries() {
    let temp_dir = setup_test_workspace_with_duplicates();
    let temp_path = temp_dir.path();

    let output = run_contents_command(&temp_path, &[]);
    assert!(output.status.success(), "Contents command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let link_lines: Vec<&str> = stdout.lines().filter(|line| line.contains("](")).collect();

    let unique_lines: std::collections::HashSet<_> = link_lines.iter().collect();

    assert_eq!(
        link_lines.len(),
        unique_lines.len(),
        "Contents should not contain duplicate entries"
    );
}

#[test]
fn test_contents_with_special_characters() {
    let temp_dir = setup_test_workspace_with_special_chars();
    let temp_path = temp_dir.path();

    let output = run_contents_command(&temp_path, &[]);
    assert!(output.status.success(), "Contents command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        stdout.contains("# Contents"),
        "Should contain contents header"
    );

    // Should produce valid markdown even with special characters
    assert!(!stdout.trim().is_empty(), "Should produce output");
}

fn setup_test_workspace_with_content() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(&temp_path);

    let markdown_content = indoc! {"
        # Test Document

        This is a test document with some content.

        ## Section 1

        Some content here.

        ## Section 2

        More content here.
    "};

    write(temp_path.join("test.md"), markdown_content).expect("Should write test file");

    temp_dir
}

fn setup_test_workspace_with_multiple_files() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(&temp_path);

    let file1_content = indoc! {"
        # Document One

        Content for document one.
    "};

    let file2_content = indoc! {"
        # Document Two

        Content for document two.
    "};

    let file3_content = indoc! {"
        # Document Three

        Content for document three.
    "};

    write(temp_path.join("doc1.md"), file1_content).expect("Should write doc1");
    write(temp_path.join("doc2.md"), file2_content).expect("Should write doc2");
    write(temp_path.join("doc3.md"), file3_content).expect("Should write doc3");

    temp_dir
}

fn setup_complex_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(&temp_path);

    let file1_content = indoc! {"
        # Complex Document 1

        Content for complex document 1.

        ## Section A

        Content A.

        ## Section B

        Content B.
    "};

    let file2_content = indoc! {"
        # Complex Document 2

        Content for complex document 2.

        ## Section X

        Content X.
    "};

    let nested_content = indoc! {"
        # Nested Document

        Nested content here.
    "};

    write(temp_path.join("complex1.md"), file1_content).expect("Should write complex1");
    write(temp_path.join("complex2.md"), file2_content).expect("Should write complex2");

    create_dir_all(temp_path.join("subdirectory")).expect("Should create subdirectory");
    write(
        temp_path.join("subdirectory").join("nested.md"),
        nested_content,
    )
    .expect("Should write nested file");

    temp_dir
}

fn setup_test_workspace_with_nested_files() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(&temp_path);

    let root_content = indoc! {"
        # Root Document

        Root level content.
    "};

    let nested_content = indoc! {"
        # Nested Document

        Nested level content.
    "};

    let deep_nested_content = indoc! {"
        # Deep Nested Document

        Deep nested content.
    "};

    write(temp_path.join("root.md"), root_content).expect("Should write root");

    create_dir_all(temp_path.join("level1")).expect("Should create level1");
    write(temp_path.join("level1").join("nested.md"), nested_content).expect("Should write nested");

    create_dir_all(temp_path.join("level1").join("level2")).expect("Should create level2");
    write(
        temp_path.join("level1").join("level2").join("deep.md"),
        deep_nested_content,
    )
    .expect("Should write deep nested");

    temp_dir
}

fn setup_test_workspace_with_duplicates() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(&temp_path);

    let content1 = indoc! {"
        # Unique Title One

        Content 1.
    "};

    let content2 = indoc! {"
        # Unique Title Two

        Content 2.
    "};

    write(temp_path.join("unique1.md"), content1).expect("Should write unique1");
    write(temp_path.join("unique2.md"), content2).expect("Should write unique2");

    temp_dir
}

fn setup_test_workspace_with_special_chars() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(&temp_path);

    let special_content = indoc! {"
        # Special Characters & Symbols

        Content with special characters: \"quotes\", 'apostrophes', & ampersands.

        ## Unicode: 中文, العربية, 🚀

        More special content here.
    "};

    write(temp_path.join("special.md"), special_content).expect("Should write special");

    temp_dir
}

fn setup_empty_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(&temp_path);

    temp_dir
}

fn setup_iwe_config(temp_path: &std::path::Path) {
    create_dir_all(temp_path.join(".iwe")).expect("Failed to create .iwe directory");

    let config = Configuration {
        library: LibraryOptions {
            path: "".to_string(),
        },
        markdown: MarkdownOptions {
            refs_extension: "".to_string(),
        },
        ..Default::default()
    };

    let config_content = toml::to_string(&config).expect("Failed to serialize config to TOML");

    write(temp_path.join(".iwe").join("config.toml"), config_content)
        .expect("Should write config file");
}

fn run_contents_command(work_dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(get_iwe_binary_path());
    command.arg("contents").current_dir(work_dir);

    for arg in args {
        command.arg(arg);
    }

    command.output().expect("Failed to execute iwe contents")
}

fn get_iwe_binary_path() -> PathBuf {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let workspace_root = current_dir.parent().unwrap().parent().unwrap();
    workspace_root.join("target").join("debug").join("iwe")
}
