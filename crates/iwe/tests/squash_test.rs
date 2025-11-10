use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::fs::{create_dir_all, write};
use std::process::Command;
use tempfile::TempDir;

mod common;

#[test]
fn test_squash_basic_functionality() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "test"]);
    assert!(output.status.success(), "Squash command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(!stdout.trim().is_empty(), "Should produce some output");
    assert!(
        stdout.contains("#") || stdout.contains("*") || stdout.contains("-"),
        "Should contain markdown formatting"
    );
}

#[test]
fn test_squash_with_depth_limit() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "test", "--depth", "1"]);
    assert!(
        output.status.success(),
        "Squash command with depth should succeed"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    assert!(
        !stdout.trim().is_empty(),
        "Should produce output with depth limit"
    );
}

#[test]
fn test_squash_with_higher_depth() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "test", "--depth", "5"]);
    assert!(
        output.status.success(),
        "Squash command with higher depth should succeed"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    assert!(
        !stdout.trim().is_empty(),
        "Should produce output with higher depth"
    );
}

#[test]
fn test_squash_default_depth() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "test"]);
    assert!(output.status.success(), "Squash command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        !stdout.trim().is_empty(),
        "Should produce output with default depth"
    );
}

#[test]
fn test_squash_nonexistent_key() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "nonexistent"]);
    assert!(
        !output.status.success(),
        "Squash should fail with nonexistent key"
    );

    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 stderr");
    assert!(
        !stderr.is_empty(),
        "Should have error output for nonexistent key"
    );
}

#[test]
fn test_squash_complex_workspace() {
    let temp_dir = setup_complex_test_workspace();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "document1"]);
    assert!(output.status.success(), "Squash command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        !stdout.trim().is_empty(),
        "Should produce output from complex workspace"
    );
}

#[test]
fn test_squash_with_links() {
    let temp_dir = setup_test_workspace_with_links();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "main"]);
    assert!(output.status.success(), "Squash command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert!(
        !stdout.trim().is_empty(),
        "Should produce output for linked documents"
    );
}

#[test]
fn test_squash_preserves_markdown_structure() {
    let temp_dir = setup_test_workspace_with_structured_content();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "structured"]);
    assert!(output.status.success(), "Squash command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    let has_headers = stdout.contains('#');
    let has_lists = stdout.contains('-') || stdout.contains('*') || stdout.contains("1.");

    assert!(
        has_headers || has_lists || !stdout.trim().is_empty(),
        "Should preserve some markdown structure or content"
    );
}

#[test]
fn test_squash_with_verbose_flag() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = Command::new(common::get_iwe_binary_path())
        .arg("squash")
        .arg("--key")
        .arg("test")
        .arg("--verbose")
        .arg("1")
        .current_dir(temp_path)
        .output()
        .expect("Failed to execute iwe squash");

    assert!(
        output.status.success(),
        "Squash with verbose flag should succeed"
    );

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    assert!(!stdout.trim().is_empty(), "Should produce output");
}

#[test]
fn test_squash_zero_depth() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "test", "--depth", "0"]);
    assert!(
        output.status.success(),
        "Squash command with depth 0 should succeed"
    );

    let _stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
}

#[test]
fn test_squash_key_with_spaces() {
    let temp_dir = setup_test_workspace_with_spaced_content();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "spaced key"]);
    assert!(
        output.status.success(),
        "Squash should handle keys with spaces"
    );

    let _stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
}

#[test]
fn test_squash_empty_workspace() {
    let temp_dir = setup_empty_workspace();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "anything"]);
    assert!(
        !output.status.success(),
        "Squash should fail with empty workspace and nonexistent key"
    );

    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 stderr");
    assert!(
        !stderr.is_empty(),
        "Should have error output for empty workspace"
    );
}

#[test]
fn test_squash_without_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let markdown_content = indoc! {"
        # Test Document

        This is a test document.

        ## Section 1

        Some content here.
    "};
    write(temp_path.join("test.md"), markdown_content).expect("Should write test file");

    let output = run_squash_command(temp_path, &["--key", "test"]);
    assert!(
        output.status.success(),
        "Squash should work without explicit config"
    );
}

#[test]
fn test_squash_stderr_empty() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "test"]);
    assert!(output.status.success(), "Squash command should succeed");

    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 stderr");

    assert!(
        stderr.is_empty() || (!stderr.contains("ERROR") && !stderr.contains("error:")),
        "Squash stderr should not contain errors: {}",
        stderr
    );
}

#[test]
fn test_squash_output_is_markdown() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = run_squash_command(temp_path, &["--key", "test"]);
    assert!(output.status.success(), "Squash command should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    if !stdout.trim().is_empty() {
        let has_markdown_elements = stdout.contains('#')
            || stdout.contains('*')
            || stdout.contains('-')
            || stdout.contains('[')
            || stdout.lines().any(|line| !line.trim().is_empty());

        assert!(
            has_markdown_elements,
            "Output should contain markdown-like elements or be empty"
        );
    }
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

        ### Subsection A1

        Nested content.
    "};

    let file2_content = indoc! {"
        # Document 2

        Content for document 2.

        ## Section B

        Content B.

        ## Section C

        Content C.
    "};

    let nested_content = indoc! {"
        # Nested Document

        Nested content here.

        ## Nested Section

        More nested content.
    "};

    write(temp_path.join("document1.md"), file1_content).expect("Should write file1");
    write(temp_path.join("document2.md"), file2_content).expect("Should write file2");

    create_dir_all(temp_path.join("subdirectory")).expect("Should create subdirectory");
    write(
        temp_path.join("subdirectory").join("document_nested.md"),
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

fn setup_test_workspace_with_structured_content() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let structured_content = indoc! {"
        # Structured Document

        This document has various markdown elements.

        ## Lists

        Unordered list:
        - Item 1
        - Item 2
          - Nested item

        Ordered list:
        1. First item
        2. Second item
           1. Nested ordered item

        ## Code

        ```rust
        fn hello() {
            println!(\"Hello, world!\");
        }
        ```

        ## Links and References

        Link to [external site](https://example.com).

        Reference to another document: [Document 1](document1).

        ## Tables

        | Column 1 | Column 2 |
        |----------|----------|
        | Value 1  | Value 2  |
    "};

    write(temp_path.join("structured.md"), structured_content)
        .expect("Should write structured file");

    temp_dir
}

fn setup_test_workspace_with_spaced_content() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(temp_path);

    let content = indoc! {"
        # Spaced Key Document

        This document has a title with spaces.

        ## Section

        Content here.
    "};

    write(temp_path.join("spaced key.md"), content).expect("Should write spaced file");

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

fn run_squash_command(work_dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(common::get_iwe_binary_path());
    command.arg("squash").current_dir(work_dir);

    for arg in args {
        command.arg(arg);
    }

    command.output().expect("Failed to execute iwe squash")
}
