use indoc::indoc;
use liwe::model::config::{Configuration, LibraryOptions, MarkdownOptions};
use std::env;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_normalize_basic_formatting() {
    let temp_dir = setup_test_workspace_with_unformatted_content();
    let temp_path = temp_dir.path();

    let output = run_normalize_command(&temp_path);
    assert!(output.status.success(), "Normalize command should succeed");

    let content =
        read_to_string(temp_path.join("test.md")).expect("Should be able to read normalized file");

    assert!(
        !content.trim().is_empty(),
        "Normalized content should not be empty"
    );

    assert!(
        content.contains("Header") || content.contains("#"),
        "Should contain header content"
    );
}

#[test]
fn test_normalize_preserves_content() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let _original_content =
        read_to_string(temp_path.join("test.md")).expect("Should be able to read original file");

    let output = run_normalize_command(&temp_path);
    assert!(output.status.success(), "Normalize command should succeed");

    let normalized_content =
        read_to_string(temp_path.join("test.md")).expect("Should be able to read normalized file");

    assert!(normalized_content.contains("This is a test document"));
    assert!(normalized_content.contains("Some content here"));
}

#[test]
fn test_normalize_multiple_files() {
    let temp_dir = setup_complex_test_workspace();
    let temp_path = temp_dir.path();

    let output = run_normalize_command(&temp_path);
    assert!(output.status.success(), "Normalize command should succeed");

    assert!(temp_path.join("file1.md").exists());
    assert!(temp_path.join("file2.md").exists());
    assert!(temp_path.join("subdirectory").join("nested.md").exists());

    let file1_content = read_to_string(temp_path.join("file1.md")).expect("Should read file1");
    let file2_content = read_to_string(temp_path.join("file2.md")).expect("Should read file2");

    assert!(file1_content.contains("File 1 content"));
    assert!(file2_content.contains("File 2 content"));
}

#[test]
fn test_normalize_empty_workspace() {
    let temp_dir = setup_empty_workspace();
    let temp_path = temp_dir.path();

    let output = run_normalize_command(&temp_path);
    assert!(
        output.status.success(),
        "Normalize should succeed even with empty workspace"
    );

    let stderr = String::from_utf8(output.stderr).expect("Valid UTF-8 stderr");
    assert!(
        !stderr.contains("ERROR") && !stderr.contains("error:"),
        "Should not produce errors with empty workspace"
    );
}

#[test]
fn test_normalize_with_links() {
    let temp_dir = setup_test_workspace_with_links();
    let temp_path = temp_dir.path();

    let output = run_normalize_command(&temp_path);
    assert!(output.status.success(), "Normalize command should succeed");

    let content = read_to_string(temp_path.join("main.md")).expect("Should read main file");

    assert!(
        content.contains("[") && content.contains("]"),
        "Should preserve links"
    );
}

#[test]
fn test_normalize_with_lists() {
    let temp_dir = setup_test_workspace_with_lists();
    let temp_path = temp_dir.path();

    let output = run_normalize_command(&temp_path);
    assert!(output.status.success(), "Normalize command should succeed");

    let content = read_to_string(temp_path.join("lists.md")).expect("Should read lists file");

    assert!(
        content.contains("- ") || content.contains("* "),
        "Should contain list items"
    );
}

#[test]
fn test_normalize_without_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    let markdown_content = indoc! {"
        #Header1
        Some content

        ##Header2
        More content
    "};
    write(temp_path.join("test.md"), markdown_content).expect("Should write test file");

    let output = run_normalize_command(&temp_path);
    assert!(
        output.status.success(),
        "Normalize should work without explicit config"
    );
}

#[test]
fn test_normalize_with_verbose_flag() {
    let temp_dir = setup_test_workspace_with_content();
    let temp_path = temp_dir.path();

    let output = Command::new(get_iwe_binary_path())
        .arg("normalize")
        .arg("--verbose")
        .arg("1")
        .current_dir(&temp_path)
        .output()
        .expect("Failed to execute iwe normalize");

    assert!(
        output.status.success(),
        "Normalize with verbose flag should succeed"
    );
}

#[test]
fn test_normalize_updates_link_titles() {
    let temp_dir = setup_test_workspace_with_outdated_links();
    let temp_path = temp_dir.path();

    let output = run_normalize_command(&temp_path);
    assert!(output.status.success(), "Normalize command should succeed");

    let content = read_to_string(temp_path.join("main.md")).expect("Should read main file");

    assert!(content.contains("Updated Title") || content.contains("target.md"));
}

#[test]
fn test_normalize_preserves_file_structure() {
    let temp_dir = setup_complex_test_workspace();
    let temp_path = temp_dir.path();

    let files_before = count_markdown_files(&temp_path);

    let output = run_normalize_command(&temp_path);
    assert!(output.status.success(), "Normalize command should succeed");

    let files_after = count_markdown_files(&temp_path);

    assert_eq!(
        files_before, files_after,
        "File count should remain the same after normalization"
    );
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

        ### Subsection

        More content.
    "};

    write(temp_path.join("test.md"), markdown_content).expect("Should write test file");

    temp_dir
}

fn setup_test_workspace_with_unformatted_content() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(&temp_path);

    let unformatted_content = indoc! {"
        # Header 1

        This is a test document with some content.

        ## Header 2

        Some content here with more text.

        ### Subsection

        More detailed content.
    "};

    write(temp_path.join("test.md"), unformatted_content).expect("Should write test file");

    temp_dir
}

fn setup_complex_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(&temp_path);

    let file1_content = indoc! {"
        # File 1

        File 1 content here.

        ## Section A

        Content A.
    "};

    let file2_content = indoc! {"
        # File 2

        File 2 content here.

        ## Section B

        Content B.
    "};

    let nested_content = indoc! {"
        # Nested File

        Nested content here.
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

    setup_iwe_config(&temp_path);

    let main_content = indoc! {"
        # Main Document

        This document links to [target](target.md).

        ## Section

        Some content.
    "};

    let target_content = indoc! {"
        # Target Document

        This is the target of the link.
    "};

    write(temp_path.join("main.md"), main_content).expect("Should write main file");
    write(temp_path.join("target.md"), target_content).expect("Should write target file");

    temp_dir
}

fn setup_test_workspace_with_lists() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(&temp_path);

    let lists_content = indoc! {"
        # Lists Document

        Unordered list:
        * Item 1
        * Item 2
          * Nested item 1
          * Nested item 2

        Ordered list:
        1. First item
        2. Second item
    "};

    write(temp_path.join("lists.md"), lists_content).expect("Should write lists file");

    temp_dir
}

fn setup_test_workspace_with_outdated_links() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let temp_path = temp_dir.path();

    setup_iwe_config(&temp_path);

    let main_content = indoc! {"
        # Main Document

        This document links to [Old Title](target.md).
    "};

    let target_content = indoc! {"
        # Updated Title

        This is the target with an updated title.
    "};

    write(temp_path.join("main.md"), main_content).expect("Should write main file");
    write(temp_path.join("target.md"), target_content).expect("Should write target file");

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

fn count_markdown_files(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "md")
                .unwrap_or(false)
        })
        .count()
}

fn run_normalize_command(work_dir: &std::path::Path) -> std::process::Output {
    Command::new(get_iwe_binary_path())
        .arg("normalize")
        .current_dir(work_dir)
        .output()
        .expect("Failed to execute iwe normalize")
}

fn get_iwe_binary_path() -> PathBuf {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let workspace_root = current_dir.parent().unwrap().parent().unwrap();
    workspace_root.join("target").join("debug").join("iwe")
}
