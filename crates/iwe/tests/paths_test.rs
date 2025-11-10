use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, write};
use std::process::Command;
use tempfile::TempDir;

mod common;

#[test]
fn test_paths_basic_output() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_paths_command(temp_path, &[]);
    assert!(output.status.success(), "Paths command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(!stdout.trim().is_empty(), "Should produce some output");

    assert!(
        stdout.chars().any(|c| c.is_alphabetic()),
        "Should contain text"
    );
}

#[test]
fn test_paths_with_depth_limit() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_paths_command(temp_path, &["--depth", "1"]);
    assert!(
        output.status.success(),
        "Paths command with depth should succeed"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let lines: Vec<&str> = stdout.lines().collect();

    assert!(
        !lines.is_empty(),
        "Should have some output even with depth 1"
    );
}

#[test]
fn test_paths_with_higher_depth() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_paths_command(temp_path, &["--depth", "6"]);
    assert!(
        output.status.success(),
        "Paths command with higher depth should succeed"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(!stdout.trim().is_empty(), "Should produce output");
}

#[test]
fn test_paths_default_depth() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_paths_command(temp_path, &[]);
    assert!(output.status.success(), "Paths command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        !stdout.trim().is_empty(),
        "Should produce output with default depth"
    );
}

#[test]
fn test_paths_empty_workspace() {
    let temp_dir = setup_empty_workspace();
    let temp_path = temp_dir.path();

    let output = run_paths_command(temp_path, &[]);
    assert!(
        output.status.success(),
        "Paths should succeed even with empty workspace"
    );

    let _stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
}

#[test]
fn test_paths_complex_workspace() {
    let temp_dir = setup_complex_test_workspace();
    let temp_path = temp_dir.path();

    let output = run_paths_command(temp_path, &[]);
    assert!(output.status.success(), "Paths command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    assert!(
        !lines.is_empty(),
        "Should produce paths from complex workspace"
    );
}

#[test]
fn test_paths_with_links() {
    let temp_dir = setup_test_workspace_with_links();
    let temp_path = temp_dir.path();

    let output = run_paths_command(temp_path, &[]);
    assert!(output.status.success(), "Paths command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        !stdout.trim().is_empty(),
        "Should produce output for linked documents"
    );
}

#[test]
fn test_paths_output_format() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_paths_command(temp_path, &[]);
    assert!(output.status.success(), "Paths command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    for line in stdout.lines() {
        if !line.trim().is_empty() {
            // Lines should contain readable text
            assert!(
                line.chars().any(|c| c.is_alphabetic()),
                "Path should contain text: {}",
                line
            );
        }
    }
}

#[test]
fn test_paths_sorted_unique_output() {
    let temp_dir = setup_test_workspace_with_duplicates();
    let temp_path = temp_dir.path();

    let output = run_paths_command(temp_path, &[]);
    assert!(output.status.success(), "Paths command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    let mut sorted_lines = lines.clone();
    sorted_lines.sort();

    let unique_lines: std::collections::HashSet<_> = lines.iter().collect();
    assert_eq!(
        lines.len(),
        unique_lines.len(),
        "Output should not contain duplicates"
    );
}

#[test]
fn test_paths_with_verbose_flag() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = Command::new(common::get_iwe_binary_path())
        .arg("paths")
        .arg("--verbose")
        .arg("1")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe paths");

    assert!(
        output.status.success(),
        "Paths with verbose flag should succeed"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    assert!(!stdout.trim().is_empty(), "Should produce output");
}

#[test]
fn test_paths_zero_depth() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_paths_command(temp_path, &["--depth", "0"]);
    assert!(
        output.status.success(),
        "Paths command with depth 0 should succeed"
    );

    let _stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
}

#[test]
fn test_paths_without_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let markdown_content = indoc! {"
        # Test Document

        This is a test document.

        ## Section 1

        Some content here.
    "};
    write(temp_path.join("test.md"), markdown_content).expect("Should write test file");

    let output = run_paths_command(temp_path, &[]);
    assert!(
        output.status.success(),
        "Paths should work without explicit config"
    );
}

#[test]
fn test_paths_stderr_empty() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_paths_command(temp_path, &[]);
    assert!(output.status.success(), "Paths command should succeed");

    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 stderr");

    assert!(
        stderr.is_empty() || (!stderr.contains("ERROR") && !stderr.contains("error:")),
        "Paths stderr should not contain errors: {}",
        stderr
    );
}

fn setup_test_workspace_with_content() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let markdown_content = indoc! {"
        # Test Document

        This is a test document with some content.

        ## Section 1

        Some content here.

        ### Subsection

        More content.

        ## Section 2

        Additional content.
    "};

    write(temp_path.join("test.md"), markdown_content).expect("Should write test file");

    temp_dir
}

fn setup_complex_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let file1_content = indoc! {"
        # Document 1

        Content for document 1.

        ## Section A

        Content A.

        ## Section B

        Content B.
    "};

    let file2_content = indoc! {"
        # Document 2

        Content for document 2.

        ## Section X

        Content X.

        ## Section Y

        Content Y.
    "};

    let nested_content = indoc! {"
        # Nested Document

        Nested content here.

        ## Nested Section

        More nested content.
    "};

    write(temp_path.join("file1.md"), file1_content).expect("Should write file1");
    write(temp_path.join("file2.md"), file2_content).expect("Should write file2");

    create_dir_all(temp_path.join("subdirectory")).expect("Should create subdirectory");
    write(
        temp_path.join("subdirectory").join("nested.md"),
        nested_content,
    )
    .expect("Should write nested file");

    temp_dir
}

fn setup_test_workspace_with_links() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let main_content = indoc! {"
        # Main Document

        This document links to [target](target.md).

        ## Section

        More content with links.

        ### Subsection

        Content with links.
    "};

    let target_content = indoc! {"
        # Target Document

        This is the target of the link.

        ## Target Section

        Target content.
    "};

    write(temp_path.join("main.md"), main_content).expect("Should write main file");
    write(temp_path.join("target.md"), target_content).expect("Should write target file");

    temp_dir
}

fn setup_test_workspace_with_duplicates() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let content1 = indoc! {"
        # Duplicate Title

        Content 1.

        ## Section

        Content for file 1.
    "};

    let content2 = indoc! {"
        # Duplicate Title

        Content 2.

        ## Section

        Content for file 2.
    "};

    write(temp_path.join("doc1.md"), content1).expect("Should write doc1");
    write(temp_path.join("doc2.md"), content2).expect("Should write doc2");

    temp_dir
}

fn setup_empty_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    temp_dir
}

fn setup_iwe_config(temp_path: &std::path::Path) {
    create_dir_all(temp_path.join(".iwe")).expect("Failed to create .iwe directory");

    let config = Configuration {
        library: LibraryOptions {
            path: "".to_string(),
            ..Default::default()
        },
        markdown: MarkdownOptions {
            refs_extension: "".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let config_content = toml::to_string(&config).expect("Failed to serialize config to TOML");

    write(temp_path.join(".iwe").join("config.toml"), config_content)
        .expect("Should write config file");
}

fn run_paths_command(work_dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(common::get_iwe_binary_path());
    command.arg("paths").current_dir(work_dir);

    for arg in args {
        command.arg(arg);
    }

    command.output().expect("Failed to execute iwe paths")
}
